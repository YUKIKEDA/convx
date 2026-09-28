レビューしました。結論から言うと、**設計の骨格はかなり強いですが、現状のまま実装に入ると Delaunay / Voronoi と凸包の退化処理で仕様上の矛盾にぶつかる可能性が高い**です。

特に重要なのは **P0 が2点、P1 が数点**あります。

## 総合評価

| 項目             | 評価       | コメント                                                              |
| ---------------- | ---------- | --------------------------------------------------------------------- |
| 数値述語の方針   | ◎          | exact predicate をトポロジーの根拠にする方針は非常に良い              |
| API設計          | ○          | 比較的明快。ただし代表インデックス等の仕様追加が必要                  |
| 凸包のトポロジー | ○          | Quickhull + logical facet の方向性は良い                              |
| 退化処理         | △          | 凸包単体は整理されているが、Delaunayとの接続で破綻する                |
| 並列化           | △          | アイデアは明確だが「なぜ逐次と同値か」の証明条件が不足                |
| Delaunay         | **要修正** | lifted hull を現在の ConvexHull API にそのまま渡せないケースがある    |
| Voronoi          | **要修正** | Delaunay の問題を引き継ぐほか、高次元の双対表現をもう少し厳密化したい |
| 検証             | ○          | 不変条件は良い。ただし退化ケースのテストが不足                        |
| 実装順           | ◎          | predicate → sequential → parallel → Delaunay/Voronoi は妥当           |

---

# 1. 【P0】Delaunay が現在の ConvexHull の入力制約と衝突する

これは最優先で修正した方がいいです。

現在は凸包について、

> 代表が D+1 個以上あってもアフィン次元が D 未満なら `DegenerateDimension` で失敗

としています。

一方 Delaunay は、

> D 次元の Delaunay は D+1 次元へ持ち上げた凸包の下側凸包

と定義されています。

ここに問題があります。

例えば **2次元で3点だけ**を考えます。

```text
A
|\
| \
|  \
B---C
```

これは完全に正当な 2D Delaunay triangulation です。

しかし paraboloid lift

```text
(x, y) -> (x, y, x²+y²)
```

後の3点は、3次元空間の中で**3点からなる2次元平面**しか張りません。

つまり、

```text
入力空間     : affine dimension = 2
lifted space : affine dimension = 2
ambient dim  : 3
```

となります。

現在の ConvexHull は3次元凸包として `D=3` を要求するため、

```text
DegenerateDimension
```

になります。

しかしこれは「退化した入力」ではありません。

**最小の Delaunay simplex は D+1 点であり、その lifted hull は ambient D+1 次元を張る必要がない**からです。

### さらに重大なのが共球

2Dで4点が同一円周上にあるケース：

```text
    A
 B     C
    D
```

は、Delaunay の典型的な退化ケースです。

4点を lift すると、4点が同一平面上に載ります。

つまり、

```text
original affine dimension = 2
lifted affine dimension   = 2
lifted ambient dimension  = 3
```

です。

ところが仕様では明確に、

> 共面・共球の分割が版ごとに変わり得る

ことを許容しています。

さらに Delaunay 側でも、

> 持ち上げた点の向きが厳密にゼロで、対角が複数あるときは、その版のアルゴリズムが選んだ分割を返す

としています。

つまり仕様上は**共球を正常系として扱うつもり**です。

ところが ConvexHull 側では、その時点で rank deficiency として失敗します。

### 推奨修正

Delaunay を

```text
Delaunay
  ↓
lift
  ↓
「一般の full-dimensional ConvexHull」
```

と単純に実装するのではなく、

```text
Delaunay core
  ├─ lifted affine hull を求める
  ├─ その affine hull 内で lower hull を構築
  └─ deterministic tie-break
```

という仕様にした方がいいです。

あるいは ConvexHull 自体を、

```rust
ConvexHullBuilder::build()
```

とは別に、

```rust
build_in_affine_subspace(...)
```

のような内部プリミティブとして設計する方法もあります。

---

# 2. 【P0】「厳密符号」と保存された `FacetPlane` の関係を明確にする必要がある

ここもかなり重要です。

仕様では、

> トポロジー判定は行列式で行い、法線を求めてから内積を取った符号を本体にはしない

としています。

これは正しい方向です。

ところが公開結果では、

```rust
pub struct FacetPlane {
    pub normal: Vec<f64>,
    pub offset: f64,
}
```

を持っています。

そして点分類では、

> ある論理ファセットへの距離が厳密に正

などと書かれています。

しかし `normal` と `offset` は `f64` で計算されています。

したがって、

```text
x · normal + offset
```

を計算したものを「厳密符号」として扱うことはできません。

### ここは用語を分離した方がいいです

例えば：

```text
predicate_value
    ↓
ExactSign
    ↓
topology
```

と、

```text
FacetPlane
    ↓
f64 representation
    ↓
display / geometry query / numerical calculation
```

を明確に分ける。

そして仕様として、

> `FacetPlane` の `normal` / `offset` はトポロジー判定の根拠ではない。

を明記するとかなり強くなります。

---

# 3. 【P1】「論理ファセットは一意」と「並列結果一致」の間に証明すべき部分がある

並列化の考え方はかなり良いです。

特に、

```text
T(P) ∩ T(Q) = ∅
H(P) ∩ H(Q) = ∅
```

を条件にして、`V` だけでなく `N` も予約するのは重要です。

ここは設計として評価できます。

ただし、

> 触る面が互いに素なので、この適用順はトポロジーを変えない

は、現時点では**仕様上の主張であって証明条件になっていません**。

特に Quickhull では、

```text
P を入れる
 ↓
新しい facet ができる
 ↓
Q の visible set が変わる
```

という依存があります。

したがって、

```text
T(P) ∩ T(Q) = ∅
```

だけで十分なのか、

```text
outside sets
horizon
new facets
point assignment
```

まで含めて独立なのかを明示した方がいいです。

### おすすめ

「並列バッチは可換」という内部不変条件を追加します。

例えば概念的には、

```text
apply(P, state)
apply(Q, state)
```

について、

```text
apply(Q, apply(P, state))
==
apply(P, apply(Q, state))
```

が成立する条件を定義する。

その条件を満たさない場合は同じバッチに入れない。

これなら parallel correctness がかなり説明しやすくなります。

---

# 4. 【P1】`representative` と `u32` の最大入力数を仕様化した方がいい

現在、

```rust
representative: Vec<u32>
```

です。

しかし入力数については明示的な上限がありません。

さらに、

> D >= 1 であり、上限は設けない

としています。

すると理論上、

```text
point index > u32::MAX
```

が発生できます。

これは API の仕様矛盾になります。

### どちらかにすべき

#### A

```text
n <= u32::MAX
```

を入力条件にする。

#### B

すべて `usize` にする。

個人的には内部 index と公開 index を分けて、

```rust
type PointIndex = u32;
type InternalIndex = usize;
```

のようにして、

> 公開 point index は u32 の範囲

を仕様にするのが扱いやすいと思います。

---

# 5. 【P1】`coplanar_points` の分類は「凸結合」の定義をもう一段明確にしたい

現在、

> 現在の面の極点集合の凸結合で表せない点は、その論理ファセットの頂点に加える

となっています。

数学的には方向性は分かります。

ただし実装仕様としては、

```text
point ∈ conv(facet.vertices)
```

の判定自体が数値問題になります。

ここも predicate philosophy に合わせて、

```text
facet containment
```

を何によって exact に決めるのかを定義した方がいいです。

特に D>=4 では facet 自体が simplex ではなく polytope なので、

```text
facet の supporting hyperplane 上にある
+
facet の内部にある
```

をどう判定するかが必要です。

---

# 6. 【P1】論理ファセットの adjacency は高次元で仕様をもう少し明確に

公開構造は、

```rust
pub struct LogicalFacet {
    pub vertices: Vec<u32>,
    pub plane: FacetPlane,
    pub neighbors: Vec<u32>,
}
```

です。

しかし `neighbors` について、

> 隣接ファセット番号。昇順

だけだと、D>=4では情報が不足する可能性があります。

「隣接」と言っても、

```text
A と B が隣接
```

だけなのか、

```text
A と B がどの ridge を共有しているか
```

まで必要なのかが曖昧です。

内部構築では ridge が必要なので、

```rust
neighbors: Vec<u32>
```

だけでは、

```text
neighbor facet
shared ridge
```

の対応が失われます。

もし公開 API で shared ridge が不要なら、

> `neighbors[i]` は単に隣接ファセット集合であり、対応する ridge は公開しない

と明記するとよいです。

---

# 7. 【P1】Voronoi の「ray = hull facet normal」は、向きの仕様をさらに厳密にしたい

現在、

> レイの方向は論理ファセットの外向き単位法線

となっています。

これは duality として自然です。

ただし Voronoi の unbounded face は、単に

```text
ray direction
```

だけでは高次元で完全な幾何情報にならないケースがあります。

例えば D=3 なら Voronoi の unbounded face は面・半直線などになります。

「Delaunay boundary simplex の双対が ray」という説明と、

```rust
VoronoiInterface {
    vertices,
    rays
}
```

の対応について、

```text
どの有限 face とどの ray が同じ Voronoi face を構成するか
```

をもう少し定義すると実装しやすくなります。

---

# 8. 【P1】Voronoi の外心計算は「有限である」だけでは不十分

外心計算について、

> 座標は f64、厳密値は保証しない。非有限なら `NonFiniteCircumcenter`

となっています。

ここは良いですが、

```text
finite
```

と

```text
correct enough to preserve topology
```

は別問題です。

幸い、Voronoi のトポロジーは Delaunay から決める方針なので、

> 外心座標の丸め誤差はトポロジー決定に使用しない

を明記すると、設計全体の思想と綺麗につながります。

---

# 9. 【P2】Euler characteristic の検証式は「何を数えるか」を固定したい

検証として Euler characteristic を使うのは非常に良いです。

ただし、

```text
F_k
```

が

```text
boundary complex の全 k-face
```

なのか、

```text
facet ごとの simplex decomposition
```

なのかを厳密に固定した方がいいです。

特に logical facet を導入しているため、

```text
triangle mesh
vs
polygonal boundary
```

で数え方が変わります。

仕様中でも「混ぜない」と書いているので、ここをさらに、

```text
F_k = unique k-dimensional faces of the boundary cell complex
```

のように定義するとよいです。

---

# 10. 【P2】`volume()` は D=1〜高次元まで同じ API で良いが、意味を明記したい

現在、

> 正規化した単体を辞書順に足した f64

となっています。

これは deterministic という意味では良いです。

ただし `f64` の足し算は、

```text
(a+b)+c
```

と

```text
a+(b+c)
```

で結果が変わります。

なので、

> 同一バイナリでは決定的

とするなら、

```text
summation order
```

まで仕様に含める必要があります。

今の「辞書順」はそこまで指定しているので、この点はむしろ良いです。

ただ、

```text
normalised simplex
```

が何を意味するかを定義すると完成度が上がります。

---

# 11. 良い点：述語設計はこのまま維持したい

ここはかなり良いです。

特に、

> 誤差は、その述語の式を評価したときの丸めに限る

> 上界以内は同じ多項式の厳密符号に落とす

> 悪条件であること自体は失敗にしない

という方針です。

これは geometric algorithm で非常に重要な、

```text
numerical approximation
        ≠
topological decision
```

を明確に分離しています。

さらに、

```text
SIMD distance
       ↓
fast candidate filtering

exact predicate
       ↓
topology
```

という構造も良いです。

**この部分は設計の中心思想として維持していいと思います。**

---

# 12. 良い点：Logical Facet を公開概念にしたのも良い

通常の Quickhull 実装では、

```text
triangle / simplex
```

をそのまま公開しがちですが、この設計では、

```text
construction simplex
        ↓
merge
        ↓
LogicalFacet
```

としています。

これは特に、

```text
4点以上の共面頂点を持つ面
```

を扱うときに合理的です。

公開結果を、

> 正規化した論理ファセットで比較する

とした方針とも一致しています。

---

# 13. 実装前に仕様へ追加したいテストケース

現在の検証項目に、以下を明示的に追加することを強く勧めます。

### Convex Hull

```text
D=1
  2 points
  duplicate points
  -0/+0

D=2
  triangle
  square
  square + edge points
  square + interior points
  all collinear
  duplicate-heavy

D=3
  tetrahedron
  cube
  cube + face-interior points
  cube + edge points
  coplanar input
  nearly coplanar input
```

### Predicate

```text
exact zero
1 ulp away from zero
huge coordinate
tiny coordinate
huge + tiny mixed scale
cancellation-heavy determinant
```

### Delaunay

特に重要なのが：

```text
D=1, exactly 2 sites
D=2, exactly 3 sites
D=2, square / cocircular
D=2, multiple cocircular points
D=3, exactly 4 sites
D=3, cospherical points
```

です。

**このセットを通せない限り、Delaunay の実装開始は待った方がいいです。**

---

# 14. 推奨する仕様変更の優先順位

私なら次の順番で直します。

### P0

**① Delaunay と ConvexHull の affine-rank の関係を再設計**

```text
full-dimensional ConvexHull
```

をそのまま lifted hull に使う設計をやめる。

特に、

```text
D+1 sites
cospherical sites
```

を正常系として扱えるようにする。

---

### P0

**② topology predicate と `FacetPlane` の数値表現を完全分離**

例えば仕様に、

```text
FacetPlane is a numerical representation only.
No topological decision may use its f64 dot product.
```

相当の規則を入れる。

---

### P1

**③ Quickhull parallel batch の可換条件を正式な不変条件にする**

```text
P/Q are independent
=> apply(P,Q) == apply(Q,P)
```

を定義。

---

### P1

**④ index width を確定**

```text
u32
```

を使い続けるなら、

```text
number of input points <= u32::MAX
```

を仕様化。

---

### P1

**⑤ coplanar point の「凸結合」を predicate ベースで定義**

---

### P1

**⑥ Voronoi の unbounded face の構造を高次元まで明文化**

---

# 15. 現時点での最大の懸念

この設計で一番危険なのは、**凸包をかなり厳密に設計できているため、そのまま Delaunay に適用できるように見えてしまうこと**です。

実際には、

```text
Convex Hull
    ↓
full-dimensional polytope

Delaunay
    ↓
lifted point set
    ↓
lower hull
    ↓
possibly lower-dimensional lifted hull
    ↓
cospherical degeneracy
```

なので、両者にはもう一段 abstraction が必要です。

特に仕様自身が、

> 共球の分割は一意ではない

と認識している一方で、凸包の入力段階では rank deficiency をエラーにしているため、ここが現在もっとも大きな設計上の不整合です。 

---

## 最終評価

**凸包部分：かなり良い設計**

特に、

```text
Exact predicate
    ↓
topology
    ↓
mutation
```

という優先順位、Logical Facet、deterministic normalization、parallel commit の考え方は筋が通っています。

ただし、

**Delaunay / Voronoi を含めた「ライブラリ全体の仕様」としては、まだ実装開始前に1回設計を戻した方がいい段階**です。

最大の修正ポイントは、

> **「Delaunay は lifted ConvexHull の単なる特殊ケースではなく、affine rank が ambient dimension に満たない lifted point set と共球退化を正常に扱える lower-hull 問題として定義する」**

ことです。

ここを解決すれば、かなり完成度の高い仕様になります。
