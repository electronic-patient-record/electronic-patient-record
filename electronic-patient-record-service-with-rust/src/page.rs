//! A list that says when it was cut short (PR-1).
//!
//! The work queues are shown oldest first and capped. A cap with no sign of itself hides the newest entry once the
//! queue is full, so a store fetches one row beyond the cap and [`Page::cut`] turns that extra row into `more`.

use std::ops::Deref;

/// The rows up to a cap, and whether any were left behind.
#[derive(Debug, Clone)]
pub struct Page<T> {
    /// At most `limit` rows, in the order the store gave them.
    pub rows: Vec<T>,
    /// True when at least one more row exists than the cap allowed.
    pub more: bool,
}

impl<T> Page<T> {
    /// Cut `fetched` (which holds up to `limit + 1` rows) to `limit` rows and record whether a row was dropped.
    #[must_use]
    pub fn cut(mut fetched: Vec<T>, limit: u64) -> Self {
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        let more = fetched.len() > limit;
        fetched.truncate(limit);
        Self {
            rows: fetched,
            more,
        }
    }
}

impl<T> Deref for Page<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Vec<T> {
        &self.rows
    }
}

#[cfg(test)]
mod tests {
    use super::Page;

    #[test]
    fn a_full_list_is_not_cut_and_one_more_row_is() {
        assert!(!Page::cut((0..50).collect::<Vec<_>>(), 50).more);
        let p = Page::cut((0..51).collect::<Vec<_>>(), 50);
        assert!(p.more);
        assert_eq!(p.len(), 50);
        assert_eq!(p.rows.last(), Some(&49));
        assert!(!Page::<u8>::cut(vec![], 50).more);
    }
}
