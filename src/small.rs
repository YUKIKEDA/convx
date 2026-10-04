//! A short list stored inline up to a fixed length (#120).
//!
//! Construction makes about one simplex per created facet, and each kept
//! its vertices, its neighbors, and its cull frame in separate heap blocks.
//! A walk then followed a pointer per block. Up to `N` items live inside
//! the owner instead; a longer list, for a dimension above the inline
//! range, falls back to a vector. Both forms read as one slice.

use core::ops::{Deref, DerefMut};

/// Up to `N` items inline, more on the heap.
#[derive(Clone)]
pub(crate) enum Small<T: Copy + Default, const N: usize> {
    Inline { len: u8, items: [T; N] },
    Heap(Vec<T>),
}

impl<T: Copy + Default, const N: usize> Small<T, N> {
    /// An empty list.
    pub(crate) fn new() -> Self {
        Self::Inline {
            len: 0,
            items: [T::default(); N],
        }
    }
}

impl<T: Copy + Default, const N: usize> Default for Small<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Default, const N: usize> Deref for Small<T, N> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        match self {
            Self::Inline { len, items } => &items[..usize::from(*len)],
            Self::Heap(items) => items,
        }
    }
}

impl<T: Copy + Default, const N: usize> DerefMut for Small<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        match self {
            Self::Inline { len, items } => &mut items[..usize::from(*len)],
            Self::Heap(items) => items,
        }
    }
}

impl<'a, T: Copy + Default, const N: usize> IntoIterator for &'a Small<T, N> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: Copy + Default, const N: usize> FromIterator<T> for Small<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut items = [T::default(); N];
        let mut len = 0;
        let mut iter = iter.into_iter();
        for item in iter.by_ref() {
            if len == N || len == usize::from(u8::MAX) {
                let mut heap: Vec<T> = items[..len].to_vec();
                heap.push(item);
                heap.extend(iter);
                return Self::Heap(heap);
            }
            items[len] = item;
            len += 1;
        }
        Self::Inline {
            len: len as u8,
            items,
        }
    }
}

impl<T: Copy + Default, const N: usize> From<&[T]> for Small<T, N> {
    fn from(items: &[T]) -> Self {
        items.iter().copied().collect()
    }
}

impl<T: Copy + Default, const N: usize> From<Vec<T>> for Small<T, N> {
    fn from(items: Vec<T>) -> Self {
        if items.len() <= N {
            items.into_iter().collect()
        } else {
            Self::Heap(items)
        }
    }
}

impl<T: Copy + Default + core::fmt::Debug, const N: usize> core::fmt::Debug for Small<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: Copy + Default + PartialEq, const N: usize> PartialEq for Small<T, N> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: Copy + Default + Eq, const N: usize> Eq for Small<T, N> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_lists_stay_inline_and_long_ones_spill_with_the_same_items() {
        for len in 0..=9 {
            let items: Vec<u32> = (0..len).map(|i| 10 * i + 1).collect();
            let small: Small<u32, 4> = items.iter().copied().collect();
            assert_eq!(&*small, &items[..], "len {len}");
            assert_eq!(matches!(small, Small::Inline { .. }), len <= 4, "len {len}");
            let from_vec: Small<u32, 4> = items.clone().into();
            assert_eq!(from_vec, small);
            let mut changed = small.clone();
            if let Some(first) = changed.first_mut() {
                *first = 0;
                assert_ne!(changed, small);
            }
        }
    }
}
