//! Deterministic point families for simulations and fixtures.
//!
//! The generator is xoshiro256** seeded through SplitMix64. Changing anything
//! here changes the points behind every record that names this generator, so
//! such a change bumps [`GENERATOR`] and the records are reviewed together
//! (docs/verification.md, Change the generator).

/// The identity written into every record.
pub const GENERATOR: &str = "xoshiro256starstar-v1";

pub struct Rng {
    state: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut s = seed;
        let mut split = || {
            s = s.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        Self {
            state: [split(), split(), split(), split()],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.state[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.state[1] << 17;
        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);
        result
    }

    /// Uniform in [-1, 1), a multiple of 2^-52.
    pub fn symmetric(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1_u64 << 52) as f64) - 1.0
    }

    /// Uniform integer in `0..bound`.
    pub fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }
}

/// The point families.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// Uniform in the cube [-1, 1)^D.
    Cube,
    /// Uniform directions scaled to unit length, then rounded: near the
    /// sphere but almost never exactly cospherical.
    Sphere,
    /// Integer points in [0, 3]^D drawn with replacement: exact
    /// coplanarities and duplicates.
    Grid,
    /// Eight centers with offsets of a few 2^-30: near-duplicates and some
    /// exact duplicates.
    Cluster,
    /// Integer points in [-2, 2]^D drawn with replacement: many cospherical
    /// sets for Delaunay and Voronoi.
    Lattice,
    /// Integer points with `|p|^2 = R` for a fixed `R` per dimension, drawn
    /// with replacement: every site on one sphere, exactly.
    OnSphere,
    /// `OnSphere` points, each coordinate moved by -1, 0, or +1 times 2^-8:
    /// near-cospherical, decided by the exact lift.
    NearSphere,
    /// A `Cube` point with one coordinate, drawn uniformly, set to -1 or +1:
    /// uniform on the surface of [-1, 1]^D, every point on a facet's plane.
    CubeSurf,
}

impl Family {
    pub const ALL: [Self; 8] = [
        Self::Cube,
        Self::Sphere,
        Self::Grid,
        Self::Cluster,
        Self::Lattice,
        Self::OnSphere,
        Self::NearSphere,
        Self::CubeSurf,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Cube => "cube",
            Self::Sphere => "sphere",
            Self::Grid => "grid",
            Self::Cluster => "cluster",
            Self::Lattice => "lattice",
            Self::OnSphere => "onsphere",
            Self::NearSphere => "nearsphere",
            Self::CubeSurf => "cubesurf",
        }
    }

    /// Whether every coordinate is an integer, so the exact `i128` oracles
    /// apply directly.
    pub fn is_integer(self) -> bool {
        matches!(self, Self::Grid | Self::Lattice | Self::OnSphere)
    }

    /// The integer points with `|p|^2 = R` in dimension `dim`, ascending.
    fn sphere_points(dim: usize) -> Vec<Vec<i64>> {
        let radius2: i64 = match dim {
            1 => 4,
            2 => 25,
            3 => 9,
            _ => 4,
        };
        let bound = (radius2 as f64).sqrt() as i64;
        let mut out = Vec::new();
        let mut current = vec![-bound; dim];
        loop {
            if current.iter().map(|x| x * x).sum::<i64>() == radius2 {
                out.push(current.clone());
            }
            let Some(a) = (0..dim).rev().find(|&a| current[a] < bound) else {
                return out;
            };
            current[a] += 1;
            for c in &mut current[a + 1..] {
                *c = -bound;
            }
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }

    /// `count` points of dimension `dim`, row-major.
    pub fn points(self, dim: usize, count: usize, seed: u64) -> Vec<f64> {
        let mut rng = Rng::new(seed);
        match self {
            Self::Cube => (0..dim * count).map(|_| rng.symmetric()).collect(),
            Self::Sphere => (0..count)
                .flat_map(|_| {
                    let v: Vec<f64> = (0..dim).map(|_| rng.symmetric()).collect();
                    let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
                    let norm = if norm == 0.0 { 1.0 } else { norm };
                    v.into_iter().map(move |x| x / norm)
                })
                .collect(),
            Self::Grid => (0..dim * count).map(|_| rng.below(4) as f64).collect(),
            Self::Cluster => {
                let centers: Vec<f64> = (0..8 * dim).map(|_| rng.symmetric()).collect();
                let step = 1.0 / (1_u64 << 30) as f64;
                let mut points = Vec::with_capacity(dim * count);
                for _ in 0..count {
                    let c = rng.below(8) as usize;
                    for a in 0..dim {
                        points.push(centers[c * dim + a] + rng.below(4) as f64 * step);
                    }
                }
                points
            }
            Self::Lattice => (0..dim * count)
                .map(|_| rng.below(5) as f64 - 2.0)
                .collect(),
            Self::OnSphere | Self::NearSphere => {
                let sphere = Self::sphere_points(dim);
                let step = 1.0 / 256.0;
                let mut points = Vec::with_capacity(dim * count);
                for _ in 0..count {
                    let p = &sphere[rng.below(sphere.len() as u64) as usize];
                    for &x in p {
                        let moved = if self == Self::NearSphere {
                            (rng.below(3) as f64 - 1.0) * step
                        } else {
                            0.0
                        };
                        points.push(x as f64 + moved);
                    }
                }
                points
            }
            Self::CubeSurf => {
                let mut points = Vec::with_capacity(dim * count);
                for _ in 0..count {
                    let start = points.len();
                    points.extend((0..dim).map(|_| rng.symmetric()));
                    let axis = rng.below(dim as u64) as usize;
                    points[start + axis] = if rng.below(2) == 0 { -1.0 } else { 1.0 };
                }
                points
            }
        }
    }
}
