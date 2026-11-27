//! Application wiring: the configuration, the command line, the routes and the middleware stack.

use axa::postgres_sqlx::{migrate, status};
use axa::{
    AppContext, Environment, Error, Result,
    boot::{BootResult, TestApp},
    cli::{Cli, Command, DESTROY_DATA_VAR, DbCommand},
    config::{Config, Defaults, MB},
    routes::AppRoutes,
    server,
};
use axum::Router as AxumRouter;

use std::sync::Arc;

use crate::runtime::{Runtime, Settings};
use crate::{MIGRATOR, auth, authz, controllers, mllp, pool::Pool};

/// The application.
pub struct App;

/// What differs from `axa`'s defaults for this service.
fn defaults() -> Defaults {
    Defaults {
        database: "electronic_patient_record",
        port: 1241,
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

    /// Boot under the `test` environment reading every setting, `axa`'s and the service's own, through `lookup`
    /// and never the process environment, so tests in one program can differ (QA-16).
    async fn boot_test_with(lookup: axa::boot::Lookup) -> Result<BootResult> {
        let environment = Environment::Test;
        let config = Config::from_lookup(&environment, &defaults(), |name| lookup(name))?;
        let runtime = Arc::new(Runtime::with_settings(Settings::from_lookup(lookup)));
        let ctx = Self::context(environment, config).await?;
        ctx.shared_store.insert(runtime.clone());
        Self::migrate(&ctx).await?;
        let router = Runtime::scope(runtime, Self::router(&ctx)).await?;
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

    /// The routes in the service's own middleware, inside `axa`'s standard stack. The stack goes on
    /// last, as `axa` 0.6 requires, so a refusal from the service's layers still gets the request id, the
    /// security headers and a problem body, and the client's address is resolved before the audit trail
    /// records it.
    ///
    /// # Errors
    ///
    /// When a setting is wrong, or a port cannot be opened.
    pub async fn router(ctx: &AppContext) -> Result<AxumRouter> {
        let router = Self::routes(ctx).into_router(ctx.clone());
        let router = Self::after_routes(router, ctx).await?;
        // Inside `axa`'s stack, outside the service's own layers: every request reads the settings its app booted with.
        let runtime = ctx
            .shared_store
            .get_ref::<Arc<Runtime>>()
            .map_or_else(Runtime::current, |r| (*r).clone());
        let router = router.layer(axum::middleware::from_fn_with_state(
            runtime,
            crate::runtime::scope_mw,
        ));
        axa::middleware::apply(router, &ctx.config.http)
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
                let flag = crate::runtime::setting(DESTROY_DATA_VAR);
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
        // Asks the running server on this machine for its health, for an image with no `curl` (axa 0.16).
        if let Command::Health { ready, port } = cli.command {
            return axa::cli::health(&environment, &defaults(), ready, port).await;
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
                crate::schema_match::check(&ctx.pool())
                    .await
                    .map_err(Error::Message)?;
                let router = Self::router(&ctx).await?;
                let served = server::serve(router, &ctx.config.server).await;
                // Requests have finished; now the MLLP listener finishes the messages it holds (IH-7), and then
                // the outbox relay finishes its publish (OS-3).
                if !mllp::stop_kept().await {
                    tracing::error!("the MLLP listener did not stop in time");
                }
                crate::outbox::stop_kept().await;
                served
            }
            Command::Db { command } => Self::db(&ctx, command).await,
            Command::Task { .. } => Err(Error::string("this service registers no tasks")),
            // Answered above, before anything connects.
            Command::Check | Command::Health { .. } => Ok(()),
        }
    }

    /// Every route the service serves, from [`controllers::all_routes`], the one list the route census also reads.
    fn routes(_ctx: &AppContext) -> AppRoutes {
        controllers::all_routes()
    }

    /// Wire everything that is not a route, then wrap the routes in the middleware stack.
    ///
    /// In order: the authentication settings (`default_require_auth`, always on in production); the
    /// site time zone; the MLLP listener, only when `EPR_MLLP_BIND` is set; the outbox relay, only when
    /// `EPR_OUTBOX_INTERVAL_SECS` is set; the audit-partition loop (archiving only when
    /// `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` is set); the country packs; the per-subject request limits; the
    /// lockout by client address, only when `LOCKOUT_CLIENT_ADDRESS_AFTER_COUNT` is set; and the
    /// idempotency-key purge loop. A setting that is present but wrong stops the service starting: a feature that
    /// was asked for and is silently absent is worse than no service.
    ///
    /// The layers, outermost (first to run) to innermost: request timing, request id, NUL-byte refusal,
    /// lockout by client address (only when set), authentication, rate limit, authorization, idempotency key.
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        auth::init(
            production_login(&ctx.environment),
            default_require_auth(&ctx.environment),
        )
        .await
        .map_err(Error::Message)?;
        // The site's time zone, for the age on a banner: a name that is not an IANA zone stops the service.
        // Integration tokens (TL-1) sign in against the database; before this, every one is refused.
        crate::auth::init_tokens(ctx.pool());
        crate::site::init().map_err(Error::Message)?;
        // The task rules (TL-3): the site's file (`EPR_TASK_RULES_FILE`) or the built-in catalogue, and the site's
        // holidays (`EPR_SITE_HOLIDAYS`, read by `site::init`). Rules that do not load stop the service here, not at
        // the first result filed; the start says what runs.
        let rules = crate::tasks_store::init_rules().map_err(Error::Message)?;
        tracing::info!(
            rules = rules.len(),
            approved = rules.iter().filter(|r| r.approved()).count(),
            from = if crate::runtime::setting(crate::tasks_store::RULES_FILE)
                .is_some_and(|v| !v.trim().is_empty())
            {
                "site file"
            } else {
                "built-in catalogue"
            },
            "task rules loaded"
        );
        // The reasons for a missed appointment (XP-15): the deployment's file or the built-in list.
        let reasons = crate::missed_reasons::init().map_err(Error::Message)?;
        tracing::info!(reasons = reasons.len(), "missed appointment reasons loaded");
        crate::audit::init_server().map_err(Error::Message)?;
        // The referral timeouts (PC-3): a setting that is wrong stops the service rather than switching one off.
        crate::referral_timeouts::init().map_err(Error::Message)?;
        // The break-glass review deadline and the late-entry limit: a wrong value stops the service (RCA-IN-4).
        crate::break_glass::check_settings().map_err(Error::Message)?;
        crate::disclosure_pack::check_settings().map_err(Error::Message)?;
        crate::cursor::check_settings().map_err(Error::Message)?;
        crate::observations_store::check_settings().map_err(Error::Message)?;
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
            // The feed's health is judged every minute while it listens (IH-8). A bad threshold stops the start.
            let thresholds = crate::feed_health::Thresholds::from_env().map_err(Error::Message)?;
            let runtime = Runtime::current();
            tokio::spawn(crate::feed_health::run_loop(
                runtime.clone(),
                ctx.pool(),
                config.agent.clone(),
                thresholds,
            ));
            // Kept so that `run` can drain it at shutdown (IH-7).
            mllp::keep(mllp::Running::start(&runtime, listener, ctx.pool(), config));
        }
        // The outbox relay is off unless `EPR_OUTBOX_INTERVAL_SECS` is set (OS-3). Its only sink is the
        // log sink, which sends nothing; a real sink arrives with IO-6. A missing table, index or
        // trigger stops the service: there is no way to relay without the record's guards.
        if let Some(every) = crate::outbox::interval_from_env().map_err(Error::Message)? {
            let outbox = axa::outbox::Outbox::connect_existing(
                &ctx.config.database.uri,
                crate::outbox::options(every),
            )
            .await
            .map_err(|e| Error::Message(format!("outbox: {e}")))?;
            let mut sinks = crate::outbox::Sinks::new();
            sinks.insert("log".to_string(), Box::new(crate::outbox::LogSink));
            tracing::warn!(
                seconds = every.as_secs(),
                "outbox relay is ON (log sink only)"
            );
            // Stopped by `run` once the server has stopped: a publish in progress finishes and the rest
            // of the batch goes back.
            crate::outbox::keep(crate::outbox::Running::start(outbox, sinks, every)).await;
        }
        // Audit partitions are made ahead and, when `EPR_AUDIT_ARCHIVE_AFTER_MONTHS` is set, old ones are
        // moved to the archive schema (OS-13). A bad setting stops the service starting.
        let archive_after =
            crate::audit_retention::archive_months_from_env().map_err(Error::Message)?;
        tokio::spawn(crate::audit_retention::run_loop(ctx.pool(), archive_after));
        // The task reminder sender (TL-3, TL-4), off unless `EPR_TASK_REMINDER_MINUTES` is set; a bad value stops
        // the start. Off, no reminder is decided, so the start says so.
        match crate::task_reminders::interval_from(
            crate::runtime::setting(crate::task_reminders::SETTING).as_deref(),
        )
        .map_err(Error::Message)?
        {
            Some(every) => {
                tokio::spawn(crate::task_reminders::run_loop(ctx.pool(), every));
            }
            None => tracing::warn!(
                "task reminders are OFF: set {} to decide them",
                crate::task_reminders::SETTING
            ),
        }
        // Country packs the deployment enables (OS-10). A pack that does not load stops the service.
        crate::packs::init().map_err(Error::Message)?;
        // Request limits per token subject (OS-2). A bad setting stops the service starting.
        let limits = crate::rate_limit::Limits::from_env().map_err(Error::Message)?;
        let limiters = crate::rate_limit::Limiters::new(limits);
        // Lockout by client address after too many `401`s, off unless `LOCKOUT_CLIENT_ADDRESS_AFTER_COUNT`
        // is set (OS-2, H-049). A bad setting stops the service starting.
        let lockout_after = crate::lockout::count_from_env().map_err(Error::Message)?;
        let lockout = lockout_after.map(crate::lockout::limiter);
        if let Some(after) = lockout_after {
            tracing::warn!(
                after = after.get(),
                window_seconds = crate::lockout::WINDOW_SECS,
                "lockout by client address is ON: behind a proxy not listed in TRUSTED_PROXIES, every user is one client"
            );
        }
        // Permission at the point of care (WL-4): off unless set; a bad setting stops the service starting.
        if crate::permission::init().map_err(Error::Message)? {
            tracing::info!("a permission basis is required for staff reads of clinical content");
        }
        // Expired idempotency keys hold answers that can name a patient: remove them (OF-1).
        // A missing or out-of-date table stops the service here, not at its first keyed create.
        let keys = crate::idempotency::keys(ctx.pool())
            .map_err(|e| Error::Message(format!("idempotency keys: {e}")))?;
        keys.check()
            .await
            .map_err(|e| Error::Message(format!("idempotency keys: {e}")))?;
        tokio::spawn(crate::idempotency::run_loop(keys.clone()));
        // The last layer added runs first: authenticate, count, authorize, then the idempotency key.
        // Array limits run inside idempotency, before a handler can write a version; a replay was
        // checked on its first run and does not read its body again.
        let router = router.layer(axum::middleware::from_fn(crate::list_limits::enforce_mw));
        let gate = authz::Authz { db: ctx.pool() };
        let router = crate::idempotency::layer(router, keys, ctx.pool()).layer(
            axum::middleware::from_fn_with_state(gate, authz::authorize_mw),
        );
        let router = crate::rate_limit::layer(router, limiters, ctx.pool())
            .layer(axum::middleware::from_fn(auth::require_auth_mw));
        // Outside authentication, so it counts authentication's `401`s and refuses before a token is checked.
        Ok(crate::lockout::layer(router, lockout, ctx.pool())
            .layer(axum::middleware::from_fn(authz::no_nul_mw))
            // The request id is settled before authentication or the audit trail.
            .layer(axum::middleware::from_fn(authz::request_id_mw))
            // Outermost of the service's own: time every request, refusals included, labelled by route template
            // (OS-6). `axa`'s standard stack is outside it (`router`).
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
