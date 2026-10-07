//! Ids of facets during construction, and a value per id.
//!
//! A [`FacetId`] carries a slot index of the facet store and the generation
//! the slot had when the facet was added. Removing a facet bumps the
//! generation, so a stale id never reaches a reused slot.
//!
//! The store is single-threaded: construction inserts one point at a time
//! (§6), so no insert or remove is ever concurrent.

/// A value per facet id, for one search or one pass at a time, without
/// hashing (#120).
///
/// Entries are indexed by slot and stamped with the id's generation and the
/// current epoch. [`Self::clear`] starts a new epoch, so clearing does not
/// touch the entries, and a stale id never matches the entry of a reused
/// slot.
pub(crate) struct SlotMarks<V> {
    entries: Vec<(u32, u32, V)>,
    epoch: u32,
}

impl<V: Copy + Default> Default for SlotMarks<V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            epoch: 1,
        }
    }
}

impl<V: Copy + Default> SlotMarks<V> {
    /// Forgets every value.
    #[cfg(test)]
    pub(crate) fn clear(&mut self) {
        if self.epoch == u32::MAX {
            self.entries.clear();
            self.epoch = 1;
        } else {
            self.epoch += 1;
        }
    }

    /// The value of `id` since the last [`Self::clear`].
    pub(crate) fn get(&self, id: FacetId) -> Option<V> {
        match self.entries.get(id.index as usize) {
            Some(&(epoch, generation, value))
                if epoch == self.epoch && generation == id.generation =>
            {
                Some(value)
            }
            _ => None,
        }
    }

    pub(crate) fn contains(&self, id: FacetId) -> bool {
        self.get(id).is_some()
    }

    pub(crate) fn insert(&mut self, id: FacetId, value: V) {
        let index = id.index as usize;
        if index >= self.entries.len() {
            self.entries.resize(index + 1, (0, 0, V::default()));
        }
        self.entries[index] = (self.epoch, id.generation, value);
    }
}

/// A reference to a facet of a [`crate::hull::store::FacetStore`] that
/// cannot reach a reused slot.
/// The default id fills unused inline storage, which lies outside the
/// slice a list reads, so it is never read as a facet. It equals the id of
/// the first slot's first entry, so it is no marker for a missing facet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct FacetId {
    index: u32,
    generation: u32,
}

impl FacetId {
    /// The id of the entry in slot `index` while the slot has `generation`.
    pub(crate) fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// The slot index. Stable for the lifetime of the entry.
    pub(crate) fn index(self) -> u32 {
        self.index
    }

    /// The generation the slot had when the entry was inserted.
    pub(crate) fn generation(self) -> u32 {
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_marks_forget_on_clear_and_ignore_stale_ids() {
        let a = FacetId::new(0, 0);
        let b = FacetId::new(1, 0);
        let mut marks: SlotMarks<bool> = SlotMarks::default();
        marks.insert(a, true);
        assert_eq!(marks.get(a), Some(true));
        assert_eq!(marks.get(b), None);
        marks.insert(b, false);
        assert_eq!(marks.get(b), Some(false));
        // A reused slot has a new generation: the stale mark does not match.
        let c = FacetId::new(0, 1);
        assert_eq!(c.index(), a.index());
        assert!(!marks.contains(c));
        assert!(marks.contains(a));
        marks.clear();
        assert!(!marks.contains(a) && !marks.contains(b));
        // Wrapping the epoch starts over with no entry.
        marks.insert(b, true);
        marks.epoch = u32::MAX;
        marks.insert(b, true);
        marks.clear();
        assert!(!marks.contains(b));
        marks.insert(c, true);
        assert!(marks.contains(c) && !marks.contains(b));
    }
}
