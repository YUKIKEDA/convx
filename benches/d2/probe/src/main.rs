//! Times convx's Delaunay build of D = 2 beside spade's bulk load on the same
//! points, uniform in the unit square, in one process.
//!
//! usage: probe <label> <n> [<n> ...]
//! Per round: convx, convx again (the A/A control), spade, taking turns.
//! With the feature `marked`, each convx build's stage times are kept too.
use std::hint::black_box;
use std::time::Instant;

use spade::{DelaunayTriangulation, Point2, Triangulation};

fn points(n: usize, seed: u64) -> Vec<f64> {
    let mut s = seed;
    (0..2 * n)
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

fn convx_build(p: &[f64]) -> usize {
    let d = convx::DelaunayBuilder::new(2, p).build().expect("a triangulation");
    d.simplices().len()
}

fn spade_build(p: &[f64]) -> usize {
    let v: Vec<Point2<f64>> = p.chunks_exact(2).map(|q| Point2::new(q[0], q[1])).collect();
    let t = DelaunayTriangulation::<Point2<f64>>::bulk_load(v).expect("a triangulation");
    t.num_inner_faces()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let label = &args[1];
    for n in args[2..].iter().map(|a| a.parse::<usize>().unwrap()) {
        let p = points(n, 0x9E37_79B9_7F4A_7C15);
        let rounds = (3_000_000 / n).clamp(7, 20_000);
        assert_eq!(convx_build(&p), spade_build(&p), "the same number of triangles");
        for _ in 0..rounds.min(50) {
            black_box(convx_build(&p));
            black_box(spade_build(&p));
        }
        let (mut a, mut b, mut s) = (Vec::new(), Vec::new(), Vec::new());
        #[cfg(feature = "marked")]
        let mut stages: Vec<Vec<f64>> = vec![Vec::new(); convx::stages_take().len()];
        for _ in 0..rounds {
            let t = Instant::now();
            black_box(convx_build(black_box(&p)));
            a.push(t.elapsed().as_secs_f64());
            #[cfg(feature = "marked")]
            for (i, x) in convx::stages_take().into_iter().enumerate() {
                stages[i].push(x);
            }
            let t = Instant::now();
            black_box(convx_build(black_box(&p)));
            b.push(t.elapsed().as_secs_f64());
            #[cfg(feature = "marked")]
            convx::stages_take();
            let t = Instant::now();
            black_box(spade_build(black_box(&p)));
            s.push(t.elapsed().as_secs_f64());
        }
        let (ma, mb, ms) = (median(&mut a), median(&mut b), median(&mut s));
        print!(
            "{{\"label\":\"{label}\",\"n\":{n},\"rounds\":{rounds},\"convx\":{ma:e},\"control\":{mb:e},\"spade\":{ms:e}"
        );
        #[cfg(feature = "marked")]
        {
            let m: Vec<String> = stages.iter_mut().map(|v| format!("{:e}", median(v))).collect();
            print!(",\"stages\":[{}]", m.join(","));
        }
        println!("}}");
    }
}
