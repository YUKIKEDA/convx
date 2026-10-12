// Times CGAL hull and Delaunay on a Qhull-format point file (convx #215).
// usage: cgal_time <hull|delaunay> <file> <reps>
// Timed section: the hull's construction, with the D = 2 hull's vertices as
// input indices (#456); for Delaunay, the construction from
// (point, input index) pairs and the writing of each finite cell's vertices and
// neighbors into flat arrays (#455). Delaunay lines also print the time of the
// construction from the points alone (#215). Reading the file is outside it.
// g++ -std=c++17 -O3 -DNDEBUG -g -DCGAL_EIGEN3_ENABLED -I/usr/include/eigen3 -o cgal_time cgal_time.cpp -lgmp -lmpfr
#include <CGAL/Exact_predicates_inexact_constructions_kernel.h>
#include <CGAL/convex_hull_2.h>
#include <CGAL/Convex_hull_traits_adapter_2.h>
#include <CGAL/convex_hull_3.h>
#include <CGAL/Surface_mesh.h>
#include <CGAL/Delaunay_triangulation_2.h>
#include <CGAL/Delaunay_triangulation_3.h>
#include <CGAL/Epick_d.h>
#include <CGAL/Triangulation.h>
#include <CGAL/Delaunay_triangulation.h>
#include <CGAL/Triangulation_vertex_base_with_info_2.h>
#include <CGAL/Triangulation_vertex_base_with_info_3.h>
#include <CGAL/Triangulation_face_base_with_info_2.h>
#include <CGAL/Triangulation_cell_base_with_info_3.h>
#include <CGAL/spatial_sort.h>
#include <CGAL/Spatial_sort_traits_adapter_d.h>
#include <CGAL/property_map.h>
#include <chrono>
#include <cstdio>
#include <fstream>
#include <iostream>
#include <set>
#include <string>
#include <vector>

using K = CGAL::Exact_predicates_inexact_constructions_kernel;
using Clock = std::chrono::steady_clock;

static std::vector<double> coords;
static int dim = 0;
static long n = 0;

static double secs(Clock::time_point a, Clock::time_point b) {
  return std::chrono::duration<double>(b - a).count();
}

static void report(double t, long verts, long cells) {
  std::printf("%.6f %ld %ld\n", t, verts, cells);
  std::fflush(stdout);
}

static void report_delaunay(double t, long verts, long cells, double points) {
  std::printf("%.6f %ld %ld %.6f\n", t, verts, cells, points);
  std::fflush(stdout);
}

static void hull2(int reps) {
  // The hull's vertices as input indices (#456), and from the points alone
  // (#215), whose time is the line's fourth field.
  using Adapter = CGAL::Convex_hull_traits_adapter_2<K, CGAL::Pointer_property_map<K::Point_2>::type>;
  std::vector<K::Point_2> pts;
  for (long i = 0; i < n; ++i) pts.emplace_back(coords[2 * i], coords[2 * i + 1]);
  std::vector<K::Point_2> out;
  out.reserve(n);
  std::vector<std::size_t> indices((size_t)n), hull;
  hull.reserve(n);
  for (int r = 0; r < reps; ++r) {
    out.clear();
    auto a = Clock::now();
    CGAL::convex_hull_2(pts.begin(), pts.end(), std::back_inserter(out));
    auto b = Clock::now();
    hull.clear();
    for (long i = 0; i < n; ++i) indices[(size_t)i] = (std::size_t)i;
    CGAL::convex_hull_2(indices.begin(), indices.end(), std::back_inserter(hull), Adapter(CGAL::make_property_map(pts)));
    auto c = Clock::now();
    if (hull.size() != out.size()) { std::cerr << "count\n"; std::exit(1); }
    report_delaunay(secs(b, c), (long)hull.size(), (long)hull.size(), secs(a, b));
  }
}

static void hull3(int reps) {
  std::vector<K::Point_3> pts;
  for (long i = 0; i < n; ++i) pts.emplace_back(coords[3 * i], coords[3 * i + 1], coords[3 * i + 2]);
  for (int r = 0; r < reps; ++r) {
    CGAL::Surface_mesh<K::Point_3> sm;
    auto a = Clock::now();
    CGAL::convex_hull_3(pts.begin(), pts.end(), sm);
    auto b = Clock::now();
    report(secs(a, b), (long)sm.number_of_vertices(), (long)sm.number_of_faces());
  }
}

static void del2(int reps) {
  using Vb = CGAL::Triangulation_vertex_base_with_info_2<unsigned, K>;
  using Fb = CGAL::Triangulation_face_base_with_info_2<unsigned, K>;
  using Dt = CGAL::Delaunay_triangulation_2<K, CGAL::Triangulation_data_structure_2<Vb, Fb>>;
  std::vector<K::Point_2> pts;
  std::vector<std::pair<K::Point_2, unsigned>> pairs;
  for (long i = 0; i < n; ++i) {
    pts.emplace_back(coords[2 * i], coords[2 * i + 1]);
    pairs.emplace_back(pts.back(), (unsigned)i);
  }
  for (int r = 0; r < reps; ++r) {
    auto a = Clock::now();
    CGAL::Delaunay_triangulation_2<K> plain(pts.begin(), pts.end());
    auto b = Clock::now();
    Dt dt(pairs.begin(), pairs.end());
    unsigned cells = 0;
    for (auto f = dt.finite_faces_begin(); f != dt.finite_faces_end(); ++f) f->info() = cells++;
    std::vector<unsigned> vertices, neighbors;
    vertices.reserve(3 * (size_t)cells);
    neighbors.reserve(3 * (size_t)cells);
    for (auto f = dt.finite_faces_begin(); f != dt.finite_faces_end(); ++f)
      for (int k = 0; k < 3; ++k) {
        vertices.push_back(f->vertex(k)->info());
        auto g = f->neighbor(k);
        neighbors.push_back(dt.is_infinite(g) ? ~0u : g->info());
      }
    auto c = Clock::now();
    if (plain.number_of_faces() != cells || vertices.size() != 3 * (size_t)cells) { std::cerr << "count\n"; std::exit(1); }
    report_delaunay(secs(b, c), (long)dt.number_of_vertices(), (long)cells, secs(a, b));
  }
}

static void del3(int reps) {
  using Vb = CGAL::Triangulation_vertex_base_with_info_3<unsigned, K>;
  using Cb = CGAL::Triangulation_cell_base_with_info_3<unsigned, K, CGAL::Delaunay_triangulation_cell_base_3<K>>;
  using Dt = CGAL::Delaunay_triangulation_3<K, CGAL::Triangulation_data_structure_3<Vb, Cb>>;
  std::vector<K::Point_3> pts;
  std::vector<std::pair<K::Point_3, unsigned>> pairs;
  for (long i = 0; i < n; ++i) {
    pts.emplace_back(coords[3 * i], coords[3 * i + 1], coords[3 * i + 2]);
    pairs.emplace_back(pts.back(), (unsigned)i);
  }
  for (int r = 0; r < reps; ++r) {
    auto a = Clock::now();
    CGAL::Delaunay_triangulation_3<K> plain(pts.begin(), pts.end());
    auto b = Clock::now();
    Dt dt(pairs.begin(), pairs.end());
    unsigned cells = 0;
    for (auto c = dt.finite_cells_begin(); c != dt.finite_cells_end(); ++c) c->info() = cells++;
    std::vector<unsigned> vertices, neighbors;
    vertices.reserve(4 * (size_t)cells);
    neighbors.reserve(4 * (size_t)cells);
    for (auto c = dt.finite_cells_begin(); c != dt.finite_cells_end(); ++c)
      for (int k = 0; k < 4; ++k) {
        vertices.push_back(c->vertex(k)->info());
        auto g = c->neighbor(k);
        neighbors.push_back(dt.is_infinite(g) ? ~0u : g->info());
      }
    auto e = Clock::now();
    if (plain.number_of_finite_cells() != cells) { std::cerr << "count\n"; std::exit(1); }
    report_delaunay(secs(b, e), (long)dt.number_of_vertices(), (long)cells, secs(a, b));
  }
}

template <int D> static void hulld(int reps) {
  using Kd = CGAL::Epick_d<CGAL::Dimension_tag<D>>;
  using T = CGAL::Triangulation<Kd>;
  using P = typename Kd::Point_d;
  std::vector<P> pts;
  for (long i = 0; i < n; ++i) pts.emplace_back(&coords[D * i], &coords[D * i] + D);
  for (int r = 0; r < reps; ++r) {
    T t(D);
    auto a = Clock::now();
    t.insert(pts.begin(), pts.end());
    auto b = Clock::now();
    std::vector<typename T::Full_cell_handle> inf;
    t.incident_full_cells(t.infinite_vertex(), std::back_inserter(inf));
    std::set<typename T::Vertex_handle> vs;
    for (auto c : inf)
      for (int k = 0; k <= D; ++k)
        if (!t.is_infinite(c->vertex(k))) vs.insert(c->vertex(k));
    report(secs(a, b), (long)vs.size(), (long)inf.size());
  }
}

template <int D> static void deld(int reps) {
  using Kd = CGAL::Epick_d<CGAL::Dimension_tag<D>>;
  using P = typename Kd::Point_d;
  using Tds = CGAL::Triangulation_data_structure<CGAL::Dimension_tag<D>,
      CGAL::Triangulation_vertex<Kd, unsigned>, CGAL::Triangulation_full_cell<Kd, unsigned>>;
  using T = CGAL::Delaunay_triangulation<Kd, Tds>;
  using Plain = CGAL::Delaunay_triangulation<Kd>;
  std::vector<P> pts;
  for (long i = 0; i < n; ++i) pts.emplace_back(&coords[D * i], &coords[D * i] + D);
  for (int r = 0; r < reps; ++r) {
    Plain plain(D);
    auto a = Clock::now();
    plain.insert(pts.begin(), pts.end());
    auto b = Clock::now();
    // The range insert of CGAL::Delaunay_triangulation: a spatial sort, then
    // one insertion per point with the last vertex as the hint. Here the
    // sort is of indices, so each vertex is given its index.
    T t(D);
    std::vector<std::size_t> order((size_t)n);
    for (long i = 0; i < n; ++i) order[(size_t)i] = (std::size_t)i;
    using Map = typename CGAL::Pointer_property_map<P>::type;
    CGAL::Spatial_sort_traits_adapter_d<Kd, Map> traits(CGAL::make_property_map(pts));
    CGAL::spatial_sort(order.begin(), order.end(), traits);
    typename T::Full_cell_handle hint;
    for (std::size_t i : order) {
      auto v = t.insert(pts[i], hint);
      v->data() = (unsigned)i;
      hint = v->full_cell();
    }
    unsigned cells = 0;
    for (auto c = t.finite_full_cells_begin(); c != t.finite_full_cells_end(); ++c) c->data() = cells++;
    std::vector<unsigned> vertices, neighbors;
    vertices.reserve((D + 1) * (size_t)cells);
    neighbors.reserve((D + 1) * (size_t)cells);
    for (auto c = t.finite_full_cells_begin(); c != t.finite_full_cells_end(); ++c)
      for (int k = 0; k <= D; ++k) {
        vertices.push_back(c->vertex(k)->data());
        auto g = c->neighbor(k);
        neighbors.push_back(t.is_infinite(g) ? ~0u : g->data());
      }
    auto e = Clock::now();
    if ((long)plain.number_of_finite_full_cells() != (long)cells) { std::cerr << "count\n"; std::exit(1); }
    report_delaunay(secs(b, e), (long)t.number_of_vertices(), (long)cells, secs(a, b));
  }
}

int main(int argc, char** argv) {
  if (argc != 4) { std::cerr << "usage: cgal_time <hull|delaunay> <file> <reps>\n"; return 2; }
  std::string mode = argv[1];
  int reps = std::stoi(argv[3]);
  std::ifstream in(argv[2]);
  in >> dim >> n;
  coords.resize((size_t)dim * n);
  for (auto& x : coords) in >> x;
  if (!in) { std::cerr << "cannot read " << argv[2] << "\n"; return 1; }
  bool h = mode == "hull";
  switch (dim) {
    case 2: h ? hull2(reps) : del2(reps); break;
    case 3: h ? hull3(reps) : del3(reps); break;
    case 4: h ? hulld<4>(reps) : deld<4>(reps); break;
    case 5: h ? hulld<5>(reps) : deld<5>(reps); break;
    case 6: h ? hulld<6>(reps) : deld<6>(reps); break;
    default: std::cerr << "dim " << dim << " not built\n"; return 1;
  }
}
