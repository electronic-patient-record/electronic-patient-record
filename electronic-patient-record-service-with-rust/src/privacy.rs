//! The privacy rule for aggregates (PA-7): what a count table may show, and how many tables one caller may ask
//! for. Pure, like the matcher: the caller passes the counts and the budget state. Nothing in the service
//! produces an aggregate yet; OR-7, TR-10, IO-7 and IO-14 are the intended callers.
//!
//! **Small cells.** A count from 1 to `k - 1` identifies too few people and is hidden. A zero is shown: it says
//! nobody, which discloses no one. `k` is at least [`MIN_K`]; a smaller value is raised to it.
//!
//! **Secondary suppression.** When a table also publishes its total, hiding exactly one cell lets a reader get
//! it back by subtraction. So when one cell is hidden, the smallest cell still shown is hidden with it, and a
//! reader can then recover only the sum of the two, which is at least `k`... unless that is also small, in which
//! case the loop repeats. The total is never hidden: a table with a hidden total and hidden cells would show
//! nothing useful.
//!
//! **What this does not prevent.** *Differencing*: two tables over populations that differ by a few people
//! (this month and last month, or "all" and "all but one clinic") give those few people's count by
//! subtraction, and each table is safe alone. *Linkage* with other data a reader has. *Repeated near-identical
//! queries* that average out any noise. The [`Budget`] limits how many tables one caller can ask for, which
//! slows differencing and does not stop it. A pair-of-queries check is not built (PA-7 stays open).

/// The least `k` the service accepts, whatever is configured.
pub const MIN_K: u64 = 5;

/// One cell of a published table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cell {
    /// The count is shown.
    Shown(u64),
    /// The count is hidden (it was small, or hidden to protect a small one).
    Hidden,
}

/// A labelled count, and the same after suppression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// What was counted.
    pub label: String,
    /// The count, or hidden.
    pub cell: Cell,
}

/// Suppress a table whose rows add up to a published total.
///
/// Rows with `1..k` are hidden; if exactly one is hidden, or the hidden rows sum to less than `k`, the
/// smallest shown row is hidden too, until the hidden sum is at least `k` or nothing is left to hide.
/// The total is returned unchanged (`None` when `publish_total` is false: then no secondary suppression is
/// needed, because nothing can be subtracted from).
#[must_use]
pub fn suppress(counts: &[(String, u64)], k: u64, publish_total: bool) -> (Vec<Row>, Option<u64>) {
    let k = k.max(MIN_K);
    let mut hidden: Vec<bool> = counts.iter().map(|(_, n)| (1..k).contains(n)).collect();
    if publish_total {
        loop {
            let hidden_sum: u64 = counts
                .iter()
                .zip(&hidden)
                .filter(|(_, h)| **h)
                .map(|((_, n), _)| *n)
                .sum();
            let any = hidden.iter().any(|h| *h);
            if !any || hidden_sum >= k {
                break;
            }
            // Hide the smallest row still shown and not zero (a zero hides nothing and protects nothing).
            let next = counts
                .iter()
                .zip(&hidden)
                .enumerate()
                .filter(|(_, ((_, n), h))| !**h && *n > 0)
                .min_by_key(|(_, ((_, n), _))| *n)
                .map(|(i, _)| i);
            match next {
                Some(i) => hidden[i] = true,
                None => break,
            }
        }
    }
    let rows = counts
        .iter()
        .zip(hidden)
        .map(|((label, n), h)| Row {
            label: label.clone(),
            cell: if h { Cell::Hidden } else { Cell::Shown(*n) },
        })
        .collect();
    let total = publish_total.then(|| counts.iter().map(|(_, n)| *n).sum());
    (rows, total)
}

/// How many tables one caller may ask for in a window. The caller is whatever the service identifies
/// (the token `sub`); the window and the count are the caller's own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    /// Tables allowed per window.
    pub limit: u32,
    /// Length of a window, in seconds.
    pub window: i64,
    /// When the current window began, seconds since 1970-01-01 UTC.
    pub started: i64,
    /// Tables asked for in the current window.
    pub used: u32,
}

/// Whether a table may be produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Allowance {
    /// Go ahead; this many remain after it.
    Allowed {
        /// Tables left in this window after this one.
        left: u32,
    },
    /// The budget is spent; try again at this time (seconds since 1970-01-01 UTC).
    Refused {
        /// When the window ends.
        retry_at: i64,
    },
}

impl Budget {
    /// A new budget, starting at `now`.
    #[must_use]
    pub fn new(limit: u32, window: i64, now: i64) -> Self {
        Budget {
            limit,
            window,
            started: now,
            used: 0,
        }
    }

    /// Ask for one table at `now`. A refused request is not counted, and a new window starts when the old
    /// one has ended.
    pub fn ask(&mut self, now: i64) -> Allowance {
        if now >= self.started + self.window {
            self.started = now;
            self.used = 0;
        }
        if self.used >= self.limit {
            return Allowance::Refused {
                retry_at: self.started + self.window,
            };
        }
        self.used += 1;
        Allowance::Allowed {
            left: self.limit - self.used,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(counts: &[u64]) -> Vec<(String, u64)> {
        counts
            .iter()
            .enumerate()
            .map(|(i, n)| (format!("g{i}"), *n))
            .collect()
    }

    fn shown(rows: &[Row]) -> Vec<Option<u64>> {
        rows.iter()
            .map(|r| match r.cell {
                Cell::Shown(n) => Some(n),
                Cell::Hidden => None,
            })
            .collect()
    }

    #[test]
    fn a_count_under_k_is_hidden_and_k_itself_is_shown() {
        let (rows, _) = suppress(&table(&[4, 5, 6]), 5, false);
        assert_eq!(shown(&rows), vec![None, Some(5), Some(6)]);
    }

    #[test]
    fn a_zero_is_shown() {
        let (rows, _) = suppress(&table(&[0, 9]), 5, false);
        assert_eq!(shown(&rows), vec![Some(0), Some(9)]);
    }

    #[test]
    fn k_below_the_minimum_is_raised() {
        let (rows, _) = suppress(&table(&[3, 8]), 1, false);
        assert_eq!(shown(&rows), vec![None, Some(8)]);
    }

    #[test]
    fn with_no_total_published_nothing_beyond_the_small_cells_is_hidden() {
        let (rows, total) = suppress(&table(&[2, 40, 50]), 5, false);
        assert_eq!(shown(&rows), vec![None, Some(40), Some(50)]);
        assert_eq!(total, None);
    }

    #[test]
    fn one_hidden_cell_with_a_published_total_hides_the_smallest_other_cell() {
        let (rows, total) = suppress(&table(&[2, 40, 50]), 5, true);
        assert_eq!(shown(&rows), vec![None, None, Some(50)]);
        assert_eq!(total, Some(92));
    }

    #[test]
    fn the_hidden_sum_is_at_least_k_so_neither_hidden_cell_is_recoverable() {
        let counts = table(&[1, 2, 7, 90]);
        let (rows, _) = suppress(&counts, 5, true);
        let hidden_sum: u64 = counts
            .iter()
            .zip(&rows)
            .filter(|(_, r)| r.cell == Cell::Hidden)
            .map(|((_, n), _)| *n)
            .sum();
        assert!(hidden_sum >= 5, "{rows:?}");
        // 1 and 2 are small; together 3 < 5, so 7 is hidden as well.
        assert_eq!(shown(&rows), vec![None, None, None, Some(90)]);
    }

    #[test]
    fn two_small_cells_that_already_sum_to_k_need_no_third() {
        let (rows, _) = suppress(&table(&[2, 3, 40]), 5, true);
        assert_eq!(shown(&rows), vec![None, None, Some(40)]);
    }

    #[test]
    fn a_table_with_nothing_left_to_hide_stops() {
        let (rows, total) = suppress(&table(&[1, 0, 0]), 5, true);
        assert_eq!(shown(&rows), vec![None, Some(0), Some(0)]);
        assert_eq!(total, Some(1));
    }

    #[test]
    fn the_total_is_never_hidden() {
        let (_, total) = suppress(&table(&[1]), 5, true);
        assert_eq!(total, Some(1));
    }

    #[test]
    fn a_budget_allows_its_limit_then_refuses_until_the_window_ends() {
        let mut b = Budget::new(2, 60, 1_000);
        assert_eq!(b.ask(1_000), Allowance::Allowed { left: 1 });
        assert_eq!(b.ask(1_010), Allowance::Allowed { left: 0 });
        assert_eq!(b.ask(1_020), Allowance::Refused { retry_at: 1_060 });
        assert_eq!(
            b.ask(1_059),
            Allowance::Refused { retry_at: 1_060 },
            "a refusal is not counted and does not move the window"
        );
        assert_eq!(b.ask(1_060), Allowance::Allowed { left: 1 }, "a new window");
    }

    #[test]
    fn a_budget_of_zero_always_refuses() {
        let mut b = Budget::new(0, 60, 0);
        assert!(matches!(b.ask(0), Allowance::Refused { .. }));
        assert!(matches!(b.ask(10_000), Allowance::Refused { .. }));
    }
}
