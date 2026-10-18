//! Times convx's Delaunay build beside spade's bulk load on the same points,
//! uniform in the unit cube, in one process.
//!
//! usage: probe <label> <n> [<n> ...]
//! Per round: convx, convx again (the A/A control), spade, taking turns.
//! With the feature `marked`, each convx build's stage times are kept too.
//!
//! `PROBE_DIM` sets the dimension (2 when unset); spade, which is of D = 2
//! alone, is timed for D = 2 only, and reads 0 otherwise. `PROBE_POINTS`
//! names a directory: the points of each size are written there in Qhull's
//! format, `d<D>-n<n>.txt`, for another library's program to time.
use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

use spade::{DelaunayTriangulation, Point2, Triangulation};

fn points(dim: usize, n: usize, seed: u64) -> Vec<f64> {
    let mut s = seed;
    (0..dim * n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s >> 11) as f64 / (1u64 << 53) as f64
        })
        .collect()
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let m = v.len() / 2;
    if v.len() % 2 == 1 { v[m] } else { (v[m - 1] + v[m]) / 2.0 }
}

fn convx_build(dim: usize, p: &[f64]) -> usize {
    let d = convx::DelaunayBuilder::new(dim, p).build().expect("a triangulation");
    d.simplices().len()
}

fn spade_build(p: &[f64]) -> usize {
    let v: Vec<Point2<f64>> = p.chunks_exact(2).map(|q| Point2::new(q[0], q[1])).collect();
    let t = DelaunayTriangulation::<Point2<f64>>::bulk_load(v).expect("a triangulation");
    t.num_inner_faces()
}

fn write_points(dir: &str, dim: usize, n: usize, p: &[f64]) {
    let mut f = std::io::BufWriter::new(
        std::fs::File::create(format!("{dir}/d{dim}-n{n}.txt")).expect("a points file"),
    );
    writeln!(f, "{dim}\n{n}").unwrap();
    for q in p.chunks_exact(dim) {
        let row: Vec<String> = q.iter().map(|x| format!("{x:?}")).collect();
        writeln!(f, "{}", row.join(" ")).unwrap();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let label = &args[1];
    let dim: usize = std::env::var("PROBE_DIM").map_or(2, |d| d.parse().expect("a dimension"));
    let spade = dim == 2;
    for n in args[2..].iter().map(|a| a.parse::<usize>().unwrap()) {
        let p = points(dim, n, 0x9E37_79B9_7F4A_7C15);
        if let Ok(dir) = std::env::var("PROBE_POINTS") {
            write_points(&dir, dim, n, &p);
        }
        let rounds = (3_000_000 / (n * (dim - 1) * (dim - 1))).clamp(7, 20_000);
        if spade {
            assert_eq!(convx_build(dim, &p), spade_build(&p), "the same number of triangles");
        }
        for _ in 0..rounds.min(50) {
            black_box(convx_build(dim, &p));
            if spade {
                black_box(spade_build(&p));
            }
        }
        let (mut a, mut b, mut s) = (Vec::new(), Vec::new(), Vec::new());
        #[cfg(feature = "marked")]
        let mut stages: Vec<Vec<f64>> = vec![Vec::new(); convx::stages_take().len()];
        for _ in 0..rounds {
            let t = Instant::now();
            black_box(convx_build(dim, black_box(&p)));
            a.push(t.elapsed().as_secs_f64());
            #[cfg(feature = "marked")]
            for (i, x) in convx::stages_take().into_iter().enumerate() {
                stages[i].push(x);
            }
            let t = Instant::now();
            black_box(convx_build(dim, black_box(&p)));
            b.push(t.elapsed().as_secs_f64());
            #[cfg(feature = "marked")]
            convx::stages_take();
            if spade {
                let t = Instant::now();
                black_box(spade_build(black_box(&p)));
                s.push(t.elapsed().as_secs_f64());
            } else {
                s.push(0.0);
            }
        }
        let (ma, mb, ms) = (median(&mut a), median(&mut b), median(&mut s));
        print!(
            "{{\"label\":\"{label}\",\"dim\":{dim},\"n\":{n},\"rounds\":{rounds},\"convx\":{ma:e},\"control\":{mb:e},\"spade\":{ms:e}"
        );
        #[cfg(feature = "marked")]
        {
            let m: Vec<String> = stages.iter_mut().map(|v| format!("{:e}", median(v))).collect();
            print!(",\"stages\":[{}]", m.join(","));
        }
        println!("}}");
    }
}
