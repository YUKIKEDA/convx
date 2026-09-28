かなりよく整理されています。特に「**幾何述語の符号 → トポロジー → 変異**」を最上位の正しさの順序に置き、`f64` の数値計算とトポロジー判定を明確に分離している点は非常に良いです。

ただし、**このまま実装仕様として固定すると危険な箇所がいくつかあります**。特に重要なのは、

1. **Quickhull の「距離ゼロの点の分類」が一般次元では未定義に近い**
2. **並列 Quickhull のバッチ安全条件が十分条件になっていない**
3. **Delaunay の「持ち上げが平坦」の定義・処理が不十分**
4. **Voronoi の共球退化のまとめ方に数学的な問題がある**
5. **`NonFiniteFacetPlane` / `NonFiniteCircumcenter` の API 契約がやや不整合**
6. **変換不変性のテストに「負の一様スケール」が入っているのは要修正**
7. **Euler 特性の検査対象と式をもう少し厳密に定義した方がよい**

です。

以下、重要度順にレビューします。

---

# 1. 最重要：距離ゼロの点の分類が一般次元で危うい

この部分です。

> 距離ゼロの面では、頂点集合を \(V\) とする。点が \(\mathrm{conv}(V)\) の外側かどうかは、入力座標のままの orientation で決める。

そして、

> \(q\) は、\(V\) に入らない最小インデックスの極点である。リッジの内側にある頂点を \(a\) とする。点 \(p\) がそのリッジの外側にあるのは、...

これは **2D/3D の「辺に対してどちら側か」という直感を一般次元に拡張した部分が、まだ数学的に十分定義されていません**。

特に問題なのは、

> リッジの内側にある頂点を \(a\)

という部分です。

一般の \(D\) 次元論理ファセットは \((D-1)\) 次元凸多面体です。その境界には複数の \((D-2)\)-リッジがあります。しかし「そのリッジの内側にある頂点」という選択は、リッジが非単体的である場合には自明ではありません。

### ここはもっと単純化できます

そもそも `coplanar_points` の判定に、論理ファセットの「多面体内部にあるか」を独自に幾何判定する必要はありません。

支持超平面 \(H\) 上にある点 \(p\) について、

$$
p \in \operatorname{conv}(V)
$$

かどうかを、**その支持平面上の凸包への所属判定**として明示的に定義した方がよいです。

つまり、

> `p` がある論理ファセットの支持平面上にあり、かつそのファセットの頂点集合の凸包に属するなら `coplanar_points`

とする。

そしてその所属判定自体を、

* \(D=2\): 線分
* \(D=3\): 多角形
* 一般 \(D\): \((D-1)\)-次元 convex hull / half-space containment

として定義する。

現在の「リッジを一つ選び、orientation の符号を比較する」方式は、**実装上のアルゴリズムとして採用するなら、その数学的同値性を仕様で証明する必要があります**。

---

# 2. Quickhull の「非交差バッチ」はまだ安全性を保証していない

ここもかなり重要です。

> \(T(P) \cap T(Q) = \emptyset,\quad H(P) \cap H(Q) = \emptyset\)

さらに予定単体への orientation で衝突を落とす。

これは良い方向ですが、

> 予定単体のいずれかに対して Q が orientation で厳密に外側

だけでは、**P と Q の同時コミットが topology を壊さないことの十分条件になっているとはまだ言えません**。

特に Quickhull の concurrent insertion では、

* 可視領域
* horizon
* 新規ファセット
* 新規ファセット同士の共通 ridge
* 既存 facet の adjacency slot
* coplanar / outside set の再割当

まで含めて競合を考える必要があります。

現在の

```text
T(P) ∩ T(Q) = ∅
H(P) ∩ H(Q) = ∅
```

は**既存トポロジー上の予約競合**を避けているだけで、

> P を入れた結果できる新しい facet と Q を入れた結果できる新しい facet が衝突しない

ことを完全には表していません。

### 改善案

Phase 3 の仕様を、

> 「並列化の正しさは、同じ batch に含めた点を任意の順序で逐次適用した場合と、parallel commit の結果が同型になること」

まで引き上げるのがおすすめです。

さらに、

```text
batch admissibility
```

を独立した仕様概念にして、

```text
1. write-set conflict がない
2. read-set の topology が commit 前後で変化しない
3. 新規 simplex の ridge が相互に衝突しない
4. orientation predicate の符号が commit 順序によって変化しない
```

などに分けると、かなり強い仕様になります。

---

# 3. 「入力順を変えても同じ結果」と「代表インデックス」が衝突している

ここは仕様上の細かい矛盾です。

> 代表は最小の入力インデックス

としています。

一方、

> 入力順を変えたときは、代表インデックスを対応させてから比べる。

これはテストとしては問題ありません。

しかし、次のような仕様があります。

> 初期単体にどの点を選ぶかは、一般には仕様で固定しない。

さらに、

> 同じバイナリの中では、逐次と並列で同じ分割になる。

つまり、

* 入力順が変われば代表番号も変わる
* 辞書順もインデックス順に依存する
* pulling triangulation も代表インデックス順に依存する
* 初期単体もインデックス順に依存し得る

ので、**幾何学的な結果は入力順に依存しないが、退化時の combinatorial result は入力順に依存する**仕様になっています。

これは悪くありません。

ただし仕様として明文化した方がよいです。

例えば：

> 一般入力に対する幾何結果は入力順に依存しない。共面・共球による非一意な組合せ構造については、入力インデックスを tie-breaker として使用し、入力順を変更した場合には同じ幾何結果であっても異なる正規三角形分割を許容する。

この一文があるとかなり明確になります。

---

# 4. 「負の一様スケール」は変換不変性テストとして間違っている

これは明確に修正した方がいいです。

現在、

> 正の一様スケール、負の一様スケール

を試しています。

負のスカラー倍

$$
x'=-x
$$

は、単なる「スケール不変性」ではありません。

\(D\) 次元 orientation は

$$
\det(-I)=(-1)^D
$$

倍されます。

したがって、

* \(D\) 偶数 → orientation は不変
* \(D\) 奇数 → orientation は反転

します。

凸包そのものは中心反転で対応しますが、**orientation の符号規約、外向き順序、Delaunay simplex の向きなどはそのまま一致しません**。

したがってテストは、

```text
translation
positive uniform scaling
permutation of axes
input permutation
```

と、

```text
central inversion x -> -x
```

を別扱いにするべきです。

中心反転をテストするなら、

> 点集合を変換した後の結果を同じ変換で戻して比較する

必要があります。

---

# 5. `FacetPlane` の生成仕様に危険なところがある

ここです。

> 選んだ D 点から `f64` の法線を作り、凸包の内側が負になる向きへ揃え、単位長に正規化する。

しかし、

> 公開平面を作るときだけ、代表点への平行移動と、座標の幅によるスケールをしてよい。

となっています。

これは非常に良い考えですが、**「法線を作るアルゴリズム」と「法線の向きを決める方法」を分離すべき**です。

特に `f64` の法線生成が overflow / underflow した場合、

```text
normal -> NaN / inf
```

になってから正規化しても戻せません。

したがって仕様として、

```text
facet vertices
  ↓
translation
  ↓
scaling
  ↓
normal construction
  ↓
normalization
  ↓
orientation correction
  ↓
offset reconstruction
```

の順序を固定した方が安全です。

また、`offset` も

$$
offset=-p\cdot n
$$

で計算すると巨大座標で overflow する可能性があります。

「平行移動した座標系で offset を計算してから戻す」方法を仕様にした方がよいです。

---

# 6. `NonFiniteFacetPlane` は ConvexHull の成功/失敗設計と少し矛盾している

コメントでは、

> この平面はトポロジーに使わない。

としています。

しかし API は

```rust
pub fn build(self) -> Result<ConvexHull, ConvexHullError>;
```

です。

つまり平面を作れないだけで **凸包全体が失敗する**設計です。

これは必ずしも悪くありません。

ただ、

> 「トポロジー的には成功しているが公開表現を生成できない」

というエラーと、

> 「幾何構築そのものに失敗した」

が同じ `Result` に入っています。

ここは API 設計として、

```rust
ConvexHullError::NonFiniteFacetPlane
```

を残すなら、

> ConvexHull の topology は既に確定しているが、公開幾何表現の生成不能により build は失敗する

と明記するとよいです。

さらに Delaunay の

> NonFiniteCircumcenter は Delaunay を成功させる

という設計との差も明文化した方がいいです。

---

# 7. Delaunay の最大の問題：「持ち上げが平坦」の意味を整理した方がいい

ここは数学的に一番気になります。

> 全体が共球で持ち上げが平坦なときは、この符号を使わず、次の分割に入る。

そして、

> 持ち上げた点が \(R^{D+1}\) を張らなくても、そのアフィン包の中で下側包を作る。

これは正しい方向ですが、

**「flat lifting」と「共球退化」が混ざっています。**

Delaunay の lifting

$$
(x,\|x\|^2)
$$

では、\(D+1\) 個以上の点が同一 hypersphere 上にあると、持ち上げ点が同一 hyperplane に載ります。

ただし、

> 全体が共球

だからといって「持ち上げられたサイト集合の凸包が flat だから、通常の lower hull が定義できない」

というわけではありません。

むしろ、全点が同一球面上にある場合、lifting は \(D\)-次元 affine hyperplane 上に存在します。

この場合、**lower hull をそのまま取るのではなく、退化した regular triangulation を tie-break する必要がある**。

現在の

> pulling triangulation

はその解決策として良いです。

ただし、

> 「全体が共球なら pulling triangulation」

なのか、

> 「持ち上げ点の affine rank が D 未満なら pulling triangulation」

なのかを明確にしてください。

この二つは同じではありません。

---

# 8. Delaunay の pulling triangulation は、定義をもう少し厳密に

ここ：

> 最小インデックスの頂点を v とする。v を含まない凸包の境界面を、同じ規則で分割する。各単体に v を足して D-単体にする。

これは良いのですが、再帰の停止条件が必要です。

例えば、

```text
Pull(P):
    if P is simplex:
        return {P}

    v = minimum vertex
    H = facets of conv(P) not containing v

    for each H:
        Pull(H.vertices)
        cone every resulting simplex with v
```

のように定義すると、実装者が迷いません。

また、同じ facet が複数の親から生成されないことも保証した方がいいです。

---

# 9. Voronoi の「共球単体を一つの頂点へまとめる」は良いが、判定条件を変更した方がいい

現在：

> 有限な Voronoi 頂点は、持ち上げて同一超平面に載る Delaunay 単体を、推移的に一つへまとめたもの。

これは方向性として正しいです。

ただし、

> Delaunay simplex の adjacency graph 上で connected components を取る

だけでは、**「同じ Voronoi vertex」に対応する条件を明示できていません**。

より直接的に、

> 同じ Voronoi vertex に属する simplex は、その simplex の circumcenter が同一の exact affine solution に対応する

という定義を置く方が数学的には綺麗です。

ただし外心を exact 計算する必要はありません。

Delaunay simplex \(S_1,S_2\) が共有 ridge を持つなら、

$$
S_1 \cup S_2
$$

の全サイトが共球であるかを **lifting orientation の exact zero** で判定できます。

したがって、

```text
simplex adjacency
+
exact co-sphericity predicate
```

で equivalence class を作る、という仕様にすると明確です。

---

# 10. Voronoi interface の定義はもう少し見直した方がいい

ここ：

> 二つの異なる Voronoi 頂点がどちらもサイト a と b を含み、辺 ab が両方のセルの面であるとき、その interface は有限頂点を端に持つ。

これは良いです。

一方、

> 一つの頂点だけが a と b を含み、辺 ab が凸包の論理ファセット上にあるとき、その interface はレイを持つ。

この場合、

**有限 Voronoi interface が「一つの頂点 + 一本の ray」で表される**

という意味を明示した方がいいです。

今の型：

```rust
pub struct VoronoiInterface {
    pub sites: [u32; 2],
    pub vertices: Vec<u32>,
    pub rays: Vec<VoronoiRay>,
}
```

では、

```text
vertices = [v]
rays = [ray]
```

となることは読み取れますが、

```text
vertices = []
rays = [...]
```

のような状態を許すのかが仕様から完全には分かりません。

特に \(D=1\) では Voronoi interface は「点」なので、一般の定義をそのまま適用すると特殊ケースが増えます。

**D=1 を Voronoi だけでも独立に明示する**のがおすすめです。

---

# 11. `VoronoiCell.rays` の「境界上サイトなら少なくとも一本」は良い不変条件

これは非常に良いです。

> 凸包の境界上のサイトのセルには、少なくとも一つのレイが付く。

これを検証条件に追加するとさらに良いです。

また、

```text
interior site → rays.empty()
boundary site → !rays.empty()
```

を invariants として明文化できます。

---

# 12. Euler 特性は良いが、対象をもっと厳密に定義する

式

$$
\sum_{k=0}^{D-1}(-1)^kF_k=1-(-1)^D
$$

は、凸多面体境界の Euler characteristic として正しいです。

ただし、

> 論理ファセット複体についても、同じ式

については、

**LogicalFacet の頂点集合だけから「複体」を構成する方法を定義する必要があります。**

例えば 3D の cube なら、

```text
F2 = 6
F1 = 12
F0 = 8
```

で

$$
8-12+6=2
$$

です。

しかし LogicalFacet は polygon なので、その境界 edge をどう導出するかが必要です。

D≥4 ではさらに重要です。

したがって検証仕様に、

> LogicalFacet complex の \(k\)-face は、logical facet の vertex set の共通部分として得られる最大 proper faces から再帰的に生成する

などの定義が必要です。

---

# 13. `triangulation()` の「外向き」は少し矛盾している

ここ：

> 公開する各単体は、頂点を昇順に並べ、外向きになるよう末尾の 2 点だけを入れ替える。

これは実装しやすいですが、

**任意の昇順頂点列について、末尾2点の交換だけで外向きにできる**

という事実を仕様として使っています。

これはもちろん orientation の符号が ± のどちらかであれば成立します。

しかし、

> 共面領域の切り方は幾何的に一意とは限らず

なのに、

> 同じバイナリでは逐次と並列で同じ分割

としています。

ここは「三角形化の生成順序」と「orientation normalization」を明確に分離して、

```text
canonical simplex key = sorted vertex set
canonical orientation = parity-adjusted permutation
```

と定義した方がよいです。

---

# 14. `volume()` の定義は良いが、数値的にかなり危険

現在は、

> 境界単体との符号付き体積を f64 で足す

方式。

これは「仕様としての決定的な数値結果」を定めるという意味では良いです。

ただし巨大座標・微小座標を Phase 1 でサポートする一方で、

```rust
volume() -> f64
```

は overflow / underflow / cancellation が普通に起きます。

したがって、

> `volume()` はトポロジー判定には使用しない。有限入力でも有限結果を保証しない。

を追加した方がいいです。

おそらくこれは意図していると思いますが、明文化した方が安全です。

---

# 15. `u32::MAX` を sentinel にする設計は良い

これはかなり良いです。

> 点番号は `u32::MAX` 未満なので `u32::MAX` を欠番にする。

この設計なら、

```rust
neighbors: Vec<u32>
```

が軽くなります。

ただし、

```text
points.len() <= u32::MAX
```

なので、最大 `u32::MAX` 個の点が許されます。

点番号は

```text
0 .. u32::MAX-1
```

までなので sentinel と衝突しません。

この仕様は整合しています。

ただ、`usize -> u32` 変換時に overflow しないことを Phase 1 の入力 validation で明記しておくとよいでしょう。

---

# 16. 「次元上限なし」は理論的には良いが、実装上の契約をもう一つ追加したい

現在、

> D ≥ 1、次元上限なし。

これは良いです。

ただし、

```rust
points.len()
```

と

```text
i * D
```

の overflow を考える必要があります。

もちろん `points.len()` 自体が `usize` なので、

```rust
len % dim
```

の前に `dim == 0` を検査するのは正しいです。

しかし、

```rust
i * dim
```

については、

> 入力検証後に index arithmetic が overflow しないこと

を明示するか、slice の `chunks_exact(dim)` を使う仕様にした方が安全です。

---

# 17. `f64` の exact predicate の説明は非常に良い

ここは逆にかなり評価できます。

特に、

> 計算値と上界がともに 0 で、式が恒等的に 0 でないときも、厳密符号へ落とす。

と、

> アンダーフローで 0 になった値は、ゼロと確定しない。

は重要です。

また、

> FMA の丸めを含まないなら FMA への縮約は使わない。

も良いです。

ただし、もう一つだけ追加したいです。

### `-0.0` の述語結果

述語 API は、

```rust
Ordering
```

あるいは

```rust
Sign { Negative, Zero, Positive }
```

を返すべきで、内部で `f64` の `-0.0` を返す設計にはしない方がいいです。

つまり、

```text
predicate result:
  Negative
  Zero
  Positive
```

を数学的な primitive として仕様に置く。

これなら `-0.0` がトポロジーへ漏れません。

---

# 18. 「orientation は点数で次数が決まる」という表現は少し修正したい

ここ：

> 向きの次数は、渡した点の数で決まる。包の次元 D とは独立である。

これは数学的には正しい意図だと思いますが、少し誤解されます。

例えば、

$$
\operatorname{orient}(p_0,\ldots,p_k)
$$

は \(k\) 次元 affine independence predicate です。

なので、

> orientation の幾何次数は引数の点数 \(k+1\) により決まり、現在構築している hull dimension D とは独立した primitive として定義する。

くらいにした方が明確です。

---

# 19. 「共面」と「orientation zero」を統一した方がよい

仕様全体では、

```text
距離 zero
共面
orientation zero
coplanar
co-spherical
```

がかなり頻繁に出てきます。

ここは predicate taxonomy を一度作った方がいいです。

例えば：

```text
orientation(k+1 points)
    → sign of affine determinant

coplanar(points)
    → orientation == Zero

side_of_facet(simplex, p)
    → orientation(simplex.vertices, p)

cospherical(D+2 points)
    → lifted orientation == Zero

distance_sign(facet, p)
    → same sign as side_of_facet
       but may use filtered numeric evaluation
```

こうすると「何が primitive で、何が derived predicate なのか」が非常に明確になります。

---

# 20. `FacetPlane` の距離符号をトポロジーに使わない方針は正しい

これはかなり良い判断です。

> `FacetPlane` の \(x\cdot n + offset\) は判定に使わない。

公開用の normalized plane は、

* overflow 回避
* SIMD
* visualization
* distance query

などのための **representation** と割り切り、

topology はすべて determinant/orientation にする。

これはライブラリの設計思想として非常に強いです。

この原則は冒頭で、

> **公開数値表現は topology authority ではない**

と一文で固定しておくとさらに良いです。

---

# 21. `faer` の QR を topology から隔離しているのも良い

> QR が作るのは単位法線だけ。

これも良いです。

ただし、

> `faer` が計算した法線の向きが orientation と一致する

ことを保証するために、最後の sign correction は必ず exact predicate 側で行う、と明記するとさらに安全です。

つまり、

```text
QR:
    approximate normal

orientation:
    authoritative orientation

normal sign:
    orientation に合わせる
```

です。

---

# 22. Delaunay の下側判定は「先頭頂点 + height +1」の定義が少し特殊

ここ：

> 試験点は、その順の先頭頂点から、持ち上げ座標だけ +1 した点。

数学的には、

$$
p' = (x_0,\|x_0\|^2+1)
$$

を使って hyperplane の side を調べるということですね。

これは下側/上側を orientation で定義する方法として成立します。

ただし仕様書では、

> なぜ任意の +1 でよいのか

を一言書いておくと読み手に優しいです。

理由は、

> facet hyperplane に対する法線の符号を判定するだけなので、基準点から正の vertical direction に任意量移動した点でよく、1 は固定値として採用する。

です。

---

# 23. Delaunay の「平坦でないが一部共球」の処理は概ね良い

ここ：

> 一部の点だけが共球なときの対角は、通常の下側包の手順で決まる。

これは良いです。

つまり、

```text
global degeneracy → pulling
local cospherical degeneracy → regular lower hull's combinatorial tie-break
```

という設計ですね。

ただし、

> 共球の場合の対角はアルゴリズム依存

と明示するとより明快です。

---

# 24. Qhull 比較について

ここは注意点があります。

> Qhull を基準に測る。

性能測定としては良いですが、**Qhull を正しさの oracle としない**方がいいです。

現在はすでに、

> オラクルは `tests/fixtures/`

となっているので問題ありません。

さらに、

```text
Qhull = performance baseline
exact predicate / independent oracle = correctness
```

と明示すると完璧です。

また Qhull は通常、実数計算上の degeneracy 処理に独自の方針を持つため、

> 論理ファセット比較

をするなら、Qhull 側の coplanar / joggled / triangulated output の扱いを明示する必要があります。

---

# 25. Phase 分割はかなり良い

これは設計として非常に良いです。

特に、

> Phase 1 = predicate correctness

を最初に置いているのは正しいです。

そして、

```text
Phase 1
  predicates

Phase 2
  sequential topology

Phase 3
  parallel topology

Phase 4
  static API + Delaunay + Voronoi
```

という順番も良い。

むしろ一つ提案すると、Phase 1 をさらに

```text
1A: orientation
1B: filtered exact sign
1C: facet side
1D: lifted orientation / insphere
1E: numerical representation
```

に分けると、Delaunay まで行く前に問題を切り分けやすくなります。

---

# 26. 私なら仕様に「不変条件」を追加する

現在の検証項目に以下を追加します。

### Hull

```text
H1. every facet has exactly D vertices
H2. facet vertices are unique and sorted
H3. every neighbor is symmetric
H4. every shared ridge has exactly two incident simplices
H5. every logical facet has one support plane
H6. every vertex belongs to at least D facets
H7. every non-vertex representative belongs to exactly one classification
```

### Delaunay

```text
D1. every simplex has D+1 distinct sites
D2. every simplex is positively oriented or exactly zero only under specified degeneracy
D3. neighbors are symmetric
D4. neighboring simplices share exactly D vertices
D5. no site lies strictly inside the circumsphere of a Delaunay simplex
```

最後の D5 は、実装の internal predicate で検証するのが重要です。

### Voronoi

```text
V1. every Voronoi vertex has >= D+1 sites
V2. every finite vertex corresponds to one cospherical class
V3. cell incidence is symmetric
V4. interface incidence is symmetric
V5. interior cells have no rays
V6. boundary cells have >= 1 ray
```

---

# 27. 最大の構造的改善案

この仕様を実装仕様としてさらに強くするなら、私は全体を次の4種類に明確に分けます。

```text
                    ┌────────────────────┐
                    │  Public geometry   │
                    │ FacetPlane / f64   │
                    └─────────▲──────────┘
                              │
                    representation only
                              │
┌───────────────┐     ┌───────┴────────┐
│ Exact         │────▶│ Topology        │
│ predicates    │     │ Quickhull       │
│               │     │ Delaunay        │
│ orientation   │     │ Voronoi         │
│ insphere      │     │                 │
└───────────────┘     └───────▲────────┘
                              │
                    ┌─────────┴────────┐
                    │ Numeric helpers  │
                    │ QR / SIMD / f64  │
                    └──────────────────┘
```

そして仕様上の最重要原則を、

> **Topology shall never depend on an approximate geometric representation.**

とする。

日本語なら、

> **トポロジーは、公開用の近似幾何表現から決して導出しない。トポロジーを決定する唯一の根拠は、仕様で定義された厳密符号である。**

です。

これはこのライブラリの設計思想を非常に綺麗に表します。

---

# 総合評価

現状は **「かなり実装に入れるところまで来ているが、Delaunay/Voronoi と一般次元 coplanar 処理にはまだ仕様上の穴がある」** という評価です。

ざっくり分けると：

| 領域                     | 状態                                               |
| ------------------------ | -------------------------------------------------- |
| exact predicate の思想   | **非常に良い**                                     |
| `f64` と topology の分離 | **非常に良い**                                     |
| 入力/重複/代表管理       | **良い**                                           |
| Quickhull の基本方針     | **良い**                                           |
| LogicalFacet             | **良いが一般次元の所属判定を要明確化**             |
| arena / generation       | **良い**                                           |
| parallel Quickhull       | **まだ仕様強化が必要**                             |
| Delaunay lifting         | **方向性は良いが flat case の定義を整理**          |
| pulling triangulation    | **良いが再帰定義を明文化推奨**                     |
| Voronoi                  | **方向性は良いが共球 class の定義を強化推奨**      |
| 公開 API                 | **概ね良い**                                       |
| 検証                     | **かなり良い**                                     |
| 性能評価                 | **良い。Qhull は baseline と明記するとさらに良い** |

特に、**「数値誤差を許容して topology を決める」のではなく、「exact sign を topology authority として、その周辺を全部最適化する」**という設計は、`convx` の核としてかなり筋が良いです。

一方で、実装開始前に私なら必ず修正するのは次の **5点**です。

1. **一般次元の coplanar point 判定を正式定義する**
2. **parallel batch の十分条件を定義する**
3. **Delaunay の flat lifting / cospherical degeneracy を分離して定義する**
4. **Voronoi vertex equivalence の exact な定義を追加する**
5. **負の一様スケールを通常の不変性テストから外す**

この5つを固めれば、かなり強い仕様になります。

特に今回の仕様は「実装を書いてから辻褄を合わせる」タイプではなく、**predicate → topology → mutation の依存関係を先に固定している**ので、その方針は維持した方がいいです。
