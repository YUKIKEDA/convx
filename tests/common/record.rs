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
//! ```
//!
//! Facets are listed by ascending vertex set, in lexicographic order.

use convx::ConvexHull;

use super::generator::Family;

/// Relative volume tolerance used when a record is frozen.
pub const VOLUME_TOLERANCE: f64 = 1e-12;

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
    /// The record of a freshly built hull.
    pub fn of(
        generator: &str,
        family: Family,
        dim: usize,
        count: usize,
        seed: u64,
        hull: &ConvexHull,
    ) -> Self {
        let mut facets: Vec<Vec<u32>> = hull.facets.iter().map(|f| f.vertices.clone()).collect();
        facets.sort();
        Self {
            generator: generator.to_string(),
            family,
            dim,
            count,
            seed,
            volume: Some((hull.volume(), VOLUME_TOLERANCE)),
            vertices: hull.vertices.clone(),
            coplanar_points: hull.coplanar_points.clone(),
            interior_points: hull.interior_points.clone(),
            facets,
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
