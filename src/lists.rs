//! A list of lists in two flat arrays (#254).
//!
//! The values of every list follow one another in one array, and a second
//! array holds where each list ends. Building one costs no allocation per
//! list, and reading list `i` is two loads. Merged groups, classified faces,
//! and the published results keep their per-item lists this way.

/// Lists of `T`, numbered in the order they were pushed.
#[derive(Clone, Debug)]
pub(crate) struct Lists<T> {
    values: Vec<T>,
    /// `ends[i - 1]..ends[i]` is list `i`, and list 0 starts at 0: one
    /// entry per list. A default value allocates nothing, so a workspace
    /// that holds lists costs nothing until it is used (#531), and a push
    /// writes one entry with nothing to test first. With a leading start
    /// kept in the array, an empty value had to write it in its first push,
    /// and the test for that slowed every push (#570, #578).
    ends: Vec<u32>,
}

impl<T> Default for Lists<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            ends: Vec::new(),
        }
    }
}

impl<T: PartialEq> PartialEq for Lists<T> {
    /// The same lists in the same order.
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values && self.ends == other.ends
    }
}

impl<T: Eq> Eq for Lists<T> {}

impl<T: Copy> Lists<T> {
    /// No lists, with room for `lists` lists of `values` values in all.
    pub(crate) fn with_capacity(lists: usize, values: usize) -> Self {
        Self {
            values: Vec::with_capacity(values),
            ends: Vec::with_capacity(lists),
        }
    }

    /// Number of lists.
    pub(crate) fn len(&self) -> usize {
        self.ends.len()
    }

    /// Whether there are no lists.
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Removes every list and keeps the storage, for a buffer reused from
    /// one item to the next.
    pub(crate) fn clear(&mut self) {
        self.values.clear();
        self.ends.clear();
    }

    /// Room for `lists` more lists holding `values` more values in all.
    pub(crate) fn reserve(&mut self, lists: usize, values: usize) {
        self.ends.reserve(lists);
        self.values.reserve(values);
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
    #[inline]
    fn close(&mut self) {
        // Values are indexed by `u32`; more is an exhaustion of the index
        // space, which aborts like an allocation failure (design §3).
        let Ok(end) = u32::try_from(self.values.len()) else {
            std::process::abort();
        };
        self.ends.push(end);
    }

    /// Where list `i` starts: the end of the list before it, or 0 for the
    /// first. The lookup one before the first is out of bounds and reads
    /// as 0, so the first list costs no test of its own.
    #[inline]
    fn start(&self, i: usize) -> usize {
        self.ends
            .get(i.wrapping_sub(1))
            .map_or(0, |&end| end as usize)
    }

    /// List `i`. Panics past the end, like slice indexing; callers pass a
    /// number below [`Self::len`].
    pub(crate) fn get(&self, i: usize) -> &[T] {
        &self.values[self.start(i)..self.ends[i] as usize]
    }

    /// List `i`, to be changed in place.
    pub(crate) fn get_mut(&mut self, i: usize) -> &mut [T] {
        let (start, end) = (self.start(i), self.ends[i] as usize);
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
        assert_eq!(lists, Lists::with_capacity(2, 4));
        assert_eq!(lists.iter().count(), 0);
        lists.push(&[3, 1]);
        lists.push(&[]);
        lists.push_iter([7, 8, 9]);
        assert_eq!(lists.len(), 3);
        assert_eq!(lists.get(0), &[3, 1]);
        assert!(lists.get(1).is_empty());
        assert_eq!(lists.get(2), &[7, 8, 9]);
        lists.get_mut(2).sort_unstable_by(|a, b| b.cmp(a));
        let all: Vec<&[u32]> = lists.iter().collect();
        assert_eq!(all, vec![&[3, 1][..], &[][..], &[9, 8, 7][..]]);
        lists.clear();
        assert!(lists.is_empty());
        assert_eq!(lists, Lists::default());
    }
}
