レビューしました。結論からいうと、**設計の骨格はかなり良い**です。特に、

* exact predicate をトポロジーの唯一の根拠にする
* `FacetPlane` の `f64` をトポロジーから切り離す
* Quickhull 内部は simplicial complex、公開時に logical facet へ正規化する
* Delaunay は lifting の定義から作る
* 並列化を「速くする」より先に「逐次と同じトポロジーを得る」こととして定義する

という方針は一貫しています。以前整理していた「Delaunay の退化時も lifted affine hull 内で扱う」「Voronoi の外心近似をトポロジーに使わない」という方針とも整合しています。

ただし、**実装開始前に仕様を修正した方がいい箇所が5つあります**。そのうち3つは correctness blocker です。

---

# 1. 最重要: `coplanar_points` の分類規則が間違っている

ここです。

> その面のアフィン包の中で、面の頂点だけを入力にした凸包を考える。
> 点がそのどの部分面に対しても厳密に外側でなければ、`coplanar_points` に入れる。
> **いずれかの部分面の外側なら、その論理ファセットの頂点に加える。**

これはそのままだと誤分類します。

例えば 2D の正方形を2三角形に分割しているとします。

```text
A------B
|\     |
| \    |
|  \   |
|   \  |
|    \ |
D------C
```

境界上の点ではなく、正方形の内部点 `P` は、ほぼ必ずどちらか一方の三角形の外側です。

例えば

```text
triangle ABC
```

から見ると `P` が外側で、

```text
triangle ACD
```

から見ると内側、ということが普通に起きます。

したがって、

> 「いずれかの部分面の外側」

を頂点追加の条件にしてはいけません。

### 正しくは

論理ファセットの頂点集合 `V_F` に対して、

> **点 `p` が `conv(V_F)` の外側なら、その点は logical facet の頂点候補になる。**

です。

つまり、部分単体ではなく、

```text
facet vertex set
       ↓
affine hull
       ↓
(D-1)-dimensional convex hull
       ↓
p がその convex hull の外側か
```

を判定する必要があります。

この判定は、実質的には「facet 内での低次元 convex hull」です。

### 仕様を書き換えるなら

例えば、

> 距離ゼロの論理ファセット `F` がある場合、そのファセットの頂点集合 `V(F)` が張るアフィン空間へ制限した `(D-1)` 次元凸包を考える。
> `p` がその凸包の内部または境界上なら `coplanar_points` に入れる。
> `p` がその凸包の外側なら、`F` の極点集合へ `p` を追加する。

のようにした方が正確です。

これはかなり重要です。

Qhull でも coplanar/non-simplicial facet を別途扱っていますが、Qhull の「coplanar」の意味自体は独自の数値閾値を含むため、今回の exact-zero ベースの定義とは別物です。([Qhull][1])

---

# 2. 最重要: 並列バッチ条件がまだ「仕様」になっていない

ここも非常に重要です。

```text
T(P) ∩ T(Q) = ∅
H(P) ∩ H(Q) = ∅
```

に加えて、

> 適用順を入れ替えても単体複体が一致する組だけ

としています。

これは数学的には正しい方向ですが、**実装仕様としては未定義**です。

つまり、

> 「どうやって一致すると判定するのか？」

がありません。

しかも、

```text
T(P) ∩ T(Q) = ∅
```

だけでは、新しく作られた facet を相手の点が見る可能性を排除できません。

例えば、

```text
state
 ├─ P の cavity
 └─ Q の cavity
```

が現在は独立でも、

```text
P を適用
 ↓
P が生成した新 facet
 ↓
Q がその新 facet を見る
```

という依存があり得ます。

あなた自身が、

> 一つの非可視面が、二つのホライズンの向こう側になることがある。

と認識しているので、ここはかなり惜しいです。

## 推奨

Phase 3 の仕様を、

### Candidate

まず、

```text
T(P) ∩ T(Q) = ∅
H(P) ∩ H(Q) = ∅
```

で候補を作る。

### Conflict validation

その後、

```text
new_facets(P) に Q が outside か
new_facets(Q) に P が outside か
```

を exact predicate で確認する。

さらに必要なら、

```text
new_facets(P) が Q の cavity を変えるか
new_facets(Q) が P の cavity を変えるか
```

を確認する。

### Commit

conflict がなければ同一 batch。

という二段階にした方がよいです。

もっと形式的に、

$$
Apply(P, Apply(Q,S))
=
Apply(Q, Apply(P,S))
$$

を batch compatibility の定義として、

**実装ではこの等式を直接比較せず、十分条件となる conflict predicate を定義する**

とすると綺麗です。

「可換性」という数学的仕様と「実装可能な判定」を分離できます。

---

# 3. 最重要: Delaunay の「共球・平坦 lifting」の扱いがまだ未完成

ここは設計の思想自体は正しいです。

> 持ち上げた点が `R^(D+1)` を張らなくても、そのアフィン包の中で下側包を作る。

これは必要です。

ただし、次が曖昧です。

> 全体が共球で持ち上げが平坦なときも、サイト凸包の分割として成功する。

「平坦なときに何を triangulate するのか」が未定義です。

例えば2Dの正方形：

```text
A----B
|    |
|    |
D----C
```

4点は共円です。

Delaunay triangulation は

```text
A-B-C + A-C-D
```

でも

```text
A-B-D + B-C-D
```

でも正しい。

実際、共円・共球退化では Delaunay triangulation は一意ではありません。CGAL もこの場合を明示的に非一意な退化として扱い、実装では別途 tie-breaking をしています。([CGAL][2])

現在の仕様は、

> その版のアルゴリズムが選んだ分割

と言っているだけなので、**選ぶアルゴリズムが仕様化されていません**。

これは Phase 4 で詰まる可能性が高いです。

### 私ならこうします

Delaunay を

> lifted convex hull の **non-simplicial lower facets を exact predicate により構築し、その後 deterministic triangulation を行う**

と二段階にします。

つまり、

```text
sites
 ↓
lift
 ↓
exact lower convex hull
 ↓
non-simplicial lower facet
 ↓
deterministic triangulation
 ↓
Delaunay simplices
```

です。

そして triangulation rule を、

> lexicographically smallest valid triangulation

などと固定する。

ただし、高次元で「lexicographically smallest triangulation」を毎回求めるのは重いので、実装上は incremental insertion の deterministic tie-break でも構いません。

重要なのは、

**「何を基準に diagonal を選ぶか」を仕様にする**

ことです。

Qhull でも non-simplicial facet と triangulated output は別概念として扱われています。([Qhull][3])

---

# 4. そしてその問題が Voronoi に直結する

現在、

> Voronoi 図は、第7節の Delaunay の双対である。

とあります。

これは一般位置では綺麗ですが、**共球退化ではそのままでは真の Voronoi diagram になりません。**

例えば正方形。

Delaunay が対角 AC を選ぶと、

```text
triangle ABC
triangle ACD
```

の両方の外心は同じ中心 O です。

現在の仕様だと、

```rust
VoronoiVertex {
    coords: O,
    simplex: [A,B,C]
}

VoronoiVertex {
    coords: O,
    simplex: [A,C,D]
}
```

という**同じ幾何学的位置に2つの Voronoi vertex**ができます。

これは「Delaunay triangulation の graph-theoretic dual」としては理解できますが、通常の意味での Voronoi diagram の頂点とは違います。

実際、Delaunay の退化をそのまま dualize すると、幾何的に退化した Voronoi feature が出ることが知られています。CGAL も triangulated Delaunay の dual ではこの問題を別途扱っています。([CGAL][4])

Qhull も cospherical points の triangulation は Voronoi vertex を重複させ得るため、Voronoi では別扱いにしています。([Qhull][1])

## ここは API の意味を決める必要があります

二択です。

### A. 真の Voronoi diagram

同じ外心を exact predicate で同一視して、

```text
VoronoiVertex
    ↓
incident sites = D+1以上
```

にする。

こちらなら幾何学的な Voronoi 図です。

### B. Delaunay dual complex

`VoronoiVertex` は Delaunay simplex に1個。

つまり、

> `VoronoiDiagram` は true Voronoi geometry ではなく、選択された Delaunay triangulation の dual representation である

と明記する。

現在の文章は **A と B が混ざっています**。

私はライブラリ名が `VoronoiDiagram` なら A の意味を期待される可能性が高いので、ここは仕様として決めた方がいいです。

---

# 5. `FacetPlane` の数値仕様が足りない

ここも実装時にかなり問題になります。

```rust
pub struct FacetPlane {
    pub normal: Vec<f64>,
    pub offset: f64,
}
```

そして、

> 選んだ D 点から `f64` の法線を作り、凸包の内側が負になる向きへ揃え、単位長に正規化する。

とあります。

しかし、

```text
huge coordinates
tiny coordinates
huge + tiny mixed
```

を完了条件にしている以上、

```text
法線計算
→ norm
→ normalize
→ offset
```

で overflow / underflow / cancellation が起こります。

例えば座標差が

```text
1e308
```

なら、そのまま dot/cross 的な計算をすると簡単に `inf` になります。

一方で、

> 述語では translation / scaling しない

という制約は**そのまま維持していい**です。

これは、

```text
predicate
    ↓
raw input polynomial
```

と、

```text
display plane
    ↓
numerically stabilized computation
```

を明確に分ければ解決できます。

例えば仕様に、

> `FacetPlane` の生成では、述語の定義に影響しない範囲で平行移動・スケーリングを行ってよい。これは公開 plane の数値表現を安定化するためだけであり、トポロジー判定には使用しない。

を追加するとよいです。

そして、

```rust
FacetPlane {
    normal: finite,
    offset: finite,
}
```

を成功条件にするのか、それとも `NaN/inf` を許すのかも決めるべきです。

個人的には**公開 API で単位法線を返すなら finite を成功条件にした方がいい**と思います。

その場合、

```rust
NonFiniteFacetPlane
```

のようなエラーを追加するか、内部の安定化アルゴリズムで必ず回避する必要があります。

---

# 6. `volume()` の仕様が曖昧

ここはかなり気になりました。

> 単体分割は別ビューで出す。
> 各単体の体積は、外向きに揃えた頂点から計算した非負の `f64`。
> その順に左から足す。

「各単体」が何を指すかが曖昧です。

現在の `Simplex` は、

```rust
vertices: Vec<u32> // 長さ D
```

なので、これは **facet simplex** です。

一方、D次元の体積を求める simplex は、

```text
D + 1 vertices
```

必要です。

例えば3Dなら、

```text
facet = triangle = 3 vertices
volume simplex = tetrahedron = 4 vertices
```

です。

したがって、

> `triangulation()` が返すのは facet triangulation なのか、polytope interior の D-simplex decomposition なのか

を明確にしてください。

`volume()` を実装するなら、例えば

```text
interior reference point r
+
各 boundary facet simplex
→ D-simplex
```

という decomposition が自然です。

ただし、その場合 `r` の選び方も仕様に影響します。

なお、単に facet の面積を足すだけでは volume にはなりません。

---

# 7. `rank` / `spanning_points` の仕様をもう一段厳密にしたい

ここは方針は正しいです。

> `spanning_points` はアフィン独立な代表点のインデックスで、長さは actual_dim + 1、辞書順で最小の列

ただし、

**「辞書順で最小の列」**

の意味をアルゴリズムとして明確にした方がいいです。

例えば、

```text
points = p0 p1 p2 p3 ...
```

に対して、

```text
[p0,p1,p2]
```

が独立ならそれが採用。

そうでなければ次の組。

という意味なら、

> インデックス集合を辞書順に列挙し、最初にアフィン独立となる集合を返す

と明記すれば十分です。

ただし高次元では全組合せ列挙は当然使えないので、

```text
incremental basis construction
```

で得られる結果が「辞書順最小集合」と一致するようにアルゴリズムを定義する必要があります。

ここは Phase 1 の exact rank predicate とセットにするべきです。

---

# 8. exact predicate の「式」をもう少し固定した方がいい

現在、

> 同じ多項式の厳密符号

という設計は非常に良いです。

ただし、

> 厳密評価のアルゴリズムは仕様で固定しない。

との組み合わせには少し注意が必要です。

例えば determinant は、

```text
det(A)
```

という数学的多項式として定義するのか、

```text
Leibniz expansion
```

なのか、

```text
Bareiss
```

なのか、

```text
fraction-free elimination
```

なのか。

これは**厳密結果だけ見れば同じ**です。

したがって、

> 「数学的に定義された polynomial の exact sign」

とする現在の思想は正しい。

ただし `f64` は IEEE-754 binary64 なので、入力値は二進有理数です。したがって exact evaluator は実質、

```text
sign of exact rational/integer polynomial evaluated at binary64 values
```

です。

ここを仕様用語として、

> 入力 `f64` をその binary64 value が表す exact dyadic rational と解釈する。

と1行入れておくと、かなり強い仕様になります。

特に、

```text
-0.0 == +0.0
```

と、

```text
exact polynomial evaluation
```

の関係も明確になります。

---

# 9. 「distance predicate」と `FacetPlane` の関係をさらに整理したい

現在、

> `FacetPlane` の x·n + offset は、判定に使わない。

これは非常に良いです。

ただし、

```text
距離符号
```

という言葉と、

```text
orientation
```

という言葉が混在しているので、API/内部型として分けるとさらに安全です。

例えば、

```rust
enum Sign {
    Negative,
    Zero,
    Positive,
}
```

に対して、

```rust
orientation(...)
signed_distance_sign(...)
ridge_convexity(...)
insphere(...)
```

と全部「predicate」として扱う。

そして、

```rust
f64_distance(...)
```

は明確に

> numerical estimate only

とする。

そうすると、

```text
SIMD distance
       ↓
filter
       ↓
uncertain
       ↓
exact predicate
```

という設計がコード上でも強制できます。

---

# 10. `Delaunay` の lower hull predicate は符号規約をもう一段固定したい

ここ：

> 下側かどうかは、持ち上げた行列式から高さの列を除いた小行列式の厳密符号で決める。

は少し危険です。

`minor` だけでは、

```text
どの行順
どの列順
どの cofactor sign
```

なのかが曖昧です。

Delaunay の `insphere` は lifted orientation として扱う方針で問題ありません。実際、3D の insphere predicate を4次元 lifting の orientation として扱うのは標準的な考え方です。([数学ユーザー][5])

なので、

> `LiftedOrientation(p0,...,pD+1)` の引数順を固定し、その符号と lower/upper の対応を定義する。

まで書いた方がいいです。

例えば、

```text
det
[
  x0 ... xD  ||x||² 1
  ...
]
```

のように行列自体を仕様にする。

これなら後から式を最適化しても、

```text
optimized expression == this determinant
```

という検証ができます。

---

# 11. `NonFiniteCircumcenter` は少しだけ設計を再検討したい

これは悪くないのですが、

> 凸包と Delaunay はこのエラーを返さない。

という仕様はかなり意識的なので、その理由を一言書いておくといいです。

Delaunay の topology は exact predicate だけで確定し、

```text
circumcenter
```

は Voronoi の表示座標生成にしか使わないからです。

つまり、

```text
Delaunay = exact topology
Voronoi = Delaunay topology + finite numerical geometry
```

ということですね。

この分離は非常に良いです。

むしろ仕様に明示的に、

> `NonFiniteCircumcenter` は幾何学的退化を表さない。Delaunay topology は既に確定しており、Voronoi 座標を有限な `f64` として生成できなかったことだけを表す。

と書くと API の意味がかなり明瞭になります。

---

# 12. `VoronoiInterface` の定義は高次元で要注意

```rust
pub struct VoronoiInterface {
    pub sites: [u32; 2],
    pub vertices: Vec<u32>,
    pub rays: Vec<VoronoiRay>,
}
```

ここで「interface」が何を表すのかを固定した方がいいです。

2Dなら、

```text
2 sites
→ Voronoi edge
```

なので自然。

3Dなら、

```text
2 sites
→ Voronoi face
```

です。

4D以上では、

```text
2 sites
→ (D-1)-dimensional Voronoi face
```

になります。

したがって `vertices` + `rays` だけで本当に任意次元の `(D-1)` polyhedron を表現できるのか、かなり慎重に決める必要があります。

特に D≥4 では、あなた自身が convex hull の logical facet について

> 閉路は定義しない

としているので、Voronoi interface も単なる「頂点集合＋ray集合」では位相構造が不足する可能性があります。

もし保存形式として

```text
vertices + rays
```

だけにするなら、

> これは完全な combinatorial representation ではなく、幾何学的 feature の incidence の一部だけを返す

と定義する必要があります。

---

# 13. `neighbors` の定義はかなり良い。ただし D=1 を明文化

```rust
neighbors: Vec<u32>
```

を

> 隣接ファセット番号の集合

としたのは良いです。

特に、

> 配列の位置は共有リッジと対応しない

としているので、公開 API がかなり扱いやすい。

ただし D=1 では ridge dimension が

$$
D-2=-1
$$

になります。

したがって、

```text
D=1:
facets = 2 endpoints
neighbors = []
```

を検証規則にも追加した方がいいです。

---

# 14. Euler characteristic の検査は良い。ただし「どの complex か」をもっと強調

この式：

$$
\sum_{k=0}^{D-1}(-1)^kF_k=1-(-1)^D
$$

は、D次元凸多面体の境界球面の Euler characteristic と一致しているので正しいです。

例えば、

```text
D=2: V-E = 0
D=3: V-E+F = 2
D=4: V-E+F-C = 0
```

です。

ここで仕様に、

> `F_k` は境界複体の k-face の個数であり、入力点、coplanar point、Delaunay simplex などを直接数えない。

と書くとさらに良いです。

特に logical facet と simplicial facet を混ぜないという注意は重要です。

---

# 15. テスト項目に「predicate metamorphic test」を追加したい

現在の Phase 1 のテストはかなり良いですが、exact predicate にはもう一つ欲しいです。

例えば orientation なら、

```text
swap two vertices
→ sign flips

cyclic permutation
→ sign preserved/appropriate parity

duplicate vertex
→ zero

translate all points
→ exact sign unchanged
```

です。

ただし最初の仕様に、

> predicate の前に平行移動・scale はしない

とあるので、これは**実装内部で変換するという意味ではなく、テストとして別入力を与える**ものです。

例えば、

```text
orientation(P0,P1,P2)
```

と

```text
orientation(P0+C,P1+C,P2+C)
```

が同じ符号になることを確認する。

これで exact evaluator と filter のバグをかなり発見できます。

---

# 16. 「巨大座標・微小座標」は絶対値だけではなく exponent range を振ると良い

現在、

> 巨大座標、微小座標、巨大と微小の混在

となっています。

これは良いですが、具体的には例えば、

```text
2^-1022
2^-500
2^-100
1
2^100
2^500
2^1023
```

あたりを意識するといいです。

ただし、

```text
2^1023 + 2^1023
```

のように intermediate が overflow するケースと、

```text
input itself is finite
```

を明確に分ける必要があります。

今回の仕様では、

> 入力は finite なら受け付ける

なので、

```text
input finite
predicate intermediate overflow
```

は**失敗してはいけない**。

これは Phase 1 の重要なテストになります。

---

# 17. `u32::MAX` は仕様としては正しいが、実装上の allocation overflow を別に考える

```rust
TooManyPoints { actual }
```

を `u32::MAX` 超過に限定するのは合理的です。

ただし、

```text
points.len() <= u32::MAX
```

でも、

```rust
representative = vec![...; n]
```

が allocation failure になることはあります。

仕様では exact evaluator の allocation failure は

```rust
ExactEvaluationExhausted
```

ですが、通常の arena / output allocation failure はどうするかが未定義です。

Rust の `Vec` は通常 allocation failure で abort するので、ライブラリとして recoverable error にしたいなら別設計になります。

ここは今の段階では、

> 通常のコンテナ allocation failure は Rust の標準的な allocation failure semantics に従う。exact evaluation の作業領域だけは recoverable に扱う。

など、意図を明文化するとよいです。

---

# 18. Qhull 比較で注意したい点

性能基準として Qhull を使うのは良いですが、**同じ入力に対して完全に同じ問題設定を比較する必要があります**。

Qhull は、

* merge
* joggle
* triangulated output
* coplanar handling
* scaling

などに複数の選択肢があります。([Qhull][3])

今回の `convx` は、

```text
exact predicate
no input perturbation
logical non-simplicial facets
```

なので、例えば Qhull の `QJ` を基準にしてはいけません。

ベンチマーク仕様として、

```text
Qhull options:
  no joggle
  no input perturbation
  same dimensionality
  comparable output semantics
```

を固定しておいた方がいいです。

---

# 19. 設計として特に良いところ

逆に、かなり良いので**そのまま維持したい部分**もあります。

### ① `FacetPlane` を predicate に使わない

これは非常に重要です。

```text
exact orientation
        ↓
topology

f64 plane
        ↓
visualization / distance acceleration
```

という責務分離は正しいです。

---

### ② `-0.0 == +0.0` を明示した

ここも良いです。

```rust
if a == b
```

で重複判定する仕様を固定したことで、

```text
-0.0
+0.0
```

の扱いが曖昧になりません。

---

### ③ representative を全 API に持たせる

これはかなり使いやすい設計です。

```text
input index
 ↓
representative
 ↓
canonical site
```

を全結果で共通化できます。

特に、

```text
ConvexHull
Delaunay
Voronoi
```

で重複処理を別々にしなくて済みます。

---

### ④ logical facet と simplex facet を分離した

ここは `convx` の重要な設計ポイントだと思います。

```text
construction representation
    Simplex
        ↓
merge
        ↓
LogicalFacet
```

としているので、

```text
cube
```

を

```text
6 logical facets
```

として自然に返せます。

Qhull も simplicial facet と non-simplicial facet を別概念として扱っています。([Qhull][6])

---

### ⑤ Delaunay の退化をエラーにしない

ここも良いです。

```text
coplanar / cospherical
        ↓
not numerical failure
        ↓
valid degenerate geometry
```

という思想は、このライブラリの価値になります。

---

# 20. 今の仕様を「実装開始可能」にするなら修正優先度

私なら次の順で修正します。

| 優先度 | 項目                                              | 状態               |
| ------ | ------------------------------------------------- | ------------------ |
| **P0** | coplanar point の facet 内 convex hull 判定       | 修正必須           |
| **P0** | parallel batch の compatibility 判定              | 具体化必須         |
| **P0** | cospherical lifted facet の triangulation rule    | 具体化必須         |
| **P0** | Voronoi の「true diagram」vs「Delaunay dual」     | 意味を固定必須     |
| **P1** | `FacetPlane` の overflow/underflow 規約           | 明文化推奨         |
| **P1** | `volume()` の simplex decomposition               | 明文化必須         |
| **P1** | lifted orientation の行列・符号規約               | 固定推奨           |
| **P1** | rank / spanning_points の deterministic algorithm | 固定推奨           |
| **P1** | 高次元 Voronoi interface の表現力                 | 再検討推奨         |
| **P2** | allocation failure の扱い                         | 後回し可           |
| **P2** | Qhull benchmark option                            | Phase 2 前に固定   |
| **P2** | predicate metamorphic tests                       | Phase 1 に追加推奨 |

---

# 21. 私なら仕様の中心をこう整理する

現在の仕様はかなり情報量が多いので、さらに一段強くするなら、冒頭に次の **4つの不変条件**を置きます。

```text
Invariant 1 — Predicate correctness

すべてのトポロジー上の分岐は、仕様で定義された
数学的多項式の exact sign のみに依存する。


Invariant 2 — Numerical independence

FacetPlane、距離近似、外心などの f64 数値は、
トポロジーを変更してはならない。


Invariant 3 — Canonical public topology

公開結果は、construction representation を捨て、
logical facet / site / simplex を正規化したものとして比較する。


Invariant 4 — Parallel equivalence

parallel construction は、逐次 construction と同一の
canonical topology を生成する。

Delaunay の非一意な対角選択については、
同一 binary 内で deterministic に一致する。
```

そして各アルゴリズムを、

```text
predicate
    ↓
candidate topology
    ↓
exact validation
    ↓
mutation
    ↓
canonicalization
```

の流れにすると、かなり強固になります。

---

## 総評

**設計思想はかなり良いです。**
特に「浮動小数点で計算した平面を使って topology を決めない」という原則を最後まで守ろうとしている点は、`convx` の一番重要な部分だと思います。

一方、今は **「exact predicate の仕様」はかなり固まっているのに、「退化 topology をどう canonicalize するか」がまだ少し曖昧**です。

最大の論点はこの4つです。

```text
1. coplanar point を facet 内 convex hull に対してどう分類するか
2. parallel Quickhull の batch compatibility をどう判定するか
3. cospherical Delaunay の diagonal をどう決めるか
4. その退化 Delaunay から Voronoi をどう定義するか
```

ここを固めれば、かなり「実装仕様」として強くなります。

特に **3 → 4 は一続きの問題**です。
「Delaunay は退化しても成功させる」と決めた時点で、`VoronoiVertex { simplex }` という現在の API は再検討が必要になります。ここを曖昧なまま Phase 4 に進めると、実装途中で API を作り直す可能性が高いです。

[1]: https://www.qhull.org/html/qh-optq.htm?utm_source=chatgpt.com "Qhull control options (Q)"
[2]: https://doc.cgal.org/Manual/3.9/doc_html/cgal_manual/Triangulation_3_ref/Class_Delaunay_triangulation_3.html?utm_source=chatgpt.com "Delaunay_triangulation_3<DelaunayTriangulationTraits_3,TriangulationDataStructure_3,LocationPolicy>"
[3]: https://qhull.org/html/qhull.htm?utm_source=chatgpt.com "qhull -- convex hull and related structures"
[4]: https://cgal.github.io/9465/doc/Voronoi_diagram_2/index.html?utm_source=chatgpt.com "CGAL 6.3 - 2D Voronoi Diagram Adaptor: User Manual"
[5]: https://users.math.uoc.gr/~mkaravel/files/papers/jigsaw-voronoi-cgal.pdf?utm_source=chatgpt.com "Delaunay Tessellations and Voronoi Diagrams in CGAL 15"
[6]: https://www.qhull.org/src/libqhull/qh-poly.htm?utm_source=chatgpt.com "poly.c, poly2.c -- polyhedron operations"
