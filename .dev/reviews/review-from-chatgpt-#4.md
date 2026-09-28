全体として、**設計の方向性はかなり良いです**。特に、

> 述語の符号 → トポロジー判定 → 変異

を最上位の依存関係にして、`FacetPlane` や SIMD 距離をトポロジーから切り離しているのは、robust computational geometry の設計として筋が通っています。適応精度・厳密述語を構築と分離する考え方自体も、既存の robust predicate の設計思想と一致しています。([CMU School of Computer Science][1])

ただし、**現状は「実装開始してよい仕様」より一段手前**です。特に Delaunay/Voronoi と `volume()` に、仕様を固定する前に直したほうがいい箇所があります。

私なら優先度を、

* **P0: 数学的に修正が必要**
* **P1: 仕様を追加しないと実装者が判断できない**
* **P2: 実装上の改善**

に分けます。

---

# 1. P0: `volume()` の定義に問題があります

ここは明確に修正したほうがいいです。

現在、

> 境界単体は、末尾 2 点を入れ替える前の昇順頂点列の辞書順に並べ、その順に左から足す。最後に絶対値を返す。

となっています。

これは**符号付き体積を正しく足せることを保証しません**。

例えば 2D の正方形を考えると、境界辺を頂点インデックス順に並べただけでは、辺の向きが一貫した外向きになりません。

したがって、

```text
signed_volume(r, facet_0)
+ signed_volume(r, facet_1)
+ ...
```

をそのまま足すと、項が相殺する可能性があります。

### 修正案

`volume()` は次のどちらかにすべきです。

### 案A: 公開 triangulation の向きを使う

> `triangulation()` が返す外向き境界単体を使い、それぞれについて `r` との符号付き体積を計算して加算する。

これが一番自然です。

### 案B: 各境界単体の絶対値を足す

凸多面体について基準点 `r` を極点に取るなら、`r` を含まない境界単体との錐体の体積を絶対値で足す方法も使えます。

ただし仕様としては **A のほうを推奨**します。

例えば、

```text
1. 境界 triangulation を外向きに正規化する。
2. 各 simplex S について r と S が作る D-simplex の符号付き体積を計算する。
3. すべてを加算する。
4. 数値誤差を含むため abs() を最後に一度だけ適用する。
```

です。

---

# 2. P0: Delaunay の「全体が共球」の処理がまだ定義になっていません

ここが最大の問題です。

現在、

> 持ち上げが平坦なとき、初期単体は辞書順で最小のアフィン独立な D+1 点である。残りの代表はインデックス順に挿入する。

となっています。

しかし、これだけでは**どの Delaunay triangulation を返すかが決まりません**。

例えば正方形：

```text
0 ---- 1
|      |
|      |
3 ---- 2
```

では対角

```text
0 - 2
```

と

```text
1 - 3
```

の両方が Delaunay です。

これは仕様にも書かれているので問題ありません。

問題は、

> 「ではこのアルゴリズムが、どのようにして一方を選ぶのか？」

がまだありません。

---

## 特に問題なのが「平坦な lifting」

全点が共円なら、持ち上げ点は1枚の超平面上に載ります。

つまり、

```text
lower hull
upper hull
```

を orientation の符号だけで区別できません。

ところが仕様では、

> 全体が共球で持ち上げが平坦なときは、この符号を使わず、次の分割に入る。

としています。

この「次の分割」が仕様化されていません。

さらに、通常の `ConvexHull` は境界 `(D-1)`-facet を返す設計です。

しかし Delaunay が欲しいのは**D次元凸多面体の内部を D-simplex に分割したもの**です。

つまり、

> 「平坦な lifting に対して ConvexHull の Quickhull をどう使えば Delaunay simplex が出てくるのか」

をもう一段定義する必要があります。

---

# 3. Delaunay の退化処理は「pulling triangulation」に固定するのがおすすめ

ここは仕様をかなり綺麗にできます。

例えば、

> Delaunay の lower hull は、まず厳密述語によって Delaunay cell の集合として求める。共球により D+1 点を超える maximal lower cell が得られた場合、その cell を代表インデックス昇順の pulling triangulation により D-simplex へ分割する。

とします。

Pulling triangulation は、頂点に全順序を与えると再帰的に一意な triangulation を作れるので、今回の

```text
代表インデックス昇順
```

という設計と非常に相性がいいです。([サイエンスダイレクト][2])

これなら、

```text
入力順
 ↓
代表インデックス順
 ↓
Delaunay cell
 ↓
lexicographic / pulling triangulation
 ↓
DelaunaySimplex
```

という完全に決定的な流れになります。

そして「同じバイナリで逐次/並列が一致する」という要求もかなり簡単になります。

---

# 4. P0: 「Voronoi は Delaunay の双対」の記述を少し修正したほうがいい

一般位置なら問題ありません。

しかし、この仕様は明示的に

> 共球を許す

ので、ここが重要です。

共球の場合、数学的な Delaunay **complex** は simplex ではなく、複数サイトを頂点に持つ非単体的な cell になることがあります。Voronoi 側も、通常の generic な「1 vertex = D+1 sites」ではなく、より多くのサイトが同じ Voronoi vertex に入ります。([スプリンガーリンク][3])

つまり、

```text
Delaunay triangulation
```

と

```text
Voronoi diagram
```

は、退化時には文字通りの face-to-face 双対ではありません。

仕様の実態は、

```text
Delaunay complex
    ↓ canonical triangulation
DelaunayTriangulation
```

です。

そして Voronoi は、

```text
Delaunay complex
```

の双対です。

---

## したがって用語をこうすると綺麗です

```text
exact lower hull
    ↓
Delaunay cells
    ↓
canonical triangulation
    ↓
DelaunayTriangulation
```

Voronoi は

```text
Delaunay cells
    ↔
Voronoi cells
```

の双対。

公開 API として `DelaunayTriangulation` を出すこと自体は問題ありません。

ただし仕様書には、

> 共球退化時、`DelaunayTriangulation` は Delaunay complex の canonical triangulation であり、Voronoi との双対関係は triangulation ではなく underlying Delaunay complex に対して成立する。

と書いておくと、かなり強い仕様になります。

---

# 5. P0: Voronoi の「有限頂点をまとめる」条件は、もう少し厳密にしたい

現在は、

> 持ち上げて同一超平面に載る Delaunay 単体を、推移的に一つへまとめる

となっています。

方向としては正しいです。

ただし実装仕様としては、

```text
Delaunay simplex A
    shared D-face
Delaunay simplex B
```

だけでなく、

> **同一 maximal lower cell に属する simplex は同一 Voronoi vertex を共有する**

と定義したほうが綺麗です。

つまり、

```text
Delaunay simplex
      ↓
maximal cospherical site set
      ↓
one Voronoi vertex
```

です。

これは今の「推移的にまとめる」とほぼ同じ意味ですが、後者のほうが実装者にとって明確です。

---

# 6. P0: VoronoiInterface の意味が退化時に曖昧

ここも重要です。

正方形の場合、

```text
0 --- 1
|     |
3 --- 2
```

で対角 `0-2` を選ぶと、Delaunay には `0-2` という edge が存在します。

しかし Voronoi では、サイト 0 と 2 の cell は**面を共有せず、中央の一点だけで接します**。

したがって、

```rust
VoronoiInterface {
    sites: [0, 2],
    vertices: [center],
}
```

という結果は数学的にはあり得ます。

しかし現在のコメント、

```rust
/// この境界面の双対は、この 2 サイトを結ぶ Delaunay 辺
```

は、`interface` が常に `(D-1)` 次元だと読めます。

退化時にはそうではありません。

### 修正

例えば、

> `VoronoiInterface` は「2サイトの Voronoi cell の共通部分」を表す。一般位置では `(D-1)` 次元だが、共球退化時にはそれより低次元になることがある。

と定義しておくのが安全です。

これはかなり重要です。

---

# 7. P0: predicate の種類がまだ足りません

第1節では、

* orientation
* 距離符号
* リッジの凸性
* 共面
* 誤差上界
* 厳密符号

とありますが、仕様としてはそれぞれの**数学的な polynomial** を明示したほうがいいです。

特に、

> リッジの凸性

が何の符号なのかが分かりません。

実装者がここで独自解釈できます。

これはこの設計の思想、

> 同じ polynomial の厳密符号

と矛盾します。

---

## 最低限、predicate catalog を作るべきです

例えば：

| Predicate         | 入力                 | exact value                 |
| ----------------- | -------------------- | --------------------------- |
| `orient_k`        | k+1 点               | k-dimensional determinant   |
| `side`            | facet D点 + p        | `orient_D(...)`             |
| `affine_rank`     | 点集合               | `orient_k` の非零性         |
| `ridge_convexity` | 隣接 simplex + point | 明示した determinant        |
| `lift_orient`     | D+2 lifted points    | D+1 dimensional determinant |

こうしておくと、

```text
「この判定には orientation を使う」
```

だけでなく、

```text
「orientation_D(P0,...,PD) の符号を使う」
```

まで仕様が固定できます。

---

# 8. P1: `orientation` は D 固定ではなく k-dimensional にしたほうがいい

これは実装前に決めるべきです。

ランク判定で、

```text
0次元 → 1次元 → 2次元 → ... → D次元
```

とアフィン次元を増やします。

また、facet plane を作るための独立性判定でも低次元 orientation が必要です。

したがって内部 API は概念的に、

```rust
orient::<K>(...)
```

または

```rust
orientation(k, points)
```

が必要です。

「D <= 4 の専用式」という仕様だけでは、

```text
D=7 だけど rank=2 を判定する
```

ケースをどう処理するかが不明です。

---

# 9. P1: 「誤差上界」は非常に良いが、評価モデルを固定したほうがいい

この部分は設計としてかなり良いです。

> 実際に評価した式の絶対誤差の上界を持つ。

これは単なる epsilon 判定よりずっと正しいです。

ただし実装では、

```text
(a*b - c*d)
```

を、

* multiply → multiply → subtract
* FMA
* SIMD FMA
* compiler contraction

のどれで評価するかによって丸め誤差が変わります。

実際、FMA の有無で determinant の反交換性などに問題が出るケースも報告されています。([スプリンガーリンク][4])

したがって、

> 「実際に評価した式」

をさらに、

> **評価順序・使用演算を含む evaluator expression tree**

として定義するとかなり強くなります。

例えば、

```text
Mul(a,b)
Mul(c,d)
Sub(...)
```

という expression tree に対して error bound を生成する、という構造です。

これは将来的に非常に価値があります。

---

# 10. P1: overflow / underflow の仕様をもう一段明確に

ここはかなり重要です。

仕様では、

> 浮動小数行列式が overflow しても失敗しない。厳密符号へ落とす。

となっています。

これは良いです。

ただし、

```text
overflow
underflow
subnormal
FMA
-0.0
```

を全部、

```text
filter stage
```

の仕様に入れたほうがいいです。

特に underflow は危険です。

例えば true value が非ゼロなのに、

```text
computed = 0
error_bound = 0
```

になってしまうフィルタは不正です。

robust predicate の文献でも overflow/underflow は通常の adaptive filter とは別に注意すべき問題として扱われています。([スプリンガーリンク][4])

---

# 11. P1: Quickhull の並列化仕様は、まだ correctness proof が必要

ここは面白い設計ですが、今の仕様だけでは少し危険です。

特に、

```text
T(P) ∩ T(Q) = ∅
H(P) ∩ H(Q) = ∅
```

に加えて、

> 予定単体の orientation で衝突を落とす

としています。

しかし、これが

> P を単独で commit した結果と、P/Q をローカルに並列構築した結果が同型になる

ことの十分条件であることを、仕様として証明できていません。

---

## 特に必要なのは「予定状態」の定義

例えば、

```text
P の local mutation
Q の local mutation
```

がそれぞれどの facet を生成するのか。

その facet に対して、

```text
Q が outside になるか
```

をどう調べるのか。

そして、

```text
P commit
Q commit
```

後に adjacency が同じになること。

ここを invariant として書いたほうがいいです。

---

## 私なら Phase 3 ではもっと保守的にします

最初の並列版では、

```text
batch selection
    ↓
各 candidate の local plan
    ↓
plan conflict check
    ↓
parallel local construction
    ↓
barrier
    ↓
sequential deterministic commit
    ↓
debug build では sequential simulation と比較
```

を必須にします。

特に debug 時には、

```text
parallel batch result
==
sequentially applying same batch in index order
```

を exact topology で確認する。

これなら並列化バグをかなり早く捕まえられます。

---

# 12. P1: 「初期単体の選択は一般には未固定」と「逐次/並列一致」が少し衝突している

第5節では、

> 初期単体にどの点を選ぶかは、一般には仕様で固定しない。

一方で、

> 同じバイナリの逐次と並列で同じ分割になる。

となっています。

同一 binary 内なら実装依存でも構いませんが、並列化では初期単体が共有状態になります。

なので、

> **初期 simplex の選択は binary-local な実装詳細だが、parallel/serial の両方で同一 selection function を使用する**

と明記したほうがいいです。

例えば、

```text
select_initial_simplex(points) -> simplex
```

を deterministic function とする。

---

# 13. P1: `coplanar_points` の分類アルゴリズムは少し複雑すぎる

ここは数学的には筋が通っていますが、かなり実装が難しいです。

特に、

> 点が facet plane 上にある
>
> ↓
>
> facet polygon の convex hull の外側
>
> ↓
>
> vertex に昇格
>
> ↓
>
> facet を再分割
>
> ↓
>
> 他の facet にも反映

という処理です。

これは実質、

**最終的な hull を一度作ったあと、facet 内の coplanar points を再度 convex hull 化している**

わけです。

それなら仕様上、

```text
first:
    compute simplicial hull

second:
    merge coplanar simplicial facets
```

まで終えたあと、

```text
third:
    for every supporting hyperplane:
        compute convex hull of all input points lying on it
```

と明示したほうが理解しやすいです。

つまり、

```text
global hull topology
        ↓
supporting hyperplane groups
        ↓
points on each support plane
        ↓
facet-local convex hull
        ↓
LogicalFacet.vertices
```

という二段階構築です。

これは現在の仕様の意図ともかなり一致しています。

---

# 14. P1: `LogicalFacet.vertices` は「極点」だけ、と明示したほうがいい

今でも書いてありますが、かなり重要なので API コメントに入れたほうがいいです。

例えば立方体の一面に、

```text
A ---- B
|  P   |
D ---- C
```

と内部点 P がある場合、

```rust
vertices = [A, B, C, D]
coplanar_points = [P]
```

です。

これは現在の設計と合っています。

一方で、facet 内部の triangulation に P を使うかどうかは別問題。

ここを、

```text
LogicalFacet.vertices
    = support plane intersection polytope の extreme points

coplanar_points
    = hull boundary 上だが global hull vertex ではない points
```

と定義すると非常に明快です。

---

# 15. P1: `representative` の公開仕様は良い。ただし Delaunay/Voronoi で明記したほうがいい

ここは良い設計です。

特に、

```text
i -> representative[i]
```

を保持して、

```text
representative[representative[i]]
    == representative[i]
```

を不変条件にするのは良いです。

さらに、

> Delaunay/Voronoi の `vertices` / `sites` に出てくる index はすべて representative index である。

を明記すると API 利用者に親切です。

---

# 16. P1: `FacetPlane` はトポロジーから完全に隔離する方針を維持してほしい

これは今の設計でかなり良い部分です。

特に、

> `FacetPlane` の `x·n + offset` は判定に使わない。

これは**そのまま絶対に維持したほうがいい**です。

公開 plane は inexact construction です。

一方、

```text
orientation
```

は exact predicate。

この二つを混ぜると、せっかくの設計が壊れます。

Qhull なども hyperplane と topological orientation を別概念として扱っているので、この分離は妥当です。([Qhull][5])

---

# 17. P2: `Simplex.normal` は本当に必要か再検討してよい

```rust
normal: Vec<f64>
offset: f64
```

を construction facet に持っていますが、

> SIMD 距離用

なら、

```text
predicate geometry
```

と

```text
distance cache
```

をさらに明確に分離できます。

例えば、

```rust
struct FacetPredicate {
    vertices: ...
}

struct FacetMetricCache {
    normal: ...
    offset: ...
}
```

のようにすると、

> normal が NaN になったから topology が壊れた

という事故を構造的に防げます。

---

# 18. P2: `Vec` が大量に存在するので、高次元ではかなり重くなる

```rust
struct Simplex {
    vertices: Vec<u32>,
    neighbors: Vec<FacetId>,
    normal: Vec<f64>,
}
```

は、D が固定でも各 simplex ごとに3つの heap allocation が発生する設計です。

これは Phase 2 の correctness first なら問題ありませんが、最終設計としてはかなり重いです。

特に、

```text
D = 3
```

でも、

```text
Vec<u32> vertices
Vec<FacetId> neighbors
Vec<f64> normal
```

なので、1 simplex あたり複数 allocation。

Phase 2 では、

```rust
Vec<Simplex>
```

をまず作って correctness を優先し、

Phase 2 後半または性能フェーズで、

```text
arena packed arrays
SoA
small-vector
inline D <= N
```

を検討するのが良いです。

---

# 19. `u32::MAX` sentinel は良い

これは問題ありません。

```text
point index < u32::MAX
```

なので、

```rust
u32::MAX
```

を `None` として使えます。

ただし公開 API の

```rust
neighbors: Vec<u32>
```

だけを見ると、

```text
u32::MAX
```

が仕様を知らないユーザーには突然現れるので、

```rust
/// `u32::MAX` means no neighbor.
```

を型コメントに入れるべきです。

将来、

```rust
Option<u32>
```

に変えたくなっても、今の段階ならどちらでもいいと思います。

---

# 20. `ConvexHullError` を Delaunay/Voronoi と共有するのは少し違和感があります

現在、

```rust
ConvexHullError
```

の中に、

```rust
NonFiniteCircumcenter
```

があります。

これは API として少し不自然です。

私は、

```rust
pub enum GeometryError {
    NonPositiveDimension,
    LengthMismatch,
    NonFiniteCoordinate,
    TooManyPoints,
    InsufficientPoints,
    DegenerateDimension,
    ExactEvaluationExhausted,
    NonFiniteFacetPlane,
    NonFiniteCircumcenter,
}
```

にするか、

```rust
ConvexHullError
DelaunayError
VoronoiError
```

を分けます。

特に、

```text
NonFiniteFacetPlane
```

は ConvexHull 固有、

```text
NonFiniteCircumcenter
```

は Voronoi 固有、

なので、個人的には

```rust
GeometryError
```

共通基底的な設計が一番綺麗です。

---

# 21. Qhull を benchmark 基準にする方針は良い。ただし correctness oracle にはしない

ここは仕様として非常に重要です。

Qhull は Quickhull を実装しており、Delaunay/Voronoi も lifting を利用します。([Qhull][6])

ただし Qhull 自体は浮動小数点の precision handling と facet merging を行います。デフォルトでも次元によって precision option が入ります。([Qhull][7])

なので、

```text
Qhull == oracle
```

ではなく、

```text
exact mathematical fixtures
    = correctness oracle

Qhull
    = external implementation comparison
    = performance baseline
```

と明確に分けるのが正しいです。

現在の文章はほぼそうなっていますが、明文化したほうがいいです。

---

# 22. Qhull との比較条件も一つ修正

> Qhull はジョグルも入力の摂動も無し

は benchmark として良いです。

ただし、

> 比べるのは論理ファセット

という点も非常に重要です。

Qhull は non-simplicial facet を merge する場合があるため、

```text
convex hull geometry
```

を比較するなら、

```text
normalized vertex-set facets
```

に落として比較するのが適切です。Qhull 自体も non-simplicial facet と triangulated output を別扱いしています。([Qhull][8])

この方針は維持してよいです。

---

# 23. テストに一つ重要なカテゴリを追加したい

今のテストはかなり良いです。

さらに、

## affine transformation invariance

を追加したいです。

ただし「任意の affine transformation」ではなく、少なくとも、

```text
translation
uniform positive scaling
uniform negative scaling
coordinate permutation
point permutation
```

です。

特に point permutation は重要です。

仕様上、

```text
representative = min input index
```

なので、入力順を変えると index は変わります。

したがって、

```text
geometric result after index remapping
```

が一致することを検証します。

---

# 24. `-0.0/+0.0` テストは非常に良い

ここはそのままでいいです。

ただ、

```text
duplicate = ==
```

にしているので、

```text
-0.0 == +0.0
```

を representative selection より前に canonicalize する必要があります。

仕様には既に、

> 比較の前に符号付きゼロを +0.0 へ揃える。

とあります。

これは良いです。

さらに、

```text
+0.0
-0.0
```

が

```text
representative
vertex ordering
exact predicate
```

に一切影響しない、を fixture に入れるといいです。

---

# 25. `volume()` 以外の数値 construction も「exact/inexact」を明示するとさらに良い

今の設計では、

### exact / topology

```text
orientation
rank
coplanarity
visibility
Delaunay lower-hull
```

### inexact / construction

```text
FacetPlane
normal
offset
circumcenter
volume
```

とかなり綺麗に分かれています。

これを仕様書冒頭で表にしてしまうと、非常に読みやすくなります。

例えば：

| 処理                      | exactness        | topology に使用 |
| ------------------------- | ---------------- | --------------- |
| orientation               | exact sign       | Yes             |
| facet side                | exact sign       | Yes             |
| rank                      | exact sign       | Yes             |
| Delaunay lift orientation | exact sign       | Yes             |
| SIMD distance             | filtered/inexact | No              |
| facet normal              | f64              | No              |
| facet offset              | f64              | No              |
| circumcenter              | f64              | No              |
| volume                    | f64              | No              |

これは `convx` の設計思想そのものです。

---

# 26. 私なら仕様をこの4層にさらに整理します

今の3層を少しだけ変更して、

```text
                    ┌─────────────────────┐
                    │ Public Geometry API │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │   Topology Kernel   │
                    │ Quickhull / Delaunay│
                    │  Voronoi complexes  │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │ Exact Predicate Core│
                    │ orientation / rank  │
                    │ lift orientation    │
                    └──────────┬──────────┘
                               │
             ┌─────────────────┴────────────────┐
             │                                  │
 ┌───────────▼──────────┐          ┌───────────▼──────────┐
 │ Floating Construction│          │ Performance Kernels  │
 │ QR / circumcenter     │          │ SIMD / distance     │
 │ plane / volume        │          │ CPU dispatch        │
 └───────────────────────┘          └───────────────────────┘
```

にします。

つまり、

**Exact Predicate Core を topology の唯一の数値的根拠にする。**

これがこのプロジェクトの最大の強みです。

---

# 27. 最優先で修正するならこの5点

私なら実装開始前に、まず以下を仕様へ追加します。

### P0-1 `volume()`

現在の

> 昇順 facet をそのまま signed sum

をやめて、

> 外向きに向きを統一した boundary simplex を使用する。

---

### P0-2 Delaunay の退化

明確に、

```text
Delaunay complex
→ maximal lower cells
→ canonical pulling triangulation
→ DelaunayTriangulation
```

と定義する。

---

### P0-3 Voronoi の双対対象

```text
Voronoi ↔ Delaunay complex
```

であって、

```text
Voronoi ↔ arbitrary triangulation
```

ではない、と定義する。

---

### P0-4 Predicate catalog

最低限、

```text
orient_k
facet_side
affine_rank
ridge_convexity
lift_orientation
```

について、**入力と polynomial を固定する**。

---

### P0-5 parallel correctness

```text
local plan
→ conflict proof
→ local mutation
→ deterministic commit
```

について、

> どの条件なら serial application と同じ topology になるか

を invariant として固定する。

---

# 28. 最終評価

現状をざっくり評価すると、

| 項目                  | 状態                                       |
| --------------------- | ------------------------------------------ |
| 基本アーキテクチャ    | **かなり良い**                             |
| exact predicate 方針  | **非常に良い**                             |
| 入力/重複/ランク仕様  | **良い**                                   |
| Convex Hull topology  | **ほぼ良い**                               |
| coplanar facet の扱い | **良いが実装仕様を補強したい**             |
| Quickhull             | **アルゴリズム仕様をもう少し具体化したい** |
| 並列化                | **面白いが correctness proof が必要**      |
| Delaunay              | **退化時の定義を要修正**                   |
| Voronoi               | **Delaunay complex との関係を要修正**      |
| 数値 construction     | **概ね良い**                               |
| API                   | **良い**                                   |
| テスト方針            | **かなり良い**                             |
| benchmark 方針        | **良い**                                   |

**特に `volume()` は実装前に直すべき明確なバグ候補、Delaunay の退化処理は「未定義部分」として扱うべき箇所です。**

逆にそこを直せば、かなりしっかりした仕様になります。

そして個人的には、この仕様で最も良いところは **「公開 plane / normal / circumcenter は inexact construction、topology は exact predicate だけで決める」**という境界です。これは最後まで崩さないほうがいいです。robust geometry では、orientation や in-circle/in-sphere のような述語を adaptive/exact にし、通常の浮動小数計算を必要なところだけに限定する設計が基本的な戦略になっています。([CMU School of Computer Science][1])

なお、Rust 1.80 の `[[f64; D]]::as_flattened()` については仕様どおりで問題ありません。`as_flattened` は 1.80.0 で安定化されています。([Rust ドキュメント][9])

**次のレビューでは、特に第6節の「並列 Quickhull の予約条件」と、第7〜8節の「Delaunay complex → canonical triangulation → Voronoi」の部分を、実際に正しさが証明できるレベルまで擬似コード化すると、かなり完成度が上がると思います。**

[1]: https://www.cs.cmu.edu/~quake/robust.html?utm_source=chatgpt.com "Fast Robust Predicates for Computational Geometry"
[2]: https://www.sciencedirect.com/science/article/pii/S0195669815002541?utm_source=chatgpt.com "Gelfand–Tsetlin polytopes and the integer decomposition property - ScienceDirect"
[3]: https://link.springer.com/article/10.1007/s00454-022-00371-2?utm_source=chatgpt.com "Continuous and Discrete Radius Functions on Voronoi Tessellations and Delaunay Mosaics | Discrete & Computational Geometry | Springer Nature Link"
[4]: https://link.springer.com/article/10.1007/s10543-023-00975-x?utm_source=chatgpt.com "Fast floating-point filters for robust predicates | BIT Numerical Mathematics | Springer Nature Link"
[5]: https://www.qhull.org/src/libqhull/qh-poly.htm?utm_source=chatgpt.com "poly.c, poly2.c -- polyhedron operations"
[6]: https://qhull.org/html/qhull.htm?utm_source=chatgpt.com "qhull -- convex hull and related structures"
[7]: https://www.qhull.org/html/?utm_source=chatgpt.com "Qhull manual"
[8]: https://www.qhull.org/html/qh-optf.htm?utm_source=chatgpt.com "Qhull format options (F)"
[9]: https://doc.rust-lang.org/stable/core/primitive.slice.html?utm_source=chatgpt.com "slice - Rust"
