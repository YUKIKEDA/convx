//! The plane numbers each point was found on during construction (design
//! §3, #306).
//!
//! When the exact sign of a point against a simplex is zero, the point and
//! the simplex's plane number are recorded. Simplices with one number share
//! one supporting plane and one outer side, so a later sign of that point
//! against a simplex with that number is zero and is not evaluated.
//!
//! The record is flat: one list of entries, and per point the start of its
//! chain in that list. A point's newest entry comes first. Nothing is
//! allocated until the first record, so an input in general position pays
//! nothing.

/// No entry: the end of a chain, or a point with no record.
const NONE: u32 = u32::MAX;

/// The recorded plane numbers of every point.
#[derive(Default)]
pub(crate) struct Records {
    /// Per point, the index in `entries` of its newest record, or [`NONE`].
    /// Empty until the first record.
    starts: Vec<u32>,
    /// A plane number and the index of the point's previous entry.
    entries: Vec<(u32, u32)>,
    /// One past the largest point number.
    points: usize,
}

impl Records {
    /// No records, for points numbered below `points`.
    pub(crate) fn new(points: usize) -> Self {
        Self {
            starts: Vec::new(),
            entries: Vec::new(),
            points,
        }
    }

    /// Whether no point has a record.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The plane numbers recorded for `point`, newest first.
    pub(crate) fn numbers(&self, point: u32) -> impl Iterator<Item = u32> + '_ {
        let mut at = self.starts.get(point as usize).copied().unwrap_or(NONE);
        core::iter::from_fn(move || {
            let &(number, previous) = self.entries.get(at as usize)?;
            at = previous;
            Some(number)
        })
    }

    /// Whether `point` was recorded on plane `number`.
    pub(crate) fn holds(&self, point: u32, number: u32) -> bool {
        self.numbers(point).any(|n| n == number)
    }

    /// Records that `point` is on plane `number`.
    pub(crate) fn record(&mut self, point: u32, number: u32) {
        debug_assert!(!self.holds(point, number), "a pair is recorded once");
        if self.starts.is_empty() {
            self.starts.resize(self.points, NONE);
        }
        let at = self.entries.len();
        if at >= NONE as usize {
            // An exhaustion of the index space aborts, as an allocation
            // failure does (design §3).
            std::process::abort();
        }
        let previous = core::mem::replace(&mut self.starts[point as usize], at as u32);
        self.entries.push((number, previous));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_read_back_newest_first_per_point() {
        let mut records = Records::new(4);
        assert_eq!(records.numbers(2).count(), 0);
        assert!(!records.holds(2, 0));
        records.record(2, 7);
        records.record(0, 7);
        records.record(2, 3);
        assert_eq!(records.numbers(2).collect::<Vec<_>>(), [3, 7]);
        assert_eq!(records.numbers(0).collect::<Vec<_>>(), [7]);
        assert_eq!(records.numbers(1).count(), 0);
        assert!(records.holds(2, 7) && records.holds(2, 3) && records.holds(0, 7));
        assert!(!records.holds(0, 3) && !records.holds(3, 7));
    }
}
