レビューしました。結論から言うと、**かなり野心的で、設計の骨格はよくできていますが、このまま実装に入ると「数値幾何」「Facet Merging」「並列トポロジー」の3箇所で大きく手戻りする可能性があります。**
特に、現状は「Qhull級の堅牢性を実現するための設計」と「高速化のための設計」が一部同時に決められており、前者の数学的仕様をもう一段厳密にする必要があります。

以下、重要度順にレビューします。

---

# 総合評価

| 項目             |       評価 | コメント                                |
| ---------------- | ---------: | --------------------------------------- |
| アーキテクチャ   | ★★★★☆ | 層分離は良い                            |
| Rust設計         | ★★★★☆ | Arena / ID / Builder APIは方向性が良い  |
| 並列化設計       | ★★★☆☆ | 面白いが、まだアルゴリズムの証明が不足  |
| 数値頑健性       | ★★☆☆☆ | 最大の要改善ポイント                    |
| Facet Merging    | ★★★☆☆ | 重要性を理解している点は非常に良い      |
| Delaunay/Voronoi | ★★★☆☆ | Lifting Mapの方向性は正しいが仕様不足   |
| 検証戦略         | ★★★☆☆ | 良いが、判定方法に危険箇所あり          |
| 実装ロードマップ | ★★★★☆ | 順序は概ね良い                          |
| 実現可能性       | ★★★☆☆ | 可能だが「Qhullを凌駕」はかなり高い目標 |

**最大の問題は「実装方法」より「正しさを定義する数学的仕様」がまだ足りないことです。**

---

# 1. 最も重要：Dynamic Epsilonを「堅牢性の中心」にしてはいけない

現在、

> Dynamic Epsilon
> `E_dist = (D + 1) * max_range * eps * c1`

という設計になっています。

これは実装上のスケールとしては有用ですが、**これだけではQhull級のrobustnessは保証できません。**

問題は、例えば

* 座標値の大きさ
* 座標差
* 行列の条件数
* cancellation
* 点の配置
* determinantの符号
* hyperplane計算誤差

がすべて同じスケール則には従わないことです。

例えば、

```text
(1e10, 1e10)
(1e10 + 1, 1e10)
(1e10, 1e10 + 1)
```

のような入力では、bounding box scaleだけでは判定精度を適切に評価できません。

### 改善案

`epsilon` を1個の値に集約せず、

```text
Predicate Error Bound
        ↓
orientation / determinant
        ↓
distance-to-plane
        ↓
merge criterion
        ↓
topological decision
```

という形に分離した方がよいです。

つまり、

```rust
orientation_sign(...)
distance_sign(...)
ridge_convexity(...)
facet_merge_test(...)
```

それぞれが**独自の誤差評価**を持つ設計にします。

特に重要なのは、

> 「ε以内だから同一平面」

ではなく、

> 「この計算結果の符号がdouble precisionで確定できるか」

という考え方です。

---

# 2. `f64::EPSILON` と「数値誤差」を混同しない方がいい

仕様では

> `epsilon_mach = 2.22 × 10^-16 (f64::EPSILON)`

となっています。

ここは設計書上、かなり重要な修正ポイントです。

`f64::EPSILON` は「1.0に対して次の表現可能な数との差」であって、**任意の幾何演算の誤差上限ではありません。**

例えば determinant を計算すれば、

```text
入力誤差
× 演算回数
× cancellation
× condition number
```

によって誤差が変化します。

したがって仕様書では、

```text
machine epsilon
```

と

```text
predicate error bound
```

を明確に分離した方がいいです。

---

# 3. Hyperplane solverの選択が少し危険

現在は、

> D <= 4 → 外積
> 5 <= D <= 8 → Modified Gram-Schmidt
> D > 8 / singular → Householder QR

となっています。

ここは再検討を強く推奨します。

特に、

**「5～8次元ならModified Gram-Schmidtが最適」**

とは限りません。

MGSは数値安定性の観点で、Householder QRより不利です。

むしろ、

```text
D <= 3
    specialized determinant/orientation

D == 4..small D
    specialized kernel

general D
    Householder QR / robust linear algebra
```

のように、

**「速いアルゴリズム」と「頑健なアルゴリズム」を別軸で考える**

方がいいです。

さらに重要なのは、Convex Hullでは必ずしも毎回「法線ベクトルを求める」必要はありません。

例えば、

```text
orientation predicate
```

を直接評価できるなら、

```text
normal → dot → sign
```

より、

```text
determinant → sign
```

の方がトポロジー判定として自然です。

---

# 4. Facet Mergingは、この設計の核心なのに仕様がまだ粗い

ここはかなり重要です。

設計では、

> 法線角度
> または
> Centrumとの距離

でmergeするとしています。

しかし、これは**十分条件として危険**です。

例えば、

```text
面A
────────────

面B
              ───────────
```

のように、法線がほぼ同じでも別のsupporting planeなら同一facetではありません。

逆に、

```text
同一平面上にある
```

ことだけでも、topological facetとしてmerge可能とは限りません。

したがって、

```text
Facet Merge =
    coplanarity
    +
    convexity
    +
    adjacency
    +
    supporting-plane consistency
```

という複数条件に分解した方がよいです。

---

# 5. 「代表法線＝最大Hypervolumeの単体」は再検討推奨

現在、

> グループ内でhypervolumeが最大の単体のnormalを代表法線とする

という仕様です。

これは高速化の意図は理解できますが、**代表法線として幾何学的に自然な定義ではありません。**

同じ平面にあるfacet群なら、理想的には

```text
supporting hyperplane
```

そのものを代表値として持つべきです。

例えば、

```rust
FacetPlane {
    normal: Vec<D>,
    offset: f64,
}
```

をFacetGroup側に保持し、

```text
group plane
```

と

```text
triangle/simplex plane
```

を分離した方がよいです。

そうするとmerge判定も、

```text
plane distance
+
normal angular difference
```

で統一できます。

---

# 6. `Simplex` の `neighbors` の意味を明確にした方がいい

この構造は、

```rust
pub struct Simplex<const MAX_D: usize> {
    pub vertices: [u32; MAX_D],
    pub neighbors: [FacetId; MAX_D],
    ...
}
```

となっています。

数学的にはD次元simplexは

```text
D + 1 vertices
```

を持ちます。

例えば3Dならtetrahedronは4頂点です。

したがって、

```rust
vertices: [u32; MAX_D]
```

という定義は、**`D` の意味が「次元」なのか「頂点数」なのか曖昧**です。

もしD次元simplexなら、

```rust
vertices: [u32; D + 1]
neighbors: [FacetId; D + 1]
```

が基本になります。

ここは設計書の早い段階で修正した方がいいです。

---

# 7. Arenaの「lock-free」はかなり難しい

現在、

```rust
chunks
active_chunk: AtomicUsize
free_head: AtomicU64
```

というChunked Lock-free Generational Arenaを想定しています。

ただし、

**Arenaをlock-freeにすること自体が性能向上につながるとは限りません。**

むしろ今回のアルゴリズムでは、

```text
Batch selection
    ↓
disjoint horizon
    ↓
worker-local topology mutation
```

を作っているので、

**トポロジー更新をそもそも並列競合させない**

方が重要です。

つまり、

```text
global lock-free arena
```

より、

```text
global immutable/input state
        +
worker-local mutation
        +
barrier
        +
deterministic commit
```

の方が設計として綺麗になる可能性があります。

---

# 8. Prioritized Horizon Reservationは非常に面白いが、ここは要証明

個人的には、この設計で一番面白い部分です。

```text
Candidate points
↓
Sequential reservation
↓
non-overlapping horizons
↓
parallel commit
```

という考え方です。

ただし現在の文書では、

> Horizonが衝突しなければ安全

というところがまだ直感レベルです。

最低でも次を明文化した方がいいです。

### Invariant

Batch内の任意の2点 `Pi`, `Pj` について、

```text
VisibleFacets(Pi) ∩ VisibleFacets(Pj) = ∅
```

だけで十分なのか？

さらに、

```text
Horizon(Pi) ∩ Horizon(Pj) = ∅
```

も必要なのか？

さらに、

```text
new facets(Pi) ∩ neighborhood(Pj) = ∅
```

まで必要なのか？

ここを定義する必要があります。

**「Visible Facetsが重ならない」だけでは、生成される新しいridge同士が干渉しないことまで保証できるとは限りません。**

---

# 9. 決定論性（Deterministic）の仕様をもっと具体化すべき

冒頭で、

> Deterministic & Robust

を掲げています。

しかしRayon並列処理を入れると、

```text
thread scheduling
atomic allocation order
merge order
```

が結果に影響する可能性があります。

したがって、

```text
同一入力
+
同一設定
=
同一vertex set
+
同一facet topology
+
同一output ordering
```

のどこまでをdeterministicとするのか決める必要があります。

例えば、

### Level 1

```text
geometrically equivalent
```

### Level 2

```text
same topology
```

### Level 3

```text
same indices
```

### Level 4

```text
byte-for-byte identical
```

を分けると非常に分かりやすくなります。

---

# 10. Euler-Poincaréチェックは一般D次元ではもう少し注意が必要

テスト仕様で、

> `Σ (-1)^k F_k = 1 - (-1)^D`

を検証しています。

これは良い検証項目です。

ただし、**何を `F_k` として数えるか**を厳密に定義する必要があります。

今回、

```text
Triangulated Multi-facet
+
logical Facet Group
```

という独自トポロジーを使っているため、

```text
simplex count
```

と

```text
logical facet count
```

を混ぜるとEuler characteristicが壊れます。

したがって、

```text
Underlying simplicial complex
```

に対するEulerチェックなのか、

```text
Merged polyhedral complex
```

に対するEulerチェックなのかを分離すべきです。

---

# 11. Oracleの「Volume一致 1e-12」は危険

現在、

> Qhull / CGALの結果とvolumeを `10^-12` 精度で一致

としています。

これはテストとしては厳しすぎるというより、**テストの意味が曖昧**です。

別実装が正しくても、

```text
Qhull
vs
CGAL
vs
convx
```

で浮動小数点演算順序が違えば最後のbitは当然変わります。

したがって、

```text
absolute error
relative error
ULP distance
topological equivalence
```

を使い分けるべきです。

例えば、

```text
volume:
    relative tolerance

vertex set:
    exact

facet topology:
    canonicalized exact

geometric containment:
    predicate-based
```

のようにすると良いです。

---

# 12. DelaunayのUpper Hull pruningは、実装前に数学的条件を再整理したい

Lifting Map自体は非常に自然です。

```text
(x1,...,xD)
→
(x1,...,xD, Σxi²)
```

という方針です。

ただし、

> `N[D+1] > E_angle` ならUpper Hullとして探索を中断

という部分は、かなり慎重に設計した方がいいです。

Delaunayの判定では、

```text
lifted hull
+
orientation
+
lower hull selection
```

の符号規約を最初に固定し、

```text
normal convention
coordinate convention
orientation convention
```

を数学的に定義することを推奨します。

ここは符号を1箇所間違えると、

**Delaunayが「それっぽく動くが全部逆」という非常に発見しづらいバグ**

になります。

---

# 13. VoronoiがAPIに出ているのに仕様がほぼない

冒頭では、

> Convex Hull / Delaunay / Voronoi Engine

を目標にしています。

しかし実際の設計では、

```text
ConvexHull
DelaunayTriangulation
```

が中心で、

```text
Voronoi
```

のデータモデルがありません。

これはPhase 4で最低限、

```rust
pub struct VoronoiDiagram<const D: usize> {
    cells: ...
    vertices: ...
    ridges: ...
}
```

のような**出力モデルだけでも定義**しておいた方がいいです。

特に、

```text
infinite Voronoi cell
unbounded ridge
degenerate cell
coincident points
```

をどう表現するかは重要です。

---

# 14. APIはかなり良い

ここは素直に評価できます。

```rust
ConvexHullBuilder::new(dim, points)
    .tolerance(...)
    .parallel(...)
    .build()
```

というBuilder APIは扱いやすいです。

また、

```rust
&[f64]
```

をcore inputにする方針も、NumPy等とのFFIや、

```text
nalgebra
faer
ndarray
```

との接続を考えると合理的です。

---

# 15. ただしStatic APIのunsafeは不要に近い

現在、

```rust
let flat_ptr = points.as_ptr() as *const f64;
let flat_slice = unsafe {
    std::slice::from_raw_parts(flat_ptr, points.len() * D)
};
```

としています。

`[[f64; D]]` はメモリ上で連続しているので、この変換自体の意図は理解できます。

ただ、ライブラリAPIとしては、

```rust
points.as_flattened()
```

など、利用可能なRustバージョンで安全な方法を使えるなら、そちらを優先した方がよいです。

少なくとも、

```rust
unsafe {
    ...
}
```

の安全条件をコメントで明示することを推奨します。

---

# 16. `D <= 6` と内部 `D > 8` の関係が不自然

APIでは、

```text
Const Generics D <= 6
```

なのに内部solverでは、

```text
D > 8
```

まで扱う設計です。

つまり、

```text
7D
8D
```

がどちらにも明確に属していません。

ここは単純に、

```text
Static API: D <= 8
```

などにするか、

```text
D <= 6 is optimized API
D > 6 is dynamic API
```

と明記した方がいいです。

---

# 17. 「Qhullを速度・堅牢性・安全性すべてで凌駕」は目標としては良いが、検証可能な目標に分解すべき

冒頭の目標は、

> Qhullを速度・堅牢性・安全性のすべてで凌駕

です。

これはプロジェクトのモチベーションとしては良いのですが、仕様書としては、

```text
Benchmark target
Correctness target
Robustness target
Memory target
```

に分けた方が良いです。

例えば、

### Performance

```text
Qhull = 1.0x
convx target = ...
```

### Robustness

```text
random
adversarial
near-degenerate
large-coordinate
high-dimensional
```

### Memory

```text
bytes / input point
```

### Parallel scaling

```text
1 thread
2
4
8
16
```

という形です。

---

# 18. 実装ロードマップは良い。ただしPhase 1に「Predicate Kernel」を追加したい

現在のロードマップは、

```text
Phase 1
Arena
SIMD
Hyperplane solver

Phase 2
Sequential Quickhull
Facet merging

Phase 3
Parallelization

Phase 4
Delaunay/Voronoi
```

です。

大枠は良いです。

ただし私はPhase 1を、

```text
Phase 1
├── Geometry predicate kernel
│   ├── orientation
│   ├── determinant
│   ├── plane distance
│   ├── convexity
│   └── error bounds
│
├── Arena
├── SIMD distance kernel
└── Linear algebra
```

にしたいです。

**Predicate Kernelを最初に固定することが、このプロジェクトでは非常に重要です。**

---

# 優先順位を付けると

## P0 — 実装前に必ず修正

### ① 数値ロバスト性モデル

```text
Dynamic epsilon
```

だけに依存しない。

↓

```text
Robust geometric predicates
+
error bounds
```

を設計。

### ② Simplexの次元定義

```rust
vertices: [u32; MAX_D]
```

が本当に正しいか再確認。

D-dimensional simplexなら基本は `D+1`。

### ③ Facet Merge条件

```text
normal
+
distance
```

だけではなく、

```text
coplanarity
convexity
adjacency
supporting plane
```

を正式仕様化。

### ④ Parallel Batchの安全性

```text
VisibleFacet disjoint
```

だけで本当にmutation disjointになるのかを証明。

---

# P1 — Phase 1～2で決めたい

* Predicate Kernel
* deterministic semantics
* FacetGroupの代表hyperplane
* Euler characteristicの対象complex
* tolerance semantics
* degeneracy policy
* duplicate point policy
* infinite / unbounded Voronoiの表現

---

# P2 — 性能最適化として後回しでもいい

* AVX-512
* upper hull pruning
* lock-free arenaの高度な最適化
* virtual lifting coordinate
* aggressive SIMD specialization

この辺りは**正しいSequential Coreができてからでも遅くありません。**

---

# 私ならアーキテクチャをこう整理します

現在の4層に加えて、最下層に明確なPredicate層を置きます。

```text
┌─────────────────────────────┐
│ Public API                  │
│ ConvexHull / Delaunay       │
└─────────────┬───────────────┘
              │
┌─────────────▼───────────────┐
│ Topology Engine             │
│ Quickhull / Facet / Merge   │
└─────────────┬───────────────┘
              │
┌─────────────▼───────────────┐
│ Geometric Predicate Kernel  │  ← 最重要
│ orientation                 │
│ determinant                 │
│ convexity                   │
│ coplanarity                 │
│ error bounds                │
└─────────────┬───────────────┘
              │
┌─────────────▼───────────────┐
│ Numeric Kernel              │
│ SIMD / QR / determinant     │
└─────────────┬───────────────┘
              │
┌─────────────▼───────────────┐
│ Memory / Arena              │
└─────────────────────────────┘
```

こうすると、

**「高速な数値計算」**

と

**「正しい幾何学的判断」**

が明確に分離されます。

これは `convx` のようなライブラリではかなり重要です。

---

# 最終評価

この設計書は、単なる「RustでQuickhullを実装してみる」レベルではありません。
特に、

* Facet Mergingを最初から設計に含めている
* triangulated multifacetを採用している
* 並列化を後付けにしていない
* Arena + generational IDを考えている
* Delaunayをlifting mapで統合している
* invariant validatorを設計している

あたりはかなり良いです。

一方で、**現在の設計の最大の弱点は「幾何学的な正しさを何によって保証するのか」がまだDynamic Epsilon・Merge Rule・Invariantに分散していること**です。

ここを、

> **Robust Geometric Predicate → Topological Decision → Mutation**

という一本の設計思想に整理できれば、かなり強い設計になります。

特に次にやるなら、私は**「convx Robust Predicate / Facet Merge / Degeneracy仕様」の設計書を先に固める**ことを勧めます。ここが固まれば、その後のArena・Rayon・SIMD・Delaunayはかなり機械的に実装へ落とせます。
