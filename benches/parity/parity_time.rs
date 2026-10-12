use std::time::Instant;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (mode, file, reps) = (&a[1], &a[2], a[3].parse::<usize>().unwrap());
    let text = std::fs::read_to_string(file).unwrap();
    let mut it = text.split_ascii_whitespace();
    let dim: usize = it.next().unwrap().parse().unwrap();
    let n: usize = it.next().unwrap().parse().unwrap();
    let pts: Vec<f64> = it.map(|s| s.parse().unwrap()).collect();
    assert_eq!(pts.len(), dim * n);
    for _ in 0..reps {
        if mode == "hull" {
            let t = Instant::now();
            let h = convx::ConvexHullBuilder::new(dim, &pts).build().unwrap();
            let s = t.elapsed().as_secs_f64();
            let t = Instant::now();
            let planes = h.planes().unwrap().len();
            let p = t.elapsed().as_secs_f64();
            assert_eq!(planes, h.facets().len());
            eprintln!("build {s:.6} {} {} {p:.6}", h.vertices().len(), h.facets().len());
        } else {
            let t = Instant::now();
            let d = convx::DelaunayBuilder::new(dim, &pts).build().unwrap();
            let s = t.elapsed().as_secs_f64();
            eprintln!("build {s:.6} {} {} 0", d.representative().len(), d.simplices().len());
        }
    }
}
