//! A list of lists in two flat arrays (#254).
//!
//! The values of every list follow one another in one array, and a second
//! array holds where each list starts. Building one costs no allocation per
//! list, and reading list `i` is two loads. Merged groups, classified faces,
//! and the published results keep their per-item lists this way.

/// Lists of `T`, numbered in the order they were pushed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Lists<T> {
    values: Vec<T>,
    /// `starts[i]..starts[i + 1]` is list `i`; one more entry than lists.
    starts: Vec<u32>,
}

impl<T> Default for Lists<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            starts: vec![0],
        }
    }
}

impl<T: Copy> Lists<T> {
    /// No lists, with room for `lists` lists of `values` values in all.
    pub(crate) fn with_capacity(lists: usize, values: usize) -> Self {
        let mut starts = Vec::with_capacity(lists + 1);
        starts.push(0);
        Self {
            values: Vec::with_capacity(values),
            starts,
        }
    }

    /// Number of lists.
    pub(crate) fn len(&self) -> usize {
        self.starts.len() - 1
    }

    /// Whether there are no lists.
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends one list.
    pub(crate) fn push(&mut self, items: &[T]) {
        self.values.extend_from_slice(items);
        self.close();
    }

    /// Appends one list of the items of `items`.
    pub(crate) fn push_iter(&mut self, items: impl IntoIterator<Item = T>) {
        self.values.extend(items);
        self.close();
    }

    /// Ends the list that the values pushed since the last one form.
    fn close(&mut self) {
        // Values are indexed by `u32`; more is an exhaustion of the index
        // space, which aborts like an allocation failure (design §3).
        let Ok(end) = u32::try_from(self.values.len()) else {
            std::process::abort();
        };
        self.starts.push(end);
    }

    /// List `i`. Panics past the end, like slice indexing; callers pass a
    /// number below [`Self::len`].
    pub(crate) fn get(&self, i: usize) -> &[T] {
        &self.values[self.starts[i] as usize..self.starts[i + 1] as usize]
    }

    /// List `i`, or `None` past the end.
    pub(crate) fn try_get(&self, i: usize) -> Option<&[T]> {
        let start = *self.starts.get(i)? as usize;
        let end = *self.starts.get(i + 1)? as usize;
        Some(&self.values[start..end])
    }

    /// List `i`, to be changed in place.
    pub(crate) fn get_mut(&mut self, i: usize) -> &mut [T] {
        let (start, end) = (self.starts[i] as usize, self.starts[i + 1] as usize);
        &mut self.values[start..end]
    }

    /// Every list, in order.
    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = &[T]> + '_ {
        (0..self.len()).map(move |i| self.get(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_read_back_in_order_with_empty_ones() {
        let mut lists = Lists::default();
        assert!(lists.is_empty());
        lists.push(&[3, 1]);
        lists.push(&[]);
        lists.push_iter([7, 8, 9]);
        assert_eq!(lists.len(), 3);
        assert_eq!(lists.get(0), &[3, 1]);
        assert!(lists.get(1).is_empty());
        assert_eq!(lists.get(2), &[7, 8, 9]);
        assert_eq!(lists.try_get(3), None);
        lists.get_mut(2).sort_unstable_by(|a, b| b.cmp(a));
        let all: Vec<&[u32]> = lists.iter().collect();
        assert_eq!(all, vec![&[3, 1][..], &[][..], &[9, 8, 7][..]]);
    }
}
