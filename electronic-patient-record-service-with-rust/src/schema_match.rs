//! Whether the database is the one this build was made for (H-074). Production does not migrate at start, so an older
//! build could be started on a newer database and would not know what the newer one keeps: it would show an entry whose
//! label it has never heard of, and write rows without the rules added since. And a newer build on an older database
//! would answer errors. `start` refuses both.
//!
//! A newer database may be allowed by [`ALLOW_NEWER`], for a person who has decided that an outage is worse. Nothing
//! allows an older one. **A build from before this check does not make it, and a server already running when the database
//! is migrated is not stopped.**

use crate::pool::Db;
use sqlx::Row;
use std::collections::BTreeSet;

/// `true` starts this build on a database with migrations it does not know. Anything else is refused at start.
pub const ALLOW_NEWER: &str = "EPR_START_ON_NEWER_DATABASE";

/// How the database differs from this build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    /// Migrations the database has and this build does not know.
    pub newer: Vec<i64>,
    /// Migrations this build has and the database does not, or that failed there.
    pub pending: Vec<i64>,
}

/// Compare what this build knows with what the database has applied.
#[must_use]
pub fn compare(known: &BTreeSet<i64>, applied: &BTreeSet<i64>) -> Difference {
    Difference {
        newer: applied.difference(known).copied().collect(),
        pending: known.difference(applied).copied().collect(),
    }
}

/// Why this build may not start, if it may not. `allow_newer` is [`ALLOW_NEWER`] as set.
///
/// # Errors
///
/// The text for the person who started it.
pub fn judge(difference: &Difference, allow_newer: Option<&str>) -> Result<(), String> {
    let allowed = match allow_newer.map(str::trim) {
        None => false,
        Some("true") => true,
        Some("false") => false,
        Some(_) => return Err(format!("{ALLOW_NEWER} must be true or false")),
    };
    if !difference.pending.is_empty() {
        return Err(format!(
            "the database is older than this build: {} migration(s) not applied, the first {}. Run `db migrate` first",
            difference.pending.len(),
            list(&difference.pending)
        ));
    }
    if !difference.newer.is_empty() && !allowed {
        return Err(format!(
            "the database is newer than this build: it has {} migration(s) that this build does not know, the first {}. \
             Start the newer build: an older one may show what the newer one withholds. {ALLOW_NEWER}=true starts it anyway",
            difference.newer.len(),
            list(&difference.newer)
        ));
    }
    Ok(())
}

/// The first of `versions`, for a message.
fn list(versions: &[i64]) -> String {
    versions.first().map_or_else(String::new, i64::to_string)
}

/// The migrations the database has applied without failing. A database never migrated has none.
///
/// # Errors
///
/// A database error.
pub async fn applied(db: impl Into<Db<'_>>) -> Result<BTreeSet<i64>, sqlx::Error> {
    let mut conn = db.into().acquire().await?;
    let there: bool = sqlx::query("SELECT to_regclass('_sqlx_migrations') IS NOT NULL AS there")
        .fetch_one(&mut *conn)
        .await?
        .try_get("there")?;
    if !there {
        return Ok(BTreeSet::new());
    }
    sqlx::query("SELECT version FROM _sqlx_migrations WHERE success")
        .fetch_all(&mut *conn)
        .await?
        .iter()
        .map(|row| row.try_get("version"))
        .collect()
}

/// Refuse to start on a database that is not the one this build was made for.
///
/// # Errors
///
/// [`judge`]'s text, or that the database could not be asked.
pub async fn check(db: impl Into<Db<'_>>) -> Result<(), String> {
    let known = crate::MIGRATOR.iter().map(|m| m.version).collect();
    let applied = applied(db)
        .await
        .map_err(|e| format!("cannot read the database's migrations: {e}"))?;
    let difference = compare(&known, &applied);
    judge(&difference, crate::runtime::setting(ALLOW_NEWER).as_deref())?;
    if !difference.newer.is_empty() {
        tracing::warn!(
            unknown_migrations = difference.newer.len(),
            "started on a database newer than this build: what it withholds or requires may not be kept"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(versions: &[i64]) -> BTreeSet<i64> {
        versions.iter().copied().collect()
    }

    #[test]
    fn only_the_same_migrations_start_and_only_a_newer_database_may_be_allowed() {
        let same = compare(&set(&[1, 2]), &set(&[1, 2]));
        let newer = compare(&set(&[1, 2]), &set(&[1, 2, 3]));
        let older = compare(&set(&[1, 2]), &set(&[1]));
        let both = compare(&set(&[1, 2]), &set(&[1, 3]));
        let empty = compare(&set(&[1, 2]), &set(&[]));
        for allow in [None, Some("false"), Some("true")] {
            let is = |d: &Difference| judge(d, allow).is_ok();
            assert!(is(&same), "{allow:?}");
            assert_eq!(is(&newer), allow == Some("true"), "{allow:?}");
            assert!(!is(&older) && !is(&both) && !is(&empty), "{allow:?}");
        }
        assert!(judge(&newer, None).unwrap_err().contains("first 3"));
        assert!(judge(&older, None).unwrap_err().contains("db migrate"));
        // A setting that is wrong stops the start, with the database right too.
        for wrong in ["", "yes", "1", "TRUE"] {
            assert!(judge(&same, Some(wrong)).is_err(), "{wrong:?}");
        }
    }
}
