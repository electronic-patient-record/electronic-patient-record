//! loco.rs application wiring: the [`App`] `Hooks` implementation.

use async_trait::async_trait;
use axum::Router as AxumRouter;
use electronic_patient_record_migration::Migrator;
use loco_rs::{
    Result,
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{BootResult, StartMode, create_app},
    config::Config,
    controller::AppRoutes,
    environment::Environment,
    task::Tasks,
};
use std::path::Path;

use crate::{auth, authz, controllers, mllp};

/// The loco.rs application hooks.
pub struct App;

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        controllers::all_routes()
    }

    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let config_require = ctx
            .config
            .settings
            .as_ref()
            .and_then(|s| s.get("require_auth"))
            .and_then(serde_json::Value::as_bool);
        auth::init(ctx.environment == Environment::Production, config_require)
            .await
            .map_err(loco_rs::Error::Message)?;
        // The site's time zone, for the age on a banner: a name that is not an IANA zone stops the service.
        crate::site::init().map_err(loco_rs::Error::Message)?;
        // The MLLP listener for HL7 v2 ADT feeds is off unless `EPR_MLLP_BIND` is set. A bad
        // setting, or a port that cannot be opened, stops the service starting: a feed that was
        // asked for and is silently absent is worse than no service.
        if let Some(config) = mllp::Config::from_env().map_err(loco_rs::Error::Message)? {
            let listener = tokio::net::TcpListener::bind(config.bind)
                .await
                .map_err(|e| {
                    loco_rs::Error::Message(format!(
                        "cannot listen for MLLP on {}: {e}",
                        config.bind
                    ))
                })?;
            tracing::warn!(
                bind = %config.bind,
                agent = %config.agent,
                "MLLP listener is ON: it has no authentication or encryption"
            );
            tokio::spawn(mllp::serve(listener, ctx.db.clone(), config));
        }
        // The outbox loop is off unless `EPR_OUTBOX_INTERVAL_SECS` is set (OS-3). Its only sink is the
        // log sink, which sends nothing; a real sink arrives with IO-6.
        if let Some(every) = crate::outbox::interval_from_env().map_err(loco_rs::Error::Message)? {
            let mut sinks = crate::outbox::Sinks::new();
            sinks.insert("log".to_string(), Box::new(crate::outbox::LogSink));
            tracing::warn!(
                seconds = every.as_secs(),
                "outbox loop is ON (log sink only)"
            );
            tokio::spawn(crate::outbox::run_loop(
                ctx.db.clone(),
                crate::audit::chain(),
                sinks,
                every,
            ));
        }
        // The last layer added runs first: authenticate, then authorize.
        let gate = authz::Authz { db: ctx.db.clone() };
        Ok(router
            .layer(axum::middleware::from_fn_with_state(
                gate,
                authz::authorize_mw,
            ))
            .layer(axum::middleware::from_fn(auth::require_auth_mw))
            .layer(axum::middleware::from_fn(authz::no_nul_mw))
            // The request id is settled before authentication or the audit trail.
            .layer(axum::middleware::from_fn(authz::request_id_mw))
            // Outermost: time every request, refusals included, labelled by route template (OS-6).
            .layer(axum::middleware::from_fn(crate::metrics::duration_mw)))
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    fn register_tasks(_tasks: &mut Tasks) {}

    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
