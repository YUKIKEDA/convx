//! Pairing of equal ridges (#120).
//!
//! In a closed simplicial boundary every ridge lies in exactly two
//! simplices. Construction and classification both find those pairs from
//! the sorted vertex lists of the ridges, packed in one buffer, without a
//! hash map keyed by allocated lists.

/// The pairs of equal keys among `count` keys of equal width packed in
/// `keys`, each key occurring exactly twice. An open-addressing table
/// indexed by `fingerprint` holds each key until its twin arrives; keys are
/// compared only when their probes meet. A paired slot becomes a tombstone
/// so that later probes still pass it. Linear in `count`.
pub(crate) fn pair_equal_keys(
    keys: &[u32],
    count: usize,
    fingerprint: impl Fn(&[u32]) -> u64,
) -> Vec<(usize, usize)> {
    const EMPTY: u32 = u32::MAX;
    const PAIRED: u32 = u32::MAX - 1;
    let width = keys.len().checked_div(count).unwrap_or(0);
    let key = |i: usize| &keys[i * width..(i + 1) * width];
    let size = (2 * count).next_power_of_two().max(2);
    let mask = size - 1;
    let mut table = vec![EMPTY; size];
    let mut pairs = Vec::with_capacity(count / 2);
    for i in 0..count {
        let mut slot = fingerprint(key(i)) as usize & mask;
        loop {
            match table[slot] {
                EMPTY => {
                    table[slot] = i as u32;
                    break;
                }
                PAIRED => {}
                other if key(other as usize) == key(i) => {
                    pairs.push((other as usize, i));
                    table[slot] = PAIRED;
                    break;
                }
                _ => {}
            }
            slot = (slot + 1) & mask;
        }
    }
    debug_assert!(
        table.iter().all(|&e| e == EMPTY || e == PAIRED),
        "every key occurs exactly twice"
    );
    pairs
}

/// A hash of a sorted vertex list, for the table of [`pair_equal_keys`].
pub(crate) fn fingerprint(key: &[u32]) -> u64 {
    key.iter().fold(0x9e37_79b9_7f4a_7c15, |h, &v| {
        (h ^ u64::from(v))
            .wrapping_mul(0x0100_0000_01b3)
            .rotate_left(23)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pairs `pair_equal_keys` returns, as sorted index pairs, sorted.
    fn sorted_pairs(keys: &[u32], count: usize, constant: bool) -> Vec<(usize, usize)> {
        let mut pairs: Vec<(usize, usize)> = if constant {
            pair_equal_keys(keys, count, |_| 7)
        } else {
            pair_equal_keys(keys, count, fingerprint)
        };
        for p in &mut pairs {
            *p = (p.0.min(p.1), p.0.max(p.1));
        }
        pairs.sort_unstable();
        pairs
    }

    #[test]
    fn equal_keys_pair_even_when_every_fingerprint_collides() {
        // Width 2: keys 0..6 are [1,2] [3,4] [1,2] [5,6] [3,4] [5,6].
        let keys = [1, 2, 3, 4, 1, 2, 5, 6, 3, 4, 5, 6];
        let expected = vec![(0, 2), (1, 4), (3, 5)];
        assert_eq!(sorted_pairs(&keys, 6, false), expected);
        // One probe chain for all six: keys are compared along it, and a
        // paired slot does not end a later probe.
        assert_eq!(sorted_pairs(&keys, 6, true), expected);
        // [3,4] waits past the slot [1,2] vacated: its twin must probe
        // through the tombstone.
        let keys = [1, 2, 3, 4, 1, 2, 3, 4];
        assert_eq!(sorted_pairs(&keys, 4, true), vec![(0, 2), (1, 3)]);
        // Width 0 (D = 2): the two empty keys are equal.
        assert_eq!(sorted_pairs(&[], 2, false), vec![(0, 1)]);
        assert_eq!(sorted_pairs(&[], 2, true), vec![(0, 1)]);
    }
}
