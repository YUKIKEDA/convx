//! The simplicial facets of a hull under construction, in flat rows (#253).
//!
//! A facet is a slot. What a visibility walk reads lies in two rows per
//! slot: the vertices followed by the neighbors (as slots), and the working
//! plane (unit normal, then the slope and floor of its certified threshold).
//! The rows are sized to the hull's dimension, so a facet of D = 3 is 24 bytes
//! of indices and 40 bytes of plane, where a record sized for D = 8 took 288
//! (#209). What a walk does not read is kept apart: the outward sign, the
//! range of the facet's outside points in one pool shared by every facet,
//! the candidate of that set, and the slot's state.
//!
//! A slot is reused after its facet is removed. Its generation is bumped on
//! removal, so a [`FacetId`] held from before never reaches the new facet.
//! A live facet's neighbors are live, so neighbor rows hold bare slots.

use crate::arena::FacetId;
use crate::cull::CullPlane;
use crate::predicates::Sign;

/// What a facet's plane row holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Plane {
    /// No working normal could be certified.
    None,
    /// A certified working normal, but no certified cull threshold.
    Normal,
    /// A certified working normal and its cull threshold.
    Cull,
}

/// The facets of a hull under construction.
pub(crate) struct FacetStore {
    dim: usize,
    /// Neighbors per facet: D, or none for D = 1, whose facets are single
    /// endpoints (design §4).
    links: usize,
    /// Per slot, `dim + links` entries: the vertices, then the neighbor
    /// slots; `neighbors[i]` is across the ridge opposite `vertices[i]`.
    rows: Vec<u32>,
    /// Per slot, `dim + 2` entries: the working unit normal, then the slope
    /// and the floor of the cull threshold.
    planes: Vec<f64>,
    kinds: Vec<Plane>,
    /// The plane number of each slot (design §3): facets with one number
    /// have one supporting plane and one outer side.
    numbers: Vec<u32>,
    /// The next new plane number.
    issued: u32,
    /// The sign of `orient(vertices, q)` for a point `q` outside.
    outward: Vec<Sign>,
    generations: Vec<u32>,
    live: Vec<bool>,
    /// The facet's outside points: `pool[start..start + len]`.
    outside: Vec<(u32, u32)>,
    /// The candidate of the outside set: the point, `u32::MAX` for none,
    /// and its working distance, NaN for none.
    candidates: Vec<(u32, f64)>,
    /// Outside points of every facet, each facet's in one run. A removed
    /// facet's run stays until [`Self::compact`] drops it.
    pool: Vec<u32>,
    /// Entries of `pool` that belong to no live facet.
    garbage: usize,
    free: Vec<u32>,
    len: usize,
}

/// No point: a facet with an empty outside set has no candidate.
const NO_POINT: u32 = u32::MAX;

/// The pool is compacted when at least this many of its entries are
/// garbage, and they are at least half of it.
const COMPACT_FROM: usize = 1 << 16;

/// An index that no longer fits in `u32` is an exhaustion of the index
/// space; like an ordinary allocation failure, it aborts (design §3).
fn abort_if_full(slots: usize) {
    if slots >= u32::MAX as usize {
        std::process::abort();
    }
}

impl FacetStore {
    pub(crate) fn new(dim: usize) -> Self {
        Self {
            dim,
            links: if dim == 1 { 0 } else { dim },
            rows: Vec::new(),
            planes: Vec::new(),
            kinds: Vec::new(),
            numbers: Vec::new(),
            issued: 0,
            outward: Vec::new(),
            generations: Vec::new(),
            live: Vec::new(),
            outside: Vec::new(),
            candidates: Vec::new(),
            pool: Vec::new(),
            garbage: 0,
            free: Vec::new(),
            len: 0,
        }
    }

    /// Number of live facets.
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// One past the largest slot.
    pub(crate) fn slots(&self) -> usize {
        self.live.len()
    }

    /// A slot for a new facet with no neighbors, plane, or outside points
    /// yet, the outward sign `outward`, and the plane number `number`, or a
    /// new one. A freed slot is reused last in, first out.
    pub(crate) fn alloc(&mut self, vertices: &[u32], outward: Sign, number: Option<u32>) -> u32 {
        let d = self.dim;
        debug_assert_eq!(vertices.len(), d, "a facet has D vertices");
        let slot = match self.free.pop() {
            Some(slot) => slot,
            None => {
                let slot = self.live.len();
                abort_if_full(slot);
                self.rows.resize(self.rows.len() + d + self.links, 0);
                self.planes.resize(self.planes.len() + d + 2, 0.0);
                self.kinds.push(Plane::None);
                self.numbers.push(0);
                self.outward.push(Sign::Positive);
                self.generations.push(0);
                self.live.push(false);
                self.outside.push((0, 0));
                self.candidates.push((NO_POINT, f64::NAN));
                slot as u32
            }
        };
        let s = slot as usize;
        let row = s * (d + self.links);
        self.rows[row..row + d].copy_from_slice(vertices);
        self.kinds[s] = Plane::None;
        self.numbers[s] = number.unwrap_or_else(|| {
            let new = self.issued;
            abort_if_full(new as usize);
            self.issued += 1;
            new
        });
        self.outward[s] = outward;
        self.live[s] = true;
        self.outside[s] = (0, 0);
        self.candidates[s] = (NO_POINT, f64::NAN);
        self.len += 1;
        slot
    }

    /// Removes the facet in `slot`. Its id stops resolving.
    pub(crate) fn remove(&mut self, slot: u32) {
        let s = slot as usize;
        debug_assert!(self.live[s], "a removed facet is live");
        self.live[s] = false;
        self.generations[s] = self.generations[s].wrapping_add(1);
        self.garbage += self.outside[s].1 as usize;
        self.outside[s] = (0, 0);
        self.free.push(slot);
        self.len -= 1;
    }

    /// The id of the live facet in `slot`.
    pub(crate) fn id(&self, slot: u32) -> FacetId {
        debug_assert!(self.live[slot as usize], "an id names a live facet");
        FacetId::new(slot, self.generations[slot as usize])
    }

    /// The slot of `id`, while its facet is live.
    pub(crate) fn slot_of(&self, id: FacetId) -> Option<u32> {
        let s = id.index() as usize;
        (s < self.live.len() && self.live[s] && self.generations[s] == id.generation())
            .then_some(id.index())
    }

    pub(crate) fn get(&self, id: FacetId) -> Option<Facet<'_>> {
        self.slot_of(id).map(|slot| self.facet(slot))
    }

    /// The facet in `slot`, which must be live.
    pub(crate) fn facet(&self, slot: u32) -> Facet<'_> {
        debug_assert!(self.live[slot as usize], "a facet view reads a live slot");
        Facet { store: self, slot }
    }

    /// The live facets with their ids, in slot order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (FacetId, Facet<'_>)> + '_ {
        (0..self.live.len() as u32)
            .filter(|&slot| self.live[slot as usize])
            .map(|slot| (self.id(slot), self.facet(slot)))
    }

    /// The neighbor slots of the facet in `slot`, to be written.
    pub(crate) fn neighbors_mut(&mut self, slot: u32) -> &mut [u32] {
        let d = self.dim;
        let start = slot as usize * (d + self.links) + d;
        &mut self.rows[start..start + self.links]
    }

    /// Stores the working plane of the facet in `slot`: `normal` when one
    /// was certified, and the cull threshold when that was certified too.
    pub(crate) fn set_plane(
        &mut self,
        slot: u32,
        normal: Option<&[f64]>,
        cull: Option<(f64, f64)>,
    ) {
        let d = self.dim;
        let s = slot as usize;
        let row = &mut self.planes[s * (d + 2)..(s + 1) * (d + 2)];
        self.kinds[s] = match (normal, cull) {
            (Some(normal), cull) => {
                row[..d].copy_from_slice(normal);
                match cull {
                    Some((slope, floor)) => {
                        row[d] = slope;
                        row[d + 1] = floor;
                        Plane::Cull
                    }
                    None => Plane::Normal,
                }
            }
            (None, _) => Plane::None,
        };
    }

    /// Appends `points` as the outside set of the facet in `slot`, which
    /// has none yet, with the candidate `farthest` of that set.
    pub(crate) fn set_outside(
        &mut self,
        slot: u32,
        points: &[u32],
        farthest: Option<(u32, Option<f64>)>,
    ) {
        let s = slot as usize;
        debug_assert_eq!(self.outside[s].1, 0, "an outside set is set once");
        if points.is_empty() {
            debug_assert!(farthest.is_none(), "an empty set has no candidate");
            return;
        }
        if self.garbage >= COMPACT_FROM && 2 * self.garbage >= self.pool.len() {
            self.compact();
        }
        let start = self.pool.len();
        abort_if_full(start + points.len());
        self.pool.extend_from_slice(points);
        self.outside[s] = (start as u32, points.len() as u32);
        self.candidates[s] = match farthest {
            Some((point, distance)) => (point, distance.unwrap_or(f64::NAN)),
            None => (NO_POINT, f64::NAN),
        };
    }

    /// Drops the runs of removed facets from the pool.
    fn compact(&mut self) {
        let mut pool = Vec::with_capacity(self.pool.len() - self.garbage);
        for s in 0..self.live.len() {
            let (start, len) = self.outside[s];
            if self.live[s] && len > 0 {
                let at = pool.len() as u32;
                pool.extend_from_slice(&self.pool[start as usize..(start + len) as usize]);
                self.outside[s] = (at, len);
            }
        }
        self.pool = pool;
        self.garbage = 0;
    }
}

/// A live facet of a [`FacetStore`].
#[derive(Clone, Copy)]
pub(crate) struct Facet<'a> {
    store: &'a FacetStore,
    slot: u32,
}

impl<'a> Facet<'a> {
    #[cfg(test)]
    pub(crate) fn slot(self) -> u32 {
        self.slot
    }

    pub(crate) fn id(self) -> FacetId {
        self.store.id(self.slot)
    }

    pub(crate) fn vertices(self) -> &'a [u32] {
        let d = self.store.dim;
        let start = self.slot as usize * (d + self.store.links);
        &self.store.rows[start..start + d]
    }

    /// The neighbor slots: `neighbor_slots()[i]` is across the ridge
    /// opposite `vertices()[i]`.
    pub(crate) fn neighbor_slots(self) -> &'a [u32] {
        let d = self.store.dim;
        let links = self.store.links;
        let start = self.slot as usize * (d + links) + d;
        &self.store.rows[start..start + links]
    }

    /// The neighbor ids, in the order of [`Self::neighbor_slots`].
    pub(crate) fn neighbors(self) -> impl ExactSizeIterator<Item = FacetId> + 'a {
        let store = self.store;
        self.neighbor_slots()
            .iter()
            .map(move |&slot| store.id(slot))
    }

    /// The sign of `orient(vertices, q)` for a point `q` outside.
    pub(crate) fn outward(self) -> Sign {
        self.store.outward[self.slot as usize]
    }

    /// The plane number: see [`FacetStore::alloc`].
    pub(crate) fn number(self) -> u32 {
        self.store.numbers[self.slot as usize]
    }

    fn plane_row(self) -> &'a [f64] {
        let d = self.store.dim;
        let start = self.slot as usize * (d + 2);
        &self.store.planes[start..start + d + 2]
    }

    /// The working unit normal, when one could be certified.
    pub(crate) fn normal(self) -> Option<&'a [f64]> {
        match self.store.kinds[self.slot as usize] {
            Plane::None => None,
            Plane::Normal | Plane::Cull => Some(&self.plane_row()[..self.store.dim]),
        }
    }

    /// The cull plane, when one could be certified.
    pub(crate) fn cull(self) -> Option<CullPlane<&'a [f64]>> {
        let d = self.store.dim;
        match self.store.kinds[self.slot as usize] {
            Plane::Cull => {
                let row = self.plane_row();
                Some(CullPlane::from_parts(&row[..d], row[d], row[d + 1]))
            }
            Plane::None | Plane::Normal => None,
        }
    }

    /// Points assigned to this facet that are strictly outside it.
    pub(crate) fn outside(self) -> &'a [u32] {
        let (start, len) = self.store.outside[self.slot as usize];
        &self.store.pool[start as usize..(start + len) as usize]
    }

    /// The candidate of the outside set and its working distance: see
    /// `simplicial::farthest`.
    pub(crate) fn farthest(self) -> Option<(u32, Option<f64>)> {
        let (point, distance) = self.store.candidates[self.slot as usize];
        (point != NO_POINT).then(|| (point, (!distance.is_nan()).then_some(distance)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_slots_are_reused_and_old_ids_stop_resolving() {
        let mut store = FacetStore::new(2);
        let a = store.alloc(&[0, 1], Sign::Positive, None);
        let b = store.alloc(&[1, 2], Sign::Negative, None);
        let old = store.id(a);
        store.remove(a);
        assert!(store.get(old).is_none());
        let c = store.alloc(&[2, 3], Sign::Positive, None);
        assert_eq!(c, a, "the freed slot is reused");
        assert!(store.get(old).is_none(), "a stale id misses the new facet");
        assert_eq!(
            store.get(store.id(c)).map(|f| f.vertices()),
            Some(&[2, 3][..])
        );
        assert_eq!(store.facet(b).outward(), Sign::Negative);
        assert_eq!(store.len(), 2);
        let order: Vec<u32> = store.iter().map(|(_, f)| f.slot()).collect();
        assert_eq!(order, vec![a, b], "iteration is in slot order");
    }

    #[test]
    fn planes_and_outside_sets_read_back() {
        let mut store = FacetStore::new(2);
        let a = store.alloc(&[0, 1], Sign::Positive, None);
        assert!(store.facet(a).normal().is_none() && store.facet(a).cull().is_none());
        store.set_plane(a, Some(&[0.6, 0.8]), None);
        assert_eq!(store.facet(a).normal(), Some(&[0.6, 0.8][..]));
        assert!(store.facet(a).cull().is_none());
        store.set_plane(a, Some(&[0.0, 1.0]), Some((1e-15, 1e-300)));
        let cull = store.facet(a).cull().expect("a cull plane");
        assert_eq!(cull.normal(), &[0.0, 1.0]);
        store.set_outside(a, &[5, 7], Some((7, None)));
        assert_eq!(store.facet(a).outside(), &[5, 7]);
        assert_eq!(store.facet(a).farthest(), Some((7, None)));
        let b = store.alloc(&[1, 2], Sign::Positive, None);
        store.set_outside(b, &[9], Some((9, Some(2.5))));
        assert_eq!(store.facet(b).farthest(), Some((9, Some(2.5))));
        assert_eq!(store.facet(a).outside(), &[5, 7]);
    }

    #[test]
    fn compaction_keeps_every_live_outside_set() {
        let mut store = FacetStore::new(1);
        let mut kept = Vec::new();
        for round in 0..4u32 {
            // Many removed runs, so the pool is compacted along the way.
            for i in 0..COMPACT_FROM as u32 / 2 {
                let slot = store.alloc(&[i], Sign::Positive, None);
                store.set_outside(slot, &[i, round], Some((i, None)));
                store.remove(slot);
            }
            let slot = store.alloc(&[round], Sign::Positive, None);
            store.set_outside(slot, &[round, 100 + round], Some((round, None)));
            kept.push(slot);
        }
        assert!(
            store.pool.len() < 2 * COMPACT_FROM + 8,
            "the pool was compacted"
        );
        for (round, &slot) in kept.iter().enumerate() {
            let r = round as u32;
            assert_eq!(store.facet(slot).outside(), &[r, 100 + r]);
        }
    }
}
