//! Application wiring: the configuration, the command line, the routes and the middleware stack.

use axa::sqlx::{migrate, status};
use axa::{
    AppContext, Environment, Error, Result,
    boot::{BootResult, TestApp},
    cli::{Cli, Command, DESTROY_DATA_VAR, DbCommand},
    config::{Config, Defaults, MB},
    routes::AppRoutes,
    server,
};
use axum::Router as AxumRouter;

use crate::{MIGRATOR, auth, authz, controllers, mllp, pool::Pool};

/// The application.
pub struct App;

/// What differs from `axa`'s defaults for this service.
fn defaults() -> Defaults {
    Defaults {
        database: "electronic_patient_record",
        port: 5150,
        production_port: None,
        body_limit: 2 * MB,
        max_connections: 1,
        ..Defaults::default()
    }
}

/// The local Podman stack, which runs a release build with login off (see `compose.yaml`).
const CONTAINER_DEV: &str = "container-dev";

/// Whether `environment` is held to the production login rules. An unknown name is, so that a mistyped
/// environment cannot start a service with login off.
fn production_login(environment: &Environment) -> bool {
    environment.is_production_like() && !is_container_dev(environment)
}

fn is_container_dev(environment: &Environment) -> bool {
    matches!(environment, Environment::Other(name) if name == CONTAINER_DEV)
}

/// The default for login where a person has chosen a development environment: off. Anywhere else there is
/// no default, so login is on. `EPR_REQUIRE_AUTH` overrides this, and production ignores it.
fn default_require_auth(environment: &Environment) -> Option<bool> {
    (matches!(environment, Environment::Development | Environment::Test)
        || is_container_dev(environment))
    .then_some(false)
}

impl TestApp for App {
    /// Boot under the `test` environment, migrating the shared test database forward and keeping its rows.
    async fn boot_test() -> Result<BootResult> {
        let environment = Environment::Test;
        let config = Config::from_env(&environment, &defaults())?;
        let ctx = Self::context(environment, config).await?;
        Self::migrate(&ctx).await?;
        let router = Self::router(&ctx).await?;
        Ok(BootResult {
            app_context: ctx,
            router: Some(router),
        })
    }
}

impl App {
    /// Connect to the database and build the context.
    ///
    /// # Errors
    ///
    /// When the database cannot be reached.
    pub async fn context(environment: Environment, config: Config) -> Result<AppContext> {
        let pool = axa::db::connect(&config.database)
            .await
            .map_err(|e| Error::string(&format!("cannot reach the database: {e}")))?;
        Ok(AppContext::new(environment, pool, config))
    }

    /// The routes in the standard middleware, wrapped in the service's own.
    ///
    /// # Errors
    ///
    /// When a setting is wrong, or a port cannot be opened.
    pub async fn router(ctx: &AppContext) -> Result<AxumRouter> {
        let router = Self::routes(ctx).into_app(ctx.clone())?;
        Self::after_routes(router, ctx).await
    }

    /// Apply pending migrations.
    async fn migrate(ctx: &AppContext) -> Result<()> {
        migrate(&ctx.pool(), &MIGRATOR)
            .await
            .map_err(|e| Error::string(&format!("migration failed: {e}")))
    }

    /// Run a database command. There is no `down`: migrations have no down files, so the clinical record
    /// cannot be rolled back through them.
    async fn db(ctx: &AppContext, command: DbCommand) -> Result<()> {
        match command {
            DbCommand::Migrate => Self::migrate(ctx).await?,
            DbCommand::Down { .. } => {
                return Err(Error::string(
                    "migrations have no down files: the clinical record is not rolled back",
                ));
            }
            DbCommand::Reset => {
                let flag = std::env::var(DESTROY_DATA_VAR).ok();
                if !ctx.environment.may_destroy_data(flag.as_deref()) {
                    return Err(Error::string(&format!(
                        "db reset drops every table, so it is refused outside development and \
                         test, and needs {DESTROY_DATA_VAR}=true there"
                    )));
                }
                sqlx::raw_sql(
                    "DROP SCHEMA public CASCADE; DROP SCHEMA IF EXISTS audit_archive CASCADE; \
                     CREATE SCHEMA public;",
                )
                .execute(&ctx.pool())
                .await
                .map_err(|e| Error::string(&format!("reset failed: {e}")))?;
                Self::migrate(ctx).await?;
            }
            DbCommand::Status => {
                let all = status(&ctx.pool(), &MIGRATOR)
                    .await
                    .map_err(|e| Error::string(&format!("status failed: {e}")))?;
                for migration in all {
                    let state = if migration.applied {
                        "applied"
                    } else {
                        "pending"
                    };
                    println!("{state}\t{}\t{}", migration.version, migration.description);
                }
            }
            // The clinical record is append-only, so the service has no truncate.
            DbCommand::Truncate => {
                return Err(Error::string("this service has no truncate"));
            }
        }
        Ok(())
    }

    /// Run the command line.
    ///
    /// # Errors
    ///
    /// When the configuration is refused, or the command fails.
    pub async fn run(cli: Cli) -> Result<()> {
        let environment = cli
            .environment
            .as_deref()
            .map_or_else(Environment::from_env, Environment::parse);
        // Before anything connects: this checks settings, and must work with the database down.
        if matches!(cli.command, Command::Check) {
            return axa::cli::check(&environment, &defaults());
        }
        let mut config = Config::from_env(&environment, &defaults())?;
        if let Command::Start { binding, port, .. } = &cli.command {
            if let Some(binding) = binding {
                config.server.binding.clone_from(binding);
            }
            if let Some(port) = port {
                config.server.port = *port;
            }
        }
        let ctx = Self::context(environment, config).await?;
        axa::logger::init(env!("CARGO_CRATE_NAME"), &ctx.config.logger);
        if ctx.config.database.auto_migrate && !matches!(cli.command, Command::Db { .. }) {
            Self::migrate(&ctx).await?;
        }
        match cli.command {
            Command::Start { .. } => {
                let router = Self::router(&ctx).await?;
                server::serve(router, &ctx.config.server).await
            }
            Command::Db { command } => Self::db(&ctx, command).await,
            Command::Task { .. } => Err(Error::string("this service registers no tasks")),
            // Answered above, before anything connects.
            Command::Check => Ok(()),
        }
    }

    /// Every route the service serves, from [`controllers::all_routes`], the one list the route census also reads.
    fn routes(_ctx: &AppContext) -> AppRoutes {
        controllers::all_routes()
    }

    /// Wire everything that is not a route, then wrap the routes in the middleware stack.
    ///
    /// In order: the authentication settings (`default_require_auth`, always on in production); the
    /// site time zone; the MLLP listener, only when `EPR_MLLP_BIND` is set; the outbox loop, only when
    /// `EPR_OUTBOX_INTERVAL_SECS` is set; the audit-partition loop (archiving only when
    /// `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` is set); the country packs; the per-subject request limits; and the
    /// idempotency-key purge loop. A setting that is present but wrong stops the service starting: a feature that
    /// was asked for and is silently absent is worse than no service.
    ///
    /// The layers, outermost (first to run) to innermost: request timing, request id, NUL-byte refusal,
    /// authentication, rate limit, authorization, idempotency key.
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        auth::init(
            production_login(&ctx.environment),
            default_require_auth(&ctx.environment),
        )
        .await
        .map_err(Error::Message)?;
        // The site's time zone, for the age on a banner: a name that is not an IANA zone stops the service.
        crate::site::init().map_err(Error::Message)?;
        // The referral timeouts (PC-3): a setting that is wrong stops the service rather than switching one off.
        crate::referral_timeouts::init().map_err(Error::Message)?;
        // The MLLP listener for HL7 v2 ADT feeds is off unless `EPR_MLLP_BIND` is set. A bad
        // setting, or a port that cannot be opened, stops the service starting: a feed that was
        // asked for and is silently absent is worse than no service.
        if let Some(config) = mllp::Config::from_env().map_err(Error::Message)? {
            let listener = tokio::net::TcpListener::bind(config.bind)
                .await
                .map_err(|e| {
                    Error::Message(format!("cannot listen for MLLP on {}: {e}", config.bind))
                })?;
            tracing::warn!(
                bind = %config.bind,
                agent = %config.agent,
                "MLLP listener is ON: it has no authentication or encryption"
            );
            tokio::spawn(mllp::serve(listener, ctx.pool(), config));
        }
        // The outbox loop is off unless `EPR_OUTBOX_INTERVAL_SECS` is set (OS-3). Its only sink is the
        // log sink, which sends nothing; a real sink arrives with IO-6.
        if let Some(every) = crate::outbox::interval_from_env().map_err(Error::Message)? {
            let mut sinks = crate::outbox::Sinks::new();
            sinks.insert("log".to_string(), Box::new(crate::outbox::LogSink));
            tracing::warn!(
                seconds = every.as_secs(),
                "outbox loop is ON (log sink only)"
            );
            tokio::spawn(crate::outbox::run_loop(
                ctx.pool(),
                crate::audit::chain(),
                sinks,
                every,
            ));
        }
        // Audit partitions are made ahead and, when `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` is set, old ones are
        // moved to the archive schema (OS-13). A bad setting stops the service starting.
        let archive_after =
            crate::audit_retention::archive_months_from_env().map_err(Error::Message)?;
        tokio::spawn(crate::audit_retention::run_loop(ctx.pool(), archive_after));
        // Country packs the deployment enables (OS-10). A pack that does not load stops the service.
        crate::packs::init().map_err(Error::Message)?;
        // Request limits per token subject (OS-2). A bad setting stops the service starting.
        let limits = crate::rate_limit::Limits::from_env().map_err(Error::Message)?;
        let limiter = crate::rate_limit::Gate {
            limiter: std::sync::Arc::new(crate::rate_limit::Limiter::new(limits)),
            db: ctx.pool(),
        };
        // Expired idempotency keys hold answers that can name a patient: remove them (OF-1).
        tokio::spawn(crate::idempotency::run_loop(ctx.pool()));
        let keys = crate::idempotency::Gate { db: ctx.pool() };
        // The last layer added runs first: authenticate, count, authorize, then the idempotency key.
        let gate = authz::Authz { db: ctx.pool() };
        Ok(router
            .layer(axum::middleware::from_fn_with_state(
                keys,
                crate::idempotency::idempotency_mw,
            ))
            .layer(axum::middleware::from_fn_with_state(
                gate,
                authz::authorize_mw,
            ))
            .layer(axum::middleware::from_fn_with_state(
                limiter,
                crate::rate_limit::rate_limit_mw,
            ))
            .layer(axum::middleware::from_fn(auth::require_auth_mw))
            .layer(axum::middleware::from_fn(authz::no_nul_mw))
            // The request id is settled before authentication or the audit trail.
            .layer(axum::middleware::from_fn(authz::request_id_mw))
            // Outermost: time every request, refusals included, labelled by route template (OS-6).
            .layer(axum::middleware::from_fn(crate::metrics::duration_mw)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn other(name: &str) -> Environment {
        Environment::Other(name.to_string())
    }

    #[test]
    fn login_defaults_off_only_where_a_person_chose_a_development_environment() {
        assert_eq!(default_require_auth(&Environment::Development), Some(false));
        assert_eq!(default_require_auth(&Environment::Test), Some(false));
        assert_eq!(default_require_auth(&other(CONTAINER_DEV)), Some(false));
        assert_eq!(default_require_auth(&Environment::Production), None);
        assert_eq!(default_require_auth(&other("staging")), None);
        assert_eq!(default_require_auth(&other("productoin")), None);
    }

    #[test]
    fn db_reset_is_refused_unless_it_is_development_or_test_and_asked_for_by_name() {
        assert!(Environment::Development.may_destroy_data(Some("true")));
        assert!(Environment::Test.may_destroy_data(Some("true")));
        assert!(!Environment::Development.may_destroy_data(None));
        assert!(!Environment::Test.may_destroy_data(Some("yes")));
        assert!(!Environment::Production.may_destroy_data(Some("true")));
        assert!(!other(CONTAINER_DEV).may_destroy_data(Some("true")));
        assert!(!other("staging").may_destroy_data(Some("true")));
    }

    #[test]
    fn a_mistyped_environment_gets_the_production_login_rules() {
        assert!(production_login(&Environment::Production));
        assert!(production_login(&other("staging")));
        assert!(production_login(&other("productoin")));
        assert!(!production_login(&Environment::Development));
        assert!(!production_login(&Environment::Test));
        assert!(!production_login(&other(CONTAINER_DEV)));
    }
}
