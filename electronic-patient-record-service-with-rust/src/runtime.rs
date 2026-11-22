//! The settings one running service reads, and the way code finds them (QA-16).
//!
//! A service used to read `std::env::var` wherever it needed a setting, so a test that wanted a different
//! setting had to change the process environment, which every thread shares. Now a [`Runtime`] holds one
//! service's settings, and [`current`] gives the one the running request belongs to:
//!
//! - inside a request, the runtime the app was booted with, put there by [`scope_mw`];
//! - anywhere else (a job, the command line, a unit test that calls a function directly), the process
//!   default, which reads the process environment each time it is asked, exactly as the service did before.
//!
//! A service booted from the process environment ([`Runtime::process`]) therefore behaves as it always did, and
//! one booted with [`TestApp::boot_test_with`](axa::boot::TestApp::boot_test_with) reads the settings its
//! test named, with no process environment involved, so tests in one program can differ.
//!
//! A task spawned from a request does not inherit the runtime (a tokio task-local belongs to its task); code
//! that spawns must pass `Runtime::current()` in and call [`Runtime::scope`] on the new task.

use std::future::Future;
use std::sync::Arc;

use axum::{extract::Request, middleware::Next, response::Response};

/// Where a setting's value comes from: a lookup that gives the value of a name, or `None` when unset.
#[derive(Clone)]
pub struct Settings(axa::boot::Lookup);

impl Settings {
    /// The process environment, read at each call, so a change made later is seen.
    #[must_use]
    pub fn process() -> Self {
        Self(Arc::new(|name| std::env::var(name).ok()))
    }

    /// Settings given as pairs, and nothing else: no process environment.
    #[must_use]
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        let map: std::collections::BTreeMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        Self(Arc::new(move |name| map.get(name).cloned()))
    }

    /// Settings read through the lookup `axa` gives `boot_test_with`.
    #[must_use]
    pub fn from_lookup(lookup: axa::boot::Lookup) -> Self {
        Self(lookup)
    }

    /// The value of `name`, or `None` when unset. A blank value is returned as it is; callers that treat
    /// blank as unset say so.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<String> {
        (self.0)(name)
    }

    /// The lookup, for the constructors that take one (`from_lookup`).
    pub fn lookup(&self) -> impl Fn(&str) -> Option<String> + '_ {
        move |name| self.get(name)
    }
}

impl std::fmt::Debug for Settings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print values: settings hold keys.
        f.write_str("Settings(..)")
    }
}

/// One running service's settings.
#[derive(Clone, Debug)]
pub struct Runtime {
    settings: Settings,
}

tokio::task_local! {
    static CURRENT: Arc<Runtime>;
}

impl Runtime {
    /// A runtime that reads the process environment: what a service booted the usual way has.
    #[must_use]
    pub fn process() -> Self {
        Self {
            settings: Settings::process(),
        }
    }

    /// A runtime that reads `settings` and nothing else.
    #[must_use]
    pub fn with_settings(settings: Settings) -> Self {
        Self { settings }
    }

    /// The runtime of the running request, or the process default outside one.
    #[must_use]
    pub fn current() -> Arc<Runtime> {
        CURRENT
            .try_with(Arc::clone)
            .unwrap_or_else(|_| Arc::new(Runtime::process()))
    }

    /// This runtime's settings.
    #[must_use]
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Run `future` with `runtime` as [`Runtime::current`].
    pub async fn scope<F: Future>(runtime: Arc<Runtime>, future: F) -> F::Output {
        CURRENT.scope(runtime, future).await
    }
}

/// The value of setting `name` in the current runtime.
#[must_use]
pub fn setting(name: &str) -> Option<String> {
    Runtime::current().settings.get(name)
}

/// Middleware: make `runtime` the current one for the rest of the request.
pub async fn scope_mw(
    axum::extract::State(runtime): axum::extract::State<Arc<Runtime>>,
    request: Request,
    next: Next,
) -> Response {
    Runtime::scope(runtime, next.run(request)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_are_all_that_is_visible() {
        let s = Settings::from_pairs(&[("A", "1")]);
        assert_eq!(s.get("A").as_deref(), Some("1"));
        assert_eq!(
            s.get("PATH"),
            None,
            "the process environment is not consulted"
        );
        assert_eq!(format!("{s:?}"), "Settings(..)");
    }

    #[tokio::test]
    async fn a_scope_changes_what_current_reads_and_only_inside_it() {
        let rt = Arc::new(Runtime::with_settings(Settings::from_pairs(&[("X", "in")])));
        assert_eq!(setting("X"), None);
        let inside = Runtime::scope(rt, async { setting("X") }).await;
        assert_eq!(inside.as_deref(), Some("in"));
        assert_eq!(setting("X"), None);
    }

    #[tokio::test]
    async fn two_scopes_at_once_do_not_see_each_other() {
        let a = Arc::new(Runtime::with_settings(Settings::from_pairs(&[("X", "a")])));
        let b = Arc::new(Runtime::with_settings(Settings::from_pairs(&[("X", "b")])));
        let (ra, rb) = tokio::join!(
            Runtime::scope(a, async {
                tokio::task::yield_now().await;
                setting("X")
            }),
            Runtime::scope(b, async {
                tokio::task::yield_now().await;
                setting("X")
            }),
        );
        assert_eq!((ra.as_deref(), rb.as_deref()), (Some("a"), Some("b")));
    }

    #[test]
    fn the_process_default_reads_the_process_environment() {
        assert_eq!(
            Runtime::process().settings().get("PATH"),
            std::env::var("PATH").ok()
        );
        let s = Settings::from_pairs(&[("K", "v")]);
        assert_eq!(s.lookup()("K").as_deref(), Some("v"));
        let l = Settings::from_lookup(Arc::new(|n| (n == "Z").then(|| "z".to_string())));
        assert_eq!(l.get("Z").as_deref(), Some("z"));
    }
}
