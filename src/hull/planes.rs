//! Planes a point was found on during construction (design §3).
//!
//! Every simplex carries a [`PlaneNumber`]. Simplices with one number have
//! one supporting plane and one outer side, so a point has one sign against
//! all of them. [`OnPlane`] holds the (point, number) pairs whose exact sign
//! was zero; a later sign of such a pair is zero without being evaluated.

use core::num::NonZeroU32;

/// The number of a supporting plane with its outer side. Two simplices in
/// one plane may carry different numbers; two simplices with one number are
/// never in different planes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlaneNumber(NonZeroU32);

impl PlaneNumber {
    /// The number as an index; below [`PlaneNumbers::end`] of its source.
    pub(crate) fn index(self) -> usize {
        self.0.get() as usize
    }
}

/// The source of the plane numbers of one hull.
pub(crate) struct PlaneNumbers {
    /// The next number, or `None` when every number is taken.
    next: Option<NonZeroU32>,
}

impl Default for PlaneNumbers {
    fn default() -> Self {
        Self {
            next: Some(NonZeroU32::MIN),
        }
    }
}

impl PlaneNumbers {
    /// A number no simplex carries yet, or `None` when every number is
    /// taken. A simplex without a number records nothing and is tested as
    /// if nothing were recorded, so running out costs time, not
    /// correctness.
    pub(crate) fn fresh(&mut self) -> Option<PlaneNumber> {
        let number = self.next?;
        self.next = number.checked_add(1);
        Some(PlaneNumber(number))
    }

    /// One more than the largest [`PlaneNumber::index`] issued.
    pub(crate) fn end(&self) -> usize {
        match self.next {
            Some(next) => next.get() as usize,
            None => u32::MAX as usize + 1,
        }
    }
}

/// The set of (point, plane number) pairs found at distance zero. Open
/// addressing with linear probing, at most half full, so a probe always
/// ends. An empty set owns no storage, and a query of it reads none: a hull
/// in general position never pays for it.
#[derive(Default)]
pub(crate) struct OnPlane {
    slots: Vec<u64>,
    len: usize,
}

impl OnPlane {
    /// Marks an empty slot. It is the key of point `u32::MAX`, which numbers
    /// no point (design §3).
    const EMPTY: u64 = u64::MAX;

    fn key(point: u32, plane: PlaneNumber) -> u64 {
        debug_assert_ne!(point, u32::MAX, "no point is numbered u32::MAX");
        u64::from(point) << 32 | u64::from(plane.0.get())
    }

    /// The first slot probed for `key` among `slots` slots, a power of two:
    /// the top bits of a multiplicative hash.
    fn home(key: u64, slots: usize) -> usize {
        (key.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> (64 - slots.trailing_zeros())) as usize
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn contains(&self, point: u32, plane: PlaneNumber) -> bool {
        if self.len == 0 {
            return false;
        }
        let key = Self::key(point, plane);
        let mask = self.slots.len() - 1;
        let mut slot = Self::home(key, self.slots.len());
        loop {
            match self.slots[slot] {
                Self::EMPTY => return false,
                other if other == key => return true,
                _ => slot = (slot + 1) & mask,
            }
        }
    }

    pub(crate) fn insert(&mut self, point: u32, plane: PlaneNumber) {
        if (self.len + 1) * 2 > self.slots.len() {
            self.grow();
        }
        if Self::place(&mut self.slots, Self::key(point, plane)) {
            self.len += 1;
        }
    }

    /// Writes `key` into `slots` unless it is there; true when it was not.
    fn place(slots: &mut [u64], key: u64) -> bool {
        let mask = slots.len() - 1;
        let mut slot = Self::home(key, slots.len());
        loop {
            match slots[slot] {
                Self::EMPTY => {
                    slots[slot] = key;
                    return true;
                }
                other if other == key => return false,
                _ => slot = (slot + 1) & mask,
            }
        }
    }

    /// Doubles the slots and places every pair again.
    fn grow(&mut self) {
        let size = (self.slots.len() * 2).max(16);
        let old = core::mem::replace(&mut self.slots, vec![Self::EMPTY; size]);
        for key in old {
            if key != Self::EMPTY {
                Self::place(&mut self.slots, key);
            }
        }
    }

    /// Every pair as (point, plane index), in no particular order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (u32, usize)> + '_ {
        self.slots
            .iter()
            .filter(|&&key| key != Self::EMPTY)
            .map(|&key| ((key >> 32) as u32, (key & 0xffff_ffff) as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn numbers_are_distinct_and_below_the_end() {
        let mut numbers = PlaneNumbers::default();
        let a = numbers.fresh().unwrap();
        let b = numbers.fresh().unwrap();
        assert_ne!(a, b);
        assert!(a.index() < numbers.end() && b.index() < numbers.end());
        assert_eq!(numbers.end(), 3);
    }

    #[test]
    fn the_last_number_is_issued_once_and_then_none() {
        let mut numbers = PlaneNumbers {
            next: Some(NonZeroU32::MAX),
        };
        assert_eq!(
            numbers.fresh().map(PlaneNumber::index),
            Some(u32::MAX as usize)
        );
        assert!(numbers.fresh().is_none());
        assert_eq!(numbers.end(), u32::MAX as usize + 1);
    }

    /// The reference is `HashSet`: after each insertion, every pair of a
    /// small grid of points and planes is in the set exactly when the
    /// reference holds it, through several growths.
    #[test]
    fn the_set_agrees_with_a_hash_set() {
        let mut numbers = PlaneNumbers::default();
        let planes: Vec<PlaneNumber> = (0..40).filter_map(|_| numbers.fresh()).collect();
        let mut set = OnPlane::default();
        let mut reference: HashSet<(u32, usize)> = HashSet::new();
        assert!(set.is_empty());
        assert!(!set.contains(0, planes[0]));
        let mut state = 7u64;
        for step in 0..600 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            // Point 0 and large points, and repeats of earlier pairs.
            let point = [0, 1, 5, 1 << 30, 1 << 31][(state >> 33) as usize % 5]
                + ((state >> 40) as u32 % 7) * u32::from(step % 3 != 0);
            let plane = planes[(state >> 50) as usize % planes.len()];
            set.insert(point, plane);
            reference.insert((point, plane.index()));
            assert!(set.contains(point, plane));
        }
        assert!(!set.is_empty());
        let probes = [
            0,
            1,
            2,
            5,
            6,
            11,
            1 << 30,
            (1 << 30) + 6,
            1 << 31,
            (1 << 31) + 3,
        ];
        for point in probes.into_iter().chain([u32::MAX - 1]) {
            for &plane in &planes {
                assert_eq!(
                    set.contains(point, plane),
                    reference.contains(&(point, plane.index())),
                    "point {point} plane {}",
                    plane.index()
                );
            }
        }
        let listed: HashSet<(u32, usize)> = set.iter().collect();
        assert_eq!(listed, reference);
        assert_eq!(
            set.iter().count(),
            reference.len(),
            "no pair is listed twice"
        );
    }
}
