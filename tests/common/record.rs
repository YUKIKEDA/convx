//! Text records of frozen hull results (docs/verification.md).
//!
//! One record per file, one `key value...` line per field:
//!
//! ```text
//! generator xoshiro256starstar-v1
//! family cube
//! dim 3
//! count 100
//! seed 7
//! volume 6.123 1e-12        (or: volume unchecked)
//! vertices 0 4 9
//! coplanar_points
//! interior_points 1 2 3
//! facet 0 4 9
//! delaunay_sites 0 1 2 3 4 9
//! delaunay_simplex 0 1 4 9
//! voronoi_vertex 0 1 4 9
//! voronoi_cell 0 0 3
//! voronoi_interface 0 1
//! ```
//!
//! Facets are listed by ascending vertex set, in lexicographic order. Records
//! of D <= 5 also hold the Delaunay and Voronoi observables (lines starting
//! `delaunay_` and `voronoi_`), and only what the design promises across
//! versions: every Voronoi vertex's `sites`, each cell's vertex list, each
//! interface's sites, the Delaunay sites, and the Delaunay simplices of the
//! vertices with exactly D + 1 sites. The split of a cospherical group into
//! simplices (its diagonals) is not recorded (§7), nor are coordinates
//! (§8).

use convx::{ConvexHull, VoronoiBuilder};

use super::generator::Family;

/// Relative volume tolerance used when a record is frozen.
pub const VOLUME_TOLERANCE: f64 = 1e-12;

/// The volume is written `unchecked` below this fraction of `w^D`, where `w`
/// is the largest side of the bounding box. One rounding of a term at the
/// extent scale, about `f64::EPSILON * w^D`, then exceeds the tolerance, so
/// only the topology is compared (design §10).
pub const NEAR_ZERO_VOLUME: f64 = f64::EPSILON / VOLUME_TOLERANCE;

/// The largest side of the bounding box of `points`.
fn extent(dim: usize, points: &[f64]) -> f64 {
    (0..dim)
        .map(|axis| {
            let (low, high) = points
                .iter()
                .skip(axis)
                .step_by(dim)
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), &x| {
                    (l.min(x), h.max(x))
                });
            high - low
        })
        .fold(0.0, f64::max)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub generator: String,
    pub family: Family,
    pub dim: usize,
    pub count: usize,
    pub seed: u64,
    /// The volume and its relative tolerance, or `None` when unchecked.
    pub volume: Option<(f64, f64)>,
    pub vertices: Vec<u32>,
    pub coplanar_points: Vec<u32>,
    pub interior_points: Vec<u32>,
    pub facets: Vec<Vec<u32>>,
    /// The Delaunay and Voronoi observables, for D <= 5.
    pub delaunay: Option<DelaunayRecord>,
}

/// The Delaunay and Voronoi observables the design promises across versions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DelaunayRecord {
    /// Sites that are vertices of the triangulation (the union of the
    /// Voronoi vertices' sites), ascending.
    pub sites: Vec<u32>,
    /// Ascending vertex lists of the simplices whose Voronoi vertex has
    /// exactly D + 1 sites, in lexicographic order.
    pub simplices: Vec<Vec<u32>>,
    /// Every Voronoi vertex's `sites`, in vertex order.
    pub vertices: Vec<Vec<u32>>,
    /// Each cell as its site followed by its vertex numbers.
    pub cells: Vec<Vec<u32>>,
    /// Interface site pairs, in order.
    pub interfaces: Vec<Vec<u32>>,
}

/// The largest dimension whose records hold Delaunay observables.
pub const DELAUNAY_MAX_DIM: usize = 5;

impl DelaunayRecord {
    /// The observables of the sites `points` of dimension `dim`. A Voronoi
    /// vertex with exactly D + 1 sites is one Delaunay simplex, so the
    /// recorded simplices come from the vertices.
    pub fn of(dim: usize, points: &[f64]) -> Result<Self, String> {
        let v = VoronoiBuilder::new(dim, points)
            .build()
            .map_err(|e| e.to_string())?;
        let mut sites: Vec<u32> = v
            .vertices
            .iter()
            .flat_map(|x| x.sites.iter().copied())
            .collect();
        sites.sort_unstable();
        sites.dedup();
        let mut simplices: Vec<Vec<u32>> = v
            .vertices
            .iter()
            .map(|x| x.sites.clone())
            .filter(|x| x.len() == dim + 1)
            .collect();
        simplices.sort();
        Ok(Self {
            sites,
            simplices,
            vertices: v.vertices.iter().map(|x| x.sites.clone()).collect(),
            cells: v
                .cells
                .iter()
                .map(|c| {
                    core::iter::once(c.site)
                        .chain(c.vertices.iter().copied())
                        .collect()
                })
                .collect(),
            interfaces: v.interfaces.iter().map(|f| f.sites.to_vec()).collect(),
        })
    }

    pub fn to_text(&self) -> String {
        let mut text = String::new();
        text.push_str(&line("delaunay_sites", &self.sites));
        for s in &self.simplices {
            text.push_str(&line("delaunay_simplex", s));
        }
        for s in &self.vertices {
            text.push_str(&line("voronoi_vertex", s));
        }
        for c in &self.cells {
            text.push_str(&line("voronoi_cell", c));
        }
        for f in &self.interfaces {
            text.push_str(&line("voronoi_interface", f));
        }
        text
    }

    /// Every mismatch against a fresh computation on the same points.
    pub fn compare(&self, fresh: &Self) -> Vec<String> {
        let mut errors = Vec::new();
        let mut check = |name: &str, same: bool| {
            if !same {
                errors.push(format!("{name} differ"));
            }
        };
        check("delaunay sites", self.sites == fresh.sites);
        check("delaunay simplices", self.simplices == fresh.simplices);
        check("voronoi vertices", self.vertices == fresh.vertices);
        check("voronoi cells", self.cells == fresh.cells);
        check("voronoi interfaces", self.interfaces == fresh.interfaces);
        errors
    }
}

fn join(values: &[u32]) -> String {
    values
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

fn line(key: &str, values: &[u32]) -> String {
    if values.is_empty() {
        format!("{key}\n")
    } else {
        format!("{key} {}\n", join(values))
    }
}

impl Record {
    /// The record of a hull freshly built from `points`.
    pub fn of(
        generator: &str,
        family: Family,
        dim: usize,
        count: usize,
        seed: u64,
        points: &[f64],
        hull: &ConvexHull,
    ) -> Self {
        let mut facets: Vec<Vec<u32>> = hull.facets.iter().map(|f| f.vertices.clone()).collect();
        facets.sort();
        let volume = hull.volume();
        let near_zero = volume < NEAR_ZERO_VOLUME * extent(dim, points).powi(dim as i32);
        Self {
            generator: generator.to_string(),
            family,
            dim,
            count,
            seed,
            volume: (!near_zero).then_some((volume, VOLUME_TOLERANCE)),
            vertices: hull.vertices.clone(),
            coplanar_points: hull.coplanar_points.clone(),
            interior_points: hull.interior_points.clone(),
            facets,
            delaunay: None,
        }
    }

    pub fn file_name(&self) -> String {
        format!(
            "{}-d{}-n{}-s{}.txt",
            self.family.name(),
            self.dim,
            self.count,
            self.seed
        )
    }

    pub fn to_text(&self) -> String {
        let mut text = String::new();
        text.push_str(&format!("generator {}\n", self.generator));
        text.push_str(&format!("family {}\n", self.family.name()));
        text.push_str(&format!("dim {}\n", self.dim));
        text.push_str(&format!("count {}\n", self.count));
        text.push_str(&format!("seed {}\n", self.seed));
        match self.volume {
            Some((v, tolerance)) => text.push_str(&format!("volume {v:?} {tolerance:?}\n")),
            None => text.push_str("volume unchecked\n"),
        }
        text.push_str(&line("vertices", &self.vertices));
        text.push_str(&line("coplanar_points", &self.coplanar_points));
        text.push_str(&line("interior_points", &self.interior_points));
        for facet in &self.facets {
            text.push_str(&line("facet", facet));
        }
        if let Some(delaunay) = &self.delaunay {
            text.push_str(&delaunay.to_text());
        }
        text
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let mut generator = None;
        let mut family = None;
        let mut dim = None;
        let mut count = None;
        let mut seed = None;
        let mut volume = None;
        let mut vertices = None;
        let mut coplanar_points = None;
        let mut interior_points = None;
        let mut facets = Vec::new();
        let mut delaunay: Option<DelaunayRecord> = None;
        for raw in text.lines() {
            let mut words = raw.split_whitespace();
            let Some(key) = words.next() else {
                continue;
            };
            let rest: Vec<&str> = words.collect();
            let numbers = || -> Result<Vec<u32>, String> {
                rest.iter()
                    .map(|w| w.parse::<u32>().map_err(|e| format!("{raw}: {e}")))
                    .collect()
            };
            let single = || {
                rest.first()
                    .copied()
                    .ok_or_else(|| format!("{raw}: missing value"))
            };
            match key {
                "generator" => generator = Some(single()?.to_string()),
                "family" => {
                    family = Some(
                        Family::from_name(single()?)
                            .ok_or_else(|| format!("unknown family in {raw}"))?,
                    )
                }
                "dim" => dim = Some(single()?.parse().map_err(|e| format!("{raw}: {e}"))?),
                "count" => count = Some(single()?.parse().map_err(|e| format!("{raw}: {e}"))?),
                "seed" => seed = Some(single()?.parse().map_err(|e| format!("{raw}: {e}"))?),
                "volume" => {
                    volume = Some(if single()? == "unchecked" {
                        None
                    } else {
                        let value: f64 = single()?.parse().map_err(|e| format!("{raw}: {e}"))?;
                        let tolerance: f64 = rest
                            .get(1)
                            .ok_or_else(|| format!("{raw}: missing tolerance"))?
                            .parse()
                            .map_err(|e| format!("{raw}: {e}"))?;
                        Some((value, tolerance))
                    });
                }
                "vertices" => vertices = Some(numbers()?),
                "coplanar_points" => coplanar_points = Some(numbers()?),
                "interior_points" => interior_points = Some(numbers()?),
                "facet" => facets.push(numbers()?),
                "delaunay_sites" => {
                    delaunay.get_or_insert_with(Default::default).sites = numbers()?
                }
                "delaunay_simplex" => delaunay
                    .get_or_insert_with(Default::default)
                    .simplices
                    .push(numbers()?),
                "voronoi_vertex" => delaunay
                    .get_or_insert_with(Default::default)
                    .vertices
                    .push(numbers()?),
                "voronoi_cell" => delaunay
                    .get_or_insert_with(Default::default)
                    .cells
                    .push(numbers()?),
                "voronoi_interface" => delaunay
                    .get_or_insert_with(Default::default)
                    .interfaces
                    .push(numbers()?),
                other => return Err(format!("unknown key {other}")),
            }
        }
        Ok(Self {
            generator: generator.ok_or("missing generator")?,
            family: family.ok_or("missing family")?,
            dim: dim.ok_or("missing dim")?,
            count: count.ok_or("missing count")?,
            seed: seed.ok_or("missing seed")?,
            volume: volume.ok_or("missing volume")?,
            vertices: vertices.ok_or("missing vertices")?,
            coplanar_points: coplanar_points.ok_or("missing coplanar_points")?,
            interior_points: interior_points.ok_or("missing interior_points")?,
            facets,
            delaunay,
        })
    }

    /// Compares a rebuilt hull with this record; returns every mismatch.
    pub fn compare(&self, hull: &ConvexHull) -> Vec<String> {
        let mut errors = Vec::new();
        if hull.vertices != self.vertices {
            errors.push("vertices differ".to_string());
        }
        if hull.coplanar_points != self.coplanar_points {
            errors.push("coplanar_points differ".to_string());
        }
        if hull.interior_points != self.interior_points {
            errors.push("interior_points differ".to_string());
        }
        let mut facets: Vec<Vec<u32>> = hull.facets.iter().map(|f| f.vertices.clone()).collect();
        facets.sort();
        if facets != self.facets {
            errors.push("facets differ".to_string());
        }
        if let Some((expected, tolerance)) = self.volume {
            let got = hull.volume();
            let scale = expected.abs().max(got.abs());
            let relative = if scale == 0.0 {
                0.0
            } else {
                (expected - got).abs() / scale
            };
            if relative.is_nan() || relative > tolerance {
                errors.push(format!(
                    "volume {got} differs from {expected} by {relative}"
                ));
            }
        }
        errors
    }
}
