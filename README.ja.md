[English](https://github.com/YUKIKEDA/convx/blob/main/README.md) | 日本語

# convx

任意の次元の凸包、Delaunay 三角形分割、Voronoi 図を計算する、純 Rust のクレートである。トポロジーに関わる判定は、すべて述語の厳密な符号で行う。

- **厳密である。** 点がファセットのどちら側にあるか、点が同一平面上・同一球面上にあるかは、入力座標の行列式の符号で決める。浮動小数点演算で符号を確定できないときは、厳密に評価する。許容誤差はなく、入力を摂動させることもない。
- **任意の次元を扱う。** すべての `D >= 1` を一つの実装で扱い、`D = 2` と `D = 3` には専用の実装を用意している。
- **退化した入力もそのまま扱う。** 重複点、ファセットの上の点、同一円周上・同一球面上のサイト、格子を、一般の位置にある点と同じ規則で扱う。結果には、入力の各点が頂点・境界上の点・内部点・重複点のどれになったかが含まれる。
- このクレートに **`unsafe` はない。**

```toml
[dependencies]
convx = "0.1"
```

## 最初に呼ぶ前に

- **点は `f64` の行優先のスライス一つで渡す。** 次元が `dim` のとき、点 `i` は `points[i * dim..(i + 1) * dim]` である。`StaticConvexHull`、`StaticDelaunay`、`StaticVoronoi` は代わりに `&[[f64; D]]` を受け取る（`1 <= D <= 8`）。
- **点は入力での添字（`u32`）で表す。** 重複点は、それと等しい点のうち最小の添字で表す。
- **ファセットは論理ファセットである。** 境界の単体のうち、厳密に一つの超平面の上にあるものをまとめて一つのファセットとする。たとえば立方体のファセットは、三角形 12 枚ではなく、4 頂点からなる正方形 6 枚になる。
- **失敗しうる呼び出しは、すべて `convx::Error` を返す。** 構築は入力を決まった順で調べ、最初に見つかったエラーを返す。
- **バージョンは 0.1.0 である。** 0.x のマイナーバージョンでは、公開 API に互換性のない変更が入ることがある。

## 最初の例

```rust
use convx::ConvexHullBuilder;

fn main() -> Result<(), convx::Error> {
    // 単位正方形、その中心、辺の中点、頂点の複製。
    let points = [
        0.0, 0.0, // 0
        1.0, 0.0, // 1
        1.0, 1.0, // 2
        0.0, 1.0, // 3
        0.5, 0.5, // 4: 内部
        0.5, 0.0, // 5: 辺の上
        0.0, 0.0, // 6: 点 0 と同じ点
    ];
    let hull = ConvexHullBuilder::new(2, &points).build()?;

    // 入力のどの点も、これらのいずれか、またはその重複である。
    assert_eq!(hull.vertices(), [0, 1, 2, 3]);
    assert_eq!(hull.coplanar_points(), [5]);
    assert_eq!(hull.interior_points(), [4]);
    assert_eq!(hull.representative()[6], 0);
    Ok(())
}
```

## 使い方

### 凸包

`ConvexHullBuilder` が `ConvexHull` を作る。ファセットは `Facets` と `Facet`、その平面は `Planes` と `Plane`、単体に分けた境界は `TriangulationView` と `BoundarySimplex` を通して参照する。

```rust
use convx::{BoundarySimplex, ConvexHull, ConvexHullBuilder, Facet, Facets, Plane, Planes, TriangulationView};

fn main() -> Result<(), convx::Error> {
    // 四角錐。4 隅の底面と頂点。
    let points = [
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, // 0 から 3: 底面
        0.5, 0.5, 1.0, // 4: 頂点
    ];
    let hull: ConvexHull = ConvexHullBuilder::new(3, &points).build()?;
    assert_eq!(hull.dim(), 3);

    // ファセットは 5 枚。4 頂点の底面が 1 枚と、三角形が 4 枚。
    let facets: Facets = hull.facets();
    assert_eq!(facets.len(), 5);
    let base: Facet = facets.get(0).unwrap();
    assert_eq!(base.vertices(), [0, 1, 2, 3]);
    assert_eq!(base.neighbors(), [1, 2, 3, 4]);
    // 底面の頂点を、外から見て反時計回りに並べたもの。
    assert_eq!(hull.boundary_cycle(0), Some(vec![0, 2, 3, 1]));

    // 平面は最初の呼び出しで `f64` で計算し、保持する。
    let planes: Planes = hull.planes()?;
    let floor: Plane = planes.get(0).unwrap();
    assert_eq!(floor.normal(), [0.0, 0.0, -1.0]);
    assert_eq!(floor.offset(), 0.0);

    // 境界を三角形で表す。底面は 2 枚に分かれる。
    let triangles: TriangulationView = hull.triangulation();
    assert_eq!(triangles.len(), 6);
    let first: BoundarySimplex = triangles.get(0).unwrap();
    assert_eq!(first.facet(), 0);
    assert_eq!(first.vertices().len(), 3);

    // 体積は `f64`。底面積と高さの積の 3 分の 1。
    assert!((hull.volume() - 1.0 / 3.0).abs() < 1e-15);
    Ok(())
}
```

### Delaunay 三角形分割

`DelaunayBuilder` が `DelaunayTriangulation` を作る。単体は `Simplices` と `DelaunaySimplex` を通して参照する。

```rust
use convx::{DelaunayBuilder, DelaunaySimplex, DelaunayTriangulation, Simplices};

fn main() -> Result<(), convx::Error> {
    // 正方形の 4 隅と中心。
    let sites = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5];
    let delaunay: DelaunayTriangulation = DelaunayBuilder::new(2, &sites).build()?;
    assert_eq!(delaunay.dim(), 2);
    assert_eq!(delaunay.representative(), [0, 1, 2, 3, 4]);

    // 三角形が 4 つ。どれも中心を頂点に持つ。
    let simplices: Simplices = delaunay.simplices();
    assert_eq!(simplices.len(), 4);
    for simplex in simplices.iter() {
        let simplex: DelaunaySimplex = simplex;
        assert!(simplex.vertices().contains(&4));
        // 各面を挟んで隣接する単体。面がサイトの凸包の上にあれば `u32::MAX`。
        assert_eq!(simplex.neighbors().len(), 3);
    }
    Ok(())
}
```

サイトは、凸包の内側にあるものも含めて、すべて三角形分割の頂点になる。一つの円や球面の上にあるサイトは、述語の厳密な符号で特定して一つのグループにまとめ、添字だけで決まる規則で分割する。

### Voronoi 図

`VoronoiBuilder` が `VoronoiDiagram` を作る。`VoronoiDiagram` は、有限の頂点（`VoronoiVertices`、`VoronoiVertex`）、半直線（`VoronoiRays`、`VoronoiRay`）、サイトごとのセル（`VoronoiCells`、`VoronoiCell`）、二つのセルが接する面（`VoronoiInterfaces`、`VoronoiInterface`）からなる。

```rust
use convx::{
    VoronoiBuilder, VoronoiCell, VoronoiCells, VoronoiDiagram, VoronoiInterface,
    VoronoiInterfaces, VoronoiRay, VoronoiRays, VoronoiVertex, VoronoiVertices,
};

fn main() -> Result<(), convx::Error> {
    // 正方形の 4 隅は一つの円の上にある。頂点は中心の 1 つ。
    let sites = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0];
    let voronoi: VoronoiDiagram = VoronoiBuilder::new(2, &sites).build()?;
    assert_eq!(voronoi.dim(), 2);
    assert_eq!(voronoi.representative(), [0, 1, 2, 3]);

    let vertices: VoronoiVertices = voronoi.vertices();
    let center: VoronoiVertex = vertices.get(0).unwrap();
    assert_eq!(center.sites(), [0, 1, 2, 3]);
    assert!(center.coords().iter().all(|x| (x - 1.0).abs() < 1e-12));

    // 半直線は中心から、正方形の各辺を通って外へ出る。
    let rays: VoronoiRays = voronoi.rays();
    let down: VoronoiRay = rays.get(0).unwrap();
    assert_eq!(down.apex(), 0);
    assert_eq!(down.hull_facet(), [0, 1]);
    assert_eq!(down.direction(), [0.0, -1.0]);

    // どのセルも有界でない。頂点 1 つと半直線 2 本。
    let cells: VoronoiCells = voronoi.cells();
    let corner: VoronoiCell = cells.get(0).unwrap();
    assert_eq!(corner.site(), 0);
    assert_eq!(corner.vertices(), [0]);
    assert_eq!(corner.ray_numbers().len(), 2);
    assert_eq!(corner.rays().count(), 2);

    // 隣り合う隅のセルは接し、向かい合う隅のセルは接しない。
    let interfaces: VoronoiInterfaces = voronoi.interfaces();
    let pairs: Vec<[u32; 2]> = interfaces.iter().map(|face: VoronoiInterface| face.sites()).collect();
    assert_eq!(pairs, [[0, 1], [0, 3], [1, 2], [2, 3]]);
    let bottom: VoronoiInterface = interfaces.get(0).unwrap();
    assert_eq!(bottom.vertices(), [0]);
    assert_eq!(bottom.ray_numbers().len(), 1);
    assert_eq!(bottom.rays().count(), 1);
    Ok(())
}
```

Voronoi 頂点は、同一球面上にある Delaunay 単体の一つのグループに対応する。グループは厳密な符号で判定してまとめ、外心どうしが近いかどうかは使わない。

### スライスの代わりに配列を渡す

`StaticConvexHull`、`StaticDelaunay`、`StaticVoronoi` は、`1 <= D <= 8` について `&[[f64; D]]` を受け取り、ビルダーと同じ結果を返す。それ以外の `D` を指定するとコンパイルエラーになる。

```rust
use convx::{StaticConvexHull, StaticDelaunay, StaticVoronoi};

fn main() -> Result<(), convx::Error> {
    let tetrahedron = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(StaticConvexHull::<3>::build(&tetrahedron)?.facets().len(), 4);
    assert_eq!(StaticDelaunay::<3>::build(&tetrahedron)?.simplices().len(), 1);
    assert_eq!(StaticVoronoi::<3>::build(&tetrahedron)?.vertices().len(), 1);
    Ok(())
}
```

### エラー

構築が失敗すると `convx::Error` を返す。`planes()` も同じである。この enum は non-exhaustive なので、`match` にはワイルドカードの分岐を置く。

```rust
use convx::{ConvexHullBuilder, Error};

// 一直線上の 3 点は平面を張らない。
let points = [0.0, 0.0, 1.0, 1.0, 2.0, 2.0];
match ConvexHullBuilder::new(2, &points).build() {
    Err(Error::DegenerateDimension { actual_dim, spanning_points }) => {
        // 点が張る空間は直線（1 次元）で、点 0 と点 1 がそれを張る。
        assert_eq!(actual_dim, 1);
        assert_eq!(spanning_points, [0, 1]);
    }
    Err(other) => panic!("another failure: {other}"),
    Ok(_) => panic!("built"),
}
```

## 約束すること

- **組合せ的な結果は、与えられた `f64` の値に対する厳密演算の結果である。** 頂点、ファセットと隣接ファセット、各点の分類、Delaunay 単体、Voronoi 図の接続関係がこれに当たる。入力の座標は、そのビット列が表す数として扱う。
- **入力のどの添字も、ちょうど一つの分類に属する。** `representative()` は、重複点を、等しい点のうち最小の添字に対応づける（`-0.0` と `+0.0` は等しい）。代表点は頂点、頂点でない境界上の点、内部点のいずれかに分類され、それぞれのリストは昇順に並ぶ。
- **API が返す凸包は、構築の順序に依らない。** 同じバイナリは、同じ入力に対して、毎回同じ凸包、三角形分割、Voronoi 図を返す。
- **エラーは決まった順で報告する。** 次元、スライスの長さ、有限でない座標（最初の該当点を示す）、点の数の上限超過、異なる点の不足、次元の退化（`DegenerateDimension`。点が実際に張る部分空間の次元と、それを張る点を添える）の順に調べる。
- **構築でのメモリ割り当ての回数は、点の数とともに増えない。** クレートの割り当てテストで、そのテストが構築する種類の入力について確かめている。

## 約束しないこと

- **結果に含まれる数値は `f64` である。** 平面の法線とオフセット、体積、Voronoi 頂点の座標は丸めた値であり、どの判定にも使わない。そうした数値が有限でないとき、`planes()` と Voronoi の構築は失敗することがある。トポロジーは、これらの数値を計算する前に決まっている。
- **バージョンが変わると**、同一球面上のサイトのグループの中で選ぶ対角線と、単体を並べる順序は変わることがある。
- **`D >= 4`** の Voronoi 図は、頂点、半直線、セル、接するセルの対を並べたものであり、完全な胞複体ではない。
- **設定できる許容誤差はない。** ほぼ等しい点をまとめることも、入力を摂動させることもしない。1 ビット違う点は別の点として扱う。
- 構築は**逐次**であり、点の追加や削除で結果を**更新することはできない。**

## 制限

| | |
| --- | --- |
| 次元 | `D >= 1`。`D` が大きいほど厳密評価に時間がかかる。配列の API は `1 <= D <= 8` |
| 点の数 | 最大 `u32::MAX`。添字は `u32` |
| 座標 | 有限の `f64` |
| Rust | 1.89 以上。`std` が必要 |
| 依存 | `faer`、`pulp`、`thiserror` |

## 性能

Qhull、CGAL、Rust のクレートとの、標準的なデータでの比較は [docs/comparison.ja.md](https://github.com/YUKIKEDA/convx/blob/main/docs/comparison.ja.md) にまとめた。計測環境、各ライブラリのバージョン、各ライブラリが出力するものの違いも載せている。他のライブラリのほうが速い結果も、表から除いていない。このクレートが CGAL と Qhull に対して合否を判定する基準は、これとは別に、[docs/verification.md](https://github.com/YUKIKEDA/convx/blob/main/docs/verification.md) のパリティ計測（閾値を持つ決まったデータの集合）として定めている。

## 仕組み

仕様は [docs/design.ja.md](https://github.com/YUKIKEDA/convx/blob/main/docs/design.ja.md)（英語版: [docs/design.md](https://github.com/YUKIKEDA/convx/blob/main/docs/design.md)）、結果の確かめ方は [docs/verification.md](https://github.com/YUKIKEDA/convx/blob/main/docs/verification.md)、各判断の根拠となった計測は [docs/bench.md](https://github.com/YUKIKEDA/convx/blob/main/docs/bench.md) にある。

## ライセンス

[Apache License, Version 2.0](https://github.com/YUKIKEDA/convx/blob/main/LICENSE-APACHE) と [MIT license](https://github.com/YUKIKEDA/convx/blob/main/LICENSE-MIT) のどちらかを選んで利用できる。

明示的な別段の表明がない限り、Apache-2.0 ライセンスの定義に従ってこのクレートに含めるために意図的に提出された貢献は、追加の条項や条件なしに、上記の二つのライセンスの下で提供されるものとする。
