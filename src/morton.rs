//! Morton codes: the bits of grid coordinates interleaved, so that sorting by
//! the code visits nearby points together. Delaunay orders its insertion by
//! them (BRIO, #423); the hull of D = 3 orders its copy of the points by them
//! (ADR 0009, #488).

/// The low 32 bits of `x` spread to the even bit positions.
pub(crate) fn spread2(x: u64) -> u64 {
    let mut x = x & 0xffff_ffff;
    x = (x | x << 16) & 0x0000_ffff_0000_ffff;
    x = (x | x << 8) & 0x00ff_00ff_00ff_00ff;
    x = (x | x << 4) & 0x0f0f_0f0f_0f0f_0f0f;
    x = (x | x << 2) & 0x3333_3333_3333_3333;
    (x | x << 1) & 0x5555_5555_5555_5555
}

/// The low 21 bits of `x` spread to every third bit position.
pub(crate) fn spread3(x: u64) -> u64 {
    let mut x = x & 0x1f_ffff;
    x = (x | x << 32) & 0x001f_0000_0000_ffff;
    x = (x | x << 16) & 0x001f_0000_ff00_00ff;
    x = (x | x << 8) & 0x100f_00f0_0f00_f00f;
    x = (x | x << 4) & 0x10c3_0c30_c30c_30c3;
    (x | x << 2) & 0x1249_2492_4924_9249
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_masks_spread_every_bit() {
        // Each bit alone and every bit together, against the loop's
        // positions: bit `b` at `2b` and at `3b`.
        for b in 0..32 {
            assert_eq!(spread2(1 << b), 1 << (2 * b), "bit {b}");
        }
        for b in 0..21 {
            assert_eq!(spread3(1 << b), 1 << (3 * b), "bit {b}");
        }
        assert_eq!(spread2(u64::from(u32::MAX)), 0x5555_5555_5555_5555);
        assert_eq!(spread3(0x1f_ffff), 0x1249_2492_4924_9249);
    }
}
