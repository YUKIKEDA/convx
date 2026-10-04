//! Generational arena for simplices during construction.
//!
//! Slots live in fixed-size chunks. A chunk is reserved once at full size
//! and never grows, so a growing arena never moves an existing entry. A
//! [`FacetId`] carries the slot index and the generation the slot had when
//! the entry was inserted. Removing an entry bumps the generation, so a stale
//! id never reaches a reused slot. Freed slots are reused last in, first out.
//!
//! This arena is single-threaded. Parallel workers plan with local numbers,
//! and the commit inserts on one thread in ascending input index (§6), so
//! no arena insert or remove is ever concurrent. Lock-free allocation was
//! left out after measurement (#26): on the P2-7 `cube` sets measured
//! there, every arena insert and remove of a build took under 1% of it.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// A hash map keyed by arena ids (or tuples of ids and point numbers).
pub(crate) type IdMap<K, V> = HashMap<K, V, BuildHasherDefault<IdHasher>>;

/// Hashes each word with one rotate, xor, and multiply.
///
/// Arena ids are made by the arena, never chosen by the caller, so a keyed
/// hash (the standard `RandomState`) buys no protection here and cost about
/// a fifth of a build (#85). Keys built from caller-chosen data keep the
/// standard hasher.
#[derive(Clone, Copy, Default)]
pub(crate) struct IdHasher(u64);

impl IdHasher {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.add(u64::from(b));
        }
    }

    fn write_u32(&mut self, n: u32) {
        self.add(u64::from(n));
    }

    fn write_u64(&mut self, n: u64) {
        self.add(n);
    }

    fn write_usize(&mut self, n: usize) {
        self.add(n as u64);
    }
}

/// A value per arena id, for one search at a time, without hashing (#120).
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

/// Number of slots in one chunk. Not tuned before measurement.
const CHUNK_SIZE: usize = 1024;

/// A reference to an arena entry that cannot reach a reused slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct FacetId {
    index: u32,
    generation: u32,
}

impl FacetId {
    /// The slot index. Stable for the lifetime of the entry.
    #[cfg(test)]
    pub(crate) fn index(self) -> u32 {
        self.index
    }
}

/// The arena already holds `u32::MAX` slots. `u32::MAX` itself is reserved
/// as the missing index, so no further slot can be numbered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ArenaFull;

enum Slot<T> {
    Occupied {
        generation: u32,
        value: T,
    },
    Vacant {
        generation: u32,
        next_free: Option<u32>,
    },
}

pub(crate) struct Arena<T> {
    chunks: Vec<Vec<Slot<T>>>,
    /// Number of slots ever created.
    slots: u32,
    /// Head of the LIFO list of vacant slots.
    free_head: Option<u32>,
    len: usize,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    pub(crate) fn new() -> Self {
        Self {
            chunks: Vec::new(),
            slots: 0,
            free_head: None,
            len: 0,
        }
    }

    /// Number of live entries.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn locate(index: u32) -> (usize, usize) {
        let index = index as usize;
        (index / CHUNK_SIZE, index % CHUNK_SIZE)
    }

    fn slot(&self, index: u32) -> Option<&Slot<T>> {
        let (chunk, offset) = Self::locate(index);
        self.chunks.get(chunk)?.get(offset)
    }

    fn slot_mut(&mut self, index: u32) -> Option<&mut Slot<T>> {
        let (chunk, offset) = Self::locate(index);
        self.chunks.get_mut(chunk)?.get_mut(offset)
    }

    /// Stores `value` and returns its id.
    pub(crate) fn insert(&mut self, value: T) -> Result<FacetId, ArenaFull> {
        if let Some(index) = self.free_head {
            if let Some(slot) = self.slot_mut(index) {
                if let Slot::Vacant {
                    generation,
                    next_free,
                } = *slot
                {
                    *slot = Slot::Occupied { generation, value };
                    self.free_head = next_free;
                    self.len += 1;
                    return Ok(FacetId { index, generation });
                }
            }
            debug_assert!(false, "free list points at an occupied slot");
        }
        if self.slots == u32::MAX {
            return Err(ArenaFull);
        }
        let index = self.slots;
        let (chunk, _) = Self::locate(index);
        if chunk == self.chunks.len() {
            self.chunks.push(Vec::with_capacity(CHUNK_SIZE));
        }
        let generation = 0;
        self.chunks[chunk].push(Slot::Occupied { generation, value });
        self.slots += 1;
        self.len += 1;
        Ok(FacetId { index, generation })
    }

    /// Removes the entry and returns it. A stale or unknown id returns `None`.
    pub(crate) fn remove(&mut self, id: FacetId) -> Option<T> {
        let free_head = self.free_head;
        let slot = self.slot_mut(id.index)?;
        match slot {
            Slot::Occupied { generation, .. } if *generation == id.generation => {}
            _ => return None,
        }
        let vacant = Slot::Vacant {
            generation: id.generation.wrapping_add(1),
            next_free: free_head,
        };
        let Slot::Occupied { value, .. } = core::mem::replace(slot, vacant) else {
            return None;
        };
        self.free_head = Some(id.index);
        self.len -= 1;
        Some(value)
    }

    pub(crate) fn get(&self, id: FacetId) -> Option<&T> {
        match self.slot(id.index)? {
            Slot::Occupied { generation, value } if *generation == id.generation => Some(value),
            _ => None,
        }
    }

    pub(crate) fn get_mut(&mut self, id: FacetId) -> Option<&mut T> {
        match self.slot_mut(id.index)? {
            Slot::Occupied { generation, value } if *generation == id.generation => Some(value),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn contains(&self, id: FacetId) -> bool {
        self.get(id).is_some()
    }

    /// Live entries in slot order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (FacetId, &T)> + '_ {
        self.chunks
            .iter()
            .flatten()
            .enumerate()
            .filter_map(|(index, slot)| match slot {
                Slot::Occupied { generation, value } => Some((
                    FacetId {
                        index: index as u32,
                        generation: *generation,
                    },
                    value,
                )),
                Slot::Vacant { .. } => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_marks_forget_on_clear_and_ignore_stale_ids() {
        let mut arena = Arena::new();
        let a = arena.insert(1).unwrap();
        let b = arena.insert(2).unwrap();
        let mut marks: SlotMarks<bool> = SlotMarks::default();
        marks.insert(a, true);
        assert_eq!(marks.get(a), Some(true));
        assert_eq!(marks.get(b), None);
        marks.insert(b, false);
        assert_eq!(marks.get(b), Some(false));
        // A reused slot has a new generation: the stale mark does not match.
        arena.remove(a);
        let c = arena.insert(3).unwrap();
        assert_eq!(c.index, a.index);
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

    #[test]
    fn insert_get_remove() {
        let mut arena = Arena::new();
        assert!(arena.is_empty());
        let a = arena.insert("a").unwrap();
        let b = arena.insert("b").unwrap();
        assert_eq!(arena.len(), 2);
        assert_eq!(arena.get(a), Some(&"a"));
        assert_eq!(arena.remove(a), Some("a"));
        assert_eq!(arena.get(a), None);
        assert_eq!(arena.remove(a), None);
        assert!(arena.contains(b));
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn stale_id_does_not_reach_a_reused_slot() {
        let mut arena = Arena::new();
        let old = arena.insert(1).unwrap();
        arena.remove(old);
        let new = arena.insert(2).unwrap();
        assert_eq!(new.index(), old.index());
        assert_ne!(new, old);
        assert_eq!(arena.get(old), None);
        assert_eq!(arena.get_mut(old), None);
        assert_eq!(arena.remove(old), None);
        assert_eq!(arena.get(new), Some(&2));
    }

    #[test]
    fn freed_slots_are_reused_last_in_first_out() {
        let mut arena = Arena::new();
        let ids: Vec<_> = (0..4).map(|i| arena.insert(i).unwrap()).collect();
        arena.remove(ids[1]);
        arena.remove(ids[3]);
        assert_eq!(arena.insert(10).unwrap().index(), 3);
        assert_eq!(arena.insert(11).unwrap().index(), 1);
        assert_eq!(arena.insert(12).unwrap().index(), 4);
    }

    #[test]
    fn crossing_a_chunk_boundary_keeps_entries() {
        let mut arena = Arena::new();
        let ids: Vec<_> = (0..CHUNK_SIZE * 2 + 5)
            .map(|i| arena.insert(i).unwrap())
            .collect();
        let first = arena.get(ids[0]).map(|v| v as *const usize);
        arena.insert(usize::MAX).unwrap();
        // Entries in a full chunk do not move when a later chunk is added.
        assert_eq!(arena.get(ids[0]).map(|v| v as *const usize), first);
        for (i, &id) in ids.iter().enumerate() {
            assert_eq!(arena.get(id), Some(&i));
        }
    }

    #[test]
    fn iteration_skips_removed_entries_in_slot_order() {
        let mut arena = Arena::new();
        let ids: Vec<_> = (0..6).map(|i| arena.insert(i).unwrap()).collect();
        arena.remove(ids[0]);
        arena.remove(ids[4]);
        let values: Vec<_> = arena.iter().map(|(_, &v)| v).collect();
        assert_eq!(values, vec![1, 2, 3, 5]);
        for (id, &v) in arena.iter() {
            assert_eq!(id, ids[v]);
        }
    }

    #[test]
    fn get_mut_updates_in_place() {
        let mut arena = Arena::new();
        let id = arena.insert(vec![1]).unwrap();
        arena.get_mut(id).unwrap().push(2);
        assert_eq!(arena.get(id), Some(&vec![1, 2]));
    }
}
