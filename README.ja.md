[English](https://github.com/YUKIKEDA/convx/blob/main/README.md) | 日本語

# convx

任意の次元の凸包、Delaunay 三角形分割、Voronoi 図を計算する純 Rust のクレートである。トポロジーに関わる判定はすべて、述語の厳密な符号で行う。

- **厳密である。** 点が面のどちら側にあるか、点が同一平面上・同一球面上にあるかは、入力座標の行列式の符号で決める。浮動小数点演算で符号を確定できないときは、厳密に評価する。許容誤差はなく、入力を摂動させることもない。
- **任意の次元を扱う。** すべての `D >= 1` を一つの実装で扱い、効果のある `D = 2` と `D = 3` には専用の経路を持つ。
- **退化した入力も入力である。** 重複点、面の上の点、同一円周上・同一球面上のサイト、格子は、一般の位置にある点と同じ規則で扱う。結果は、入力の各点がどうなったかを示す。
- このクレートに **`unsafe` はない。**

```toml
[dependencies]
convx = "0.1"
```

版は 0.1.0 である。0.x のマイナー版は公開 API を壊すことがある。

## 凸包

点は `f64` の行優先のスライス一つで渡す。1 点が `D` 個の座標である。

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

    // 入力のどの点も、このどれかか、その重複である。
    assert_eq!(hull.vertices(), [0, 1, 2, 3]);
    assert_eq!(hull.coplanar_points(), [5]);
    assert_eq!(hull.interior_points(), [4]);
    assert_eq!(hull.representative()[6], 0);

    // 辺が 4 本。それぞれ 2 頂点と 2 つの隣を持つ。
    assert_eq!(hull.facets().len(), 4);
    for facet in hull.facets().iter() {
        assert_eq!(facet.vertices().len(), 2);
        assert_eq!(facet.neighbors().len(), 2);
    }

    // 平面と体積は、決まったトポロジーから `f64` で計算する。
    assert_eq!(hull.volume(), 1.0);
    let planes = hull.planes()?;
    assert_eq!(planes.len(), 4);
    Ok(())
}
```

面は**論理面**である。厳密に同一平面上にある単体は一つの面になるので、立方体の面は三角形 12 枚ではなく、4 頂点の面 6 枚である。境界を単体として欲しいときは `triangulation()` を使う。

## Delaunay 三角形分割

```rust
use convx::DelaunayBuilder;

fn main() -> Result<(), convx::Error> {
    // 正方形の 4 隅と中心。
    let sites = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5];
    let delaunay = DelaunayBuilder::new(2, &sites).build()?;

    // 三角形が 4 つ。どれも中心を頂点に持つ。
    assert_eq!(delaunay.simplices().len(), 4);
    for simplex in delaunay.simplices().iter() {
        assert!(simplex.vertices().contains(&4));
        // 各面の向こうの隣。サイトの包の上では `u32::MAX`。
        assert_eq!(simplex.neighbors().len(), 3);
    }
    Ok(())
}
```

すべてのサイトが三角形分割に入る。サイトの包の内側にあるサイトも入る。同一円周上・同一球面上にあるサイトは、厳密な符号で見つけて一つのグループにまとめ、包の面を分割するのと同じ一つの規則で分割し直す。

## Voronoi 図

```rust
use convx::VoronoiBuilder;

fn main() -> Result<(), convx::Error> {
    let sites = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5];
    let voronoi = VoronoiBuilder::new(2, &sites).build()?;

    // 中心のセルは 4 頂点の正方形で、半直線を持たない。
    let center = voronoi.cells().iter().find(|cell| cell.site() == 4).unwrap();
    assert_eq!(center.vertices().len(), 4);
    assert_eq!(center.ray_numbers().len(), 0);

    // 隅のセルは有界でない。半直線を持つ。
    let corner = voronoi.cells().iter().find(|cell| cell.site() == 0).unwrap();
    assert!(!corner.ray_numbers().is_empty());
    Ok(())
}
```

Voronoi 頂点は、同一球面上にある Delaunay 単体の一つのグループである。グループは厳密な符号でまとめ、外心の近さではまとめない。同一円周上の 4 サイトは一つの頂点になる。

## スライスの代わりに配列を渡す

`StaticConvexHull`、`StaticDelaunay`、`StaticVoronoi` は、`1 <= D <= 8` について `&[[f64; D]]` を受け取り、同じ結果を返す。

```rust
use convx::StaticConvexHull;

fn main() -> Result<(), convx::Error> {
    let tetrahedron = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let hull = StaticConvexHull::<3>::build(&tetrahedron)?;
    assert_eq!(hull.facets().len(), 4);
    Ok(())
}
```

## 約束すること

- **組合せ的な結果は、与えられた `f64` の値に対する厳密演算の結果である。** 頂点、面とその隣、各点の分類、Delaunay 単体、Voronoi の接続関係がこれに当たる。入力の座標は、そのビット列が表す数である。
- **入力のどの添字にも、行き先が一つある。** `representative()` は、重複点を、等しい点のうち最小の添字に対応づける（`-0.0` と `+0.0` は等しい）。代表点は、頂点、頂点でない境界上の点、内部点のいずれかであり、どの一覧も昇順である。
- **公開する凸包は構築の順序に依らない。** 同じバイナリは、同じ入力に対して、毎回同じ凸包、三角形分割、図を返す。
- **失敗は決まった順で報告する。** 次元、スライスの長さ、有限でない座標（最初の該当点を示す）、点が多すぎる、異なる点が少なすぎる、点が次元を張らない（`DegenerateDimension`。張っている部分を張る点を添える）の順である。
- **構築の割り当て回数は、点の数とともに増えない。** これは、クレートの割り当てテストが構築する種類の入力について確かめている。

## 約束しないこと

- **結果が持つ数値は `f64` である。** 平面の法線とオフセット、体積、Voronoi 頂点の座標は丸められた値であり、どの判定にも使わない。そのような数値が有限でないとき、`planes()` と Voronoi の構築は失敗することがある。トポロジーはその前に決まっている。
- **版をまたぐと**、同一球面上のサイトのグループの中で選ぶ対角線と、単体を並べる順序は変わることがある。
- **`D >= 4`** の Voronoi 図は、頂点、半直線、セル、接するセルの対を並べたものであり、完全な胞複体ではない。
- **設定する許容誤差はない。** ほぼ等しい点をまとめることも、入力を揺らすこともしない。1 ビット違う点は別の点である。
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

標準的なデータでの Qhull、CGAL、Rust のクレートとの比較は、計測した機械、各ライブラリの版、各ライブラリが何を出力するかとともに、[docs/comparison.md](https://github.com/YUKIKEDA/convx/blob/main/docs/comparison.md) にある。これは報告であり、他のライブラリのほうが速い行もそのまま載せている。

## 仕組み

仕様は [docs/design.ja.md](https://github.com/YUKIKEDA/convx/blob/main/docs/design.ja.md)（英語版: [docs/design.md](https://github.com/YUKIKEDA/convx/blob/main/docs/design.md)）、結果の確かめ方は [docs/verification.md](https://github.com/YUKIKEDA/convx/blob/main/docs/verification.md)、各判断の根拠となった計測は [docs/bench.md](https://github.com/YUKIKEDA/convx/blob/main/docs/bench.md) にある。

## ライセンス

[Apache License, Version 2.0](https://github.com/YUKIKEDA/convx/blob/main/LICENSE-APACHE) と [MIT license](https://github.com/YUKIKEDA/convx/blob/main/LICENSE-MIT) のどちらかを選んで利用できる。

明示的な別段の表明がない限り、Apache-2.0 ライセンスの定義に従ってこのクレートに含めるために意図的に提出された貢献は、追加の条項や条件なしに、上記の二つのライセンスの下で提供されるものとする。
