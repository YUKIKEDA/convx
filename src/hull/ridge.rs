//! Pairing of equal ridges (#120).
//!
//! In a closed simplicial boundary every ridge lies in exactly two
//! simplices. Construction and classification both find those pairs from
//! the sorted vertex lists of the ridges, packed in one buffer, without a
//! hash map keyed by allocated lists. A Delaunay triangulation has a
//! border, the boundary of the site hull, whose faces lie in one simplex;
//! it pairs its faces the same way and leaves those single.

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
    let (pairs, single) = pair_keys(keys, count, fingerprint);
    debug_assert!(single == 0, "every key occurs an even number of times");
    pairs
}

/// [`pair_equal_keys`] for a boundary with a border: each key occurs once
/// or twice, and the pairs are those that occur twice.
pub(crate) fn pair_equal_keys_with_border(
    keys: &[u32],
    count: usize,
    fingerprint: impl Fn(&[u32]) -> u64,
) -> Vec<(usize, usize)> {
    pair_keys(keys, count, fingerprint).0
}

/// The pairs of equal keys, and the number of keys left unpaired. Debug
/// builds check that no key occurs more than twice.
fn pair_keys(
    keys: &[u32],
    count: usize,
    fingerprint: impl Fn(&[u32]) -> u64,
) -> (Vec<(usize, usize)>, usize) {
    // Indices are below `count`, which a slice length keeps far below
    // these two markers.
    const EMPTY: usize = usize::MAX;
    const PAIRED: usize = usize::MAX - 1;
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
                    table[slot] = i;
                    break;
                }
                PAIRED => {}
                other if key(other) == key(i) => {
                    pairs.push((other, i));
                    table[slot] = PAIRED;
                    break;
                }
                _ => {}
            }
            slot = (slot + 1) & mask;
        }
    }
    let single = table.iter().filter(|&&e| e != EMPTY && e != PAIRED).count();
    // A key occurring three or four times would pair once and stay single,
    // or pair twice, through two slots; each key occurs at most twice
    // exactly when the keys of the pairs and of the singles are distinct.
    #[cfg(debug_assertions)]
    {
        let mut seen: Vec<&[u32]> = pairs
            .iter()
            .map(|&(a, _)| key(a))
            .chain(
                table
                    .iter()
                    .filter(|&&e| e != EMPTY && e != PAIRED)
                    .map(|&e| key(e)),
            )
            .collect();
        seen.sort_unstable();
        debug_assert!(
            seen.windows(2).all(|w| w[0] != w[1]),
            "a key occurs more than twice"
        );
    }
    (pairs, single)
}

/// A hash of a sorted vertex list, for the table of [`pair_equal_keys`].
///
/// The table takes the low bits. After the fold, those bits depend almost
/// linearly on the low bits of the last vertices, so the faces of nearby
/// vertices would share a few slots (#170). The finalizer of murmur3
/// (`fmix64`) makes every bit of the result depend on every bit of the fold.
pub(crate) fn fingerprint(key: &[u32]) -> u64 {
    let h = key.iter().fold(0x9e37_79b9_7f4a_7c15, |h, &v| {
        (h ^ u64::from(v))
            .wrapping_mul(0x0100_0000_01b3)
            .rotate_left(23)
    });
    let h = (h ^ (h >> 33)).wrapping_mul(0xff51_afd7_ed55_8ccd);
    let h = (h ^ (h >> 33)).wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
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

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "a key occurs more than twice")]
    fn a_key_occurring_four_times_fails_the_debug_check() {
        // [1,2] four times: the table pairs it twice, through two slots.
        let keys = [1, 2, 3, 4, 1, 2, 1, 2, 3, 4, 1, 2];
        sorted_pairs(&keys, 6, false);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "every key occurs an even number of times")]
    fn a_single_key_fails_the_closed_debug_check() {
        pair_equal_keys(&[1, 2, 3, 4, 1, 2], 3, fingerprint);
    }

    #[test]
    fn nearby_two_vertex_keys_spread_over_the_table() {
        // The faces of a D = 2 Delaunay triangulation: edges between nearby
        // sites. 600,000 keys, masked as `pair_keys` masks them. Thrown
        // into 2^21 slots at random, the largest bucket holds about 8; the
        // fold alone filled 2,166 slots, up to 1,100 keys in one (#170).
        let keys: Vec<[u32; 2]> = (0..2000)
            .flat_map(|i| (1..=300).map(move |k| [i, i + k]))
            .collect();
        let mask = (2 * keys.len()).next_power_of_two() - 1;
        let mut buckets = vec![0_u32; mask + 1];
        for key in &keys {
            buckets[fingerprint(key) as usize & mask] += 1;
        }
        let largest = buckets.iter().copied().max().unwrap_or(0);
        let used = buckets.iter().filter(|&&c| c > 0).count();
        assert!(largest <= 12, "largest bucket {largest}");
        assert!(used >= keys.len() * 4 / 5, "{used} slots used");
    }

    #[test]
    fn a_border_leaves_single_keys_unpaired() {
        // Width 2: [1,2] [3,4] [1,2] [5,6] [3,4]; [5,6] is on the border.
        let keys = [1, 2, 3, 4, 1, 2, 5, 6, 3, 4];
        for constant in [false, true] {
            let mut pairs = if constant {
                pair_equal_keys_with_border(&keys, 5, |_| 7)
            } else {
                pair_equal_keys_with_border(&keys, 5, fingerprint)
            };
            pairs.sort_unstable();
            assert_eq!(
                pairs,
                vec![(0, 2), (1, 4)],
                "constant fingerprint {constant}"
            );
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "a key occurs more than twice")]
    fn a_key_occurring_three_times_fails_the_border_debug_check() {
        let keys = [1, 2, 1, 2, 1, 2];
        pair_equal_keys_with_border(&keys, 3, fingerprint);
    }
}
