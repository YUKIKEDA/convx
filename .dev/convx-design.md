# Architecture & Technical Specification: `convx`

**High-Performance, Robust N-Dimensional Convex Hull, Delaunay, and Voronoi Engine for Pure Rust**

---

## 1. プロジェクト概要 & 設計思想 (Overview & Philosophy)

`convx` は、C言語の事実上の世界標準である **Qhull** を現代的なアプローチで再設計し、速度・堅牢性・安全性のすべてで凌駕することを目指す純Rust製のN次元計算幾何学ライブラリです。

### コア設計原則
1. **No "Later":** 実世界で耐えうる幾何ライブラリに不可欠な「Facet Merging（面の併合）」と「タスク並列処理（トポロジー並行更新）」を設計のコアに据え、後付けの改修を排除する。
2. **Deterministic & Robust:** 浮動小数点誤差に起因するトポロジー破綻（無限ループ、反転面）を数学的許容誤差モデルと状態不変条件（State Invariants）により排除する。
3. **Pure Rust, Stable-First:** C/C++依存をゼロにし、Nightly機能に依存せず `cargo build`（Stable Rust）で完結しつつ、ランタイムCPU機能検出（AVX-512/AVX2/NEON）による最大性能を引き出す。
4. **Zero-Overhead Projections:** 内部は計算幾何学的に完全な接続関係（Incidence Graph）を構築しつつ、外部に対してはゼロコストで軽量ビュー（頂点インデックスのみ、Delaunay分割、Voronoi双対図）を射影・提供する。

---

## 2. システムアーキテクチャ全体像 (System Architecture)

```
[ User Application / Ecosystem (NumPy, faer, nalgebra) ]
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. API Surface & Geometry Adapters                          │
│    - ConvexHullBuilder / DelaunayBuilder                    │
│    - Flat Slice `&[f64]` (Row-Major, Zero-Copy Core)        │
│    - Const Generics Wrapper `<const D: usize>` (D <= 6)     │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. Parallel Quickhull Pipeline                              │
│    - Initial Simplex Construction & Degeneracy Check (SVD)   │
│    - SIMD Farthest Point Partitioning (`pulp` / `wide`)     │
│    - Synchronized Independent Set Batch Selector            │
│      (Prioritized Horizon Reservation)                      │
│    - Rayon Parallel Worker Pools                            │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. Robust Topology & Merging Engine                         │
│    - Triangulated Multi-facet (Logical Grouping)            │
│    - Hybrid Facet Merging (Immediate + Deferred)            │
│    - Dynamic Epsilon Auto-Scaling (Bounding Box based)      │
│    - Upper Hull Pruning (Lifting Map Optimization)          │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. Memory & Linear Algebra Kernels                          │
│    - Chunked Lock-free Generational Arena                   │
│    - Hybrid Hyperplane Solver:                              │
│      * D <= 4: Analytic SIMD Cross Product                  │
│      * 5 <= D <= 8: Inlined Modified Gram-Schmidt           │
│      * D > 8 or Singular: `faer` Householder QR             │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. メモリモデル & データ構造仕様 (Memory & Data Structures)

### 3.1 Chunked Lock-free Generational Arena
頻繁な面の生成・マージ・削除（Tombstone）を並列環境下でアロケーションオーバーヘッドなしに処理するため、固定長チャンクをアトミックに払い出す世代付きインデックスアリーナを採用します。

```rust
/// 世代付きインデックスによるダングリングポインタ防止参照
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FacetId {
    pub index: u32,
    pub generation: u32,
}

/// 固定サイズ（D頂点）の単体構造。Arena 内で連続配置される
#[repr(C)]
pub struct Simplex<const MAX_D: usize> {
    pub vertices: [u32; MAX_D],      // 頂点インデックス (MAX_D = D)
    pub neighbors: [FacetId; MAX_D],  // 各Ridge（D-1面）を挟んだ隣接単体
    pub normal: [f64; MAX_D],        // 外向き単位法線ベクトル
    pub offset: f64,                 // 超平面オフセット (P · N + offset = 0)
    pub hypervolume: f64,            // 単体の超体積
    pub group_id: u32,               // 所属する論理FacetのグループID (Union-Find)
    pub flags: u32,                  // Tombstone, In-Horizon, Pruned, etc.
}

pub struct ChunkedArena<T> {
    chunks: Vec<Box<[T]>>,
    chunk_size: usize,
    active_chunk: std::sync::atomic::AtomicUsize,
    free_head: std::sync::atomic::AtomicU64,
}
```

### 3.2 Triangulated Multi-facet（面の論理グループ化）
Facet Merging（面の併合）が発生しても内部構造を可変長アロケーションに崩さず、**固定サイズの単体の集合＋論理グループID（`FacetGroupId`）** として管理します。

- **不変条件 (Invariants):**
  - 単体間の同一グループ判定は、高速なDisjoint Set（Union-Find）または世代付きグループテーブルを参照。
  - 同一グループに属する単体群の境界外部エッジ（Ridges）のみが、他グループとの「境界（Ridge）」として外部に露出する。
- **代表値（Centrum & Normal）の算出規則:**
  - **Centrum（面の中心点）:** グループに属する全単体の「一意な頂点集合」の座標算術平均。
  - **代表法線（Representative Normal）:** グループを構成する単体のうち、**超体積（Hypervolume）が最大**の単体の法線ベクトルを採用（幾何学的安定性の最大化と再計算コストの削減）。

---

## 4. 並行処理 & アルゴリズム詳細仕様 (Parallel Algorithm)

### 4.1 Prioritized Horizon Reservation による Synchronized Independent Set
競合（同一Facetの奪い合い）によるロールバックを防ぐため、**反転面を生じさせない非干渉な点のグループを選出して一括コミットするバッチ並列方式**を採用します。

```
[Outside Points Partitioning (Rayon Parallel)]
                      │
                      ▼
[Candidate Farthest Points Selection (1 per Facet)]
                      │
                      ▼
[Sequential Prioritized Reservation Phase]
  - 距離が最大の点 P_0 から順に処理
  - P_i の可視領域（Visible Facets / Horizon）を深さ優先探索（DFS）
  - 未予約の面のみで構成される場合 ──> 予約フラグをアトミックにコミットし、Batch に追加
  - 他の点とHorizonが衝突した場合 ────> 今回のBatchから除外（次回以降へ持ち越し）
                      │
                      ▼
[Rayon Parallel Execution Phase (Zero Contention)]
  - Batch 内の全点 P_k を並列ワーカーへ分配
  - 各スレッドは干渉ゼロで以下を実行:
      1. 予約済みHorizon内部面の削除（Tombstone設定）
      2. P_k と Horizon Ridge を結ぶ新単体の生成
      3. 局所的 Facet Merging の適用
                      │
                      ▼
[Barrier Synchronization & Partition Re-assignment]
```

### 4.2 Qhull完全踏襲のハイブリッド Facet Merging
凸包の幾何学的正しさを保証するため、**「中間状態も含め、凸包は常に外側に対して非凹（Non-concave）である」** という不変条件を維持します。

1. **Pre-merging（点追加直前の局所マージ）:**
   - 新単体作成時、Horizon境界にある既存面との法線の成す角が角度許容誤差未満（ほぼ同一平面）、またはCentrumとの距離が許容誤差未満の場合、即座に同一グループへ統合。
2. **Post-merging（点追加後の非凸面修復）:**
   - 新規作成された単体と隣接する単体の間に「反転（Inverted ridge）」または「凹角（Non-convex ridge）」が形成された場合、再帰的にグループをマージ。
3. **サイクル防止機構:**
   - マージにより面の超体積がゼロ以下（縮退）になる場合はトポロジー操作をロールバックし、該当頂点を同一平面上の内包点として破棄。

---

## 5. 数値計算 & SIMDカーネル (Linear Algebra & SIMD)

### 5.1 超平面算出ハイブリッド・ソルバー
単体（$D$ 点）から単位超平面方程式 $P \cdot N + d = 0$ を求める演算の最適化戦略。

| 次元 $D$ | アルゴリズム | 実装方式 | 特徴 |
| :--- | :--- | :--- | :--- |
| **$D \le 4$** | 外積・解析的行列式 | 手書きSIMD / インラインスタック配列 | 分岐なし、アロケーションゼロ、最速 |
| **$5 \le D \le 8$** | 修正Gram-Schmidt直交化 | スタック上固定配列（`[f64; 64]`）走査 | レジスタ活用、キャッシュ効率最大 |
| **$D > 8$ または 特異判定** | Householder QR 分解 | **`faer::linalg::qr`** へディスパッチ | 任意次元対応、特異値・悪条件に最強 |

### 5.2 距離計算の Stable SIMD ベクトル化
Quickhull全体のボトルネックである点群走査 $d = P \cdot N + \text{offset}$ の計算は、Stable Rustで動く **`pulp`** または **`wide`** を採用。

- **メモリアライメント:** 点群は行優先（Row-Major）フラットスライス `&[f64]`（64バイトアライメント推奨）。
- **ディスパッチ:** バイナリ起動時にランタイムCPU検出（AVX-512 / AVX2 + FMA / ARM NEON）を行い、最適なベクトル化関数ポインタを静的バインド。

### 5.3 動的許容誤差（Dynamic Epsilon）の算出ルール
Qhullの数学的証明に基づく自動スケーリング式を採用。

$$E_{\text{dist}} = (D + 1) \cdot \max_{i} (|P_{\max, i} - P_{\min, i}|) \cdot \epsilon_{\text{mach}} \cdot c_1$$

$$E_{\text{angle}} = D \cdot \epsilon_{\text{mach}} \cdot c_2$$

- $\epsilon_{\text{mach}} = 2.22 \times 10^{-16}$ (`f64::EPSILON`)
- 定数係数 $c_1, c_2$ は Qhull のキャリブレーション値（デフォルト $c_1 = 4.0, c_2 = 8.0$）を採用。
- `ConvexHullBuilder` でユーザーが任意の固定値または乗数をオーバーライド可能。

---

## 6. Lifting Map (Delaunay / Voronoi) 最適化パイプライン

$D$ 次元点群から Delaunay 分割および Voronoi 図を生成する際、$D+1$ 次元凸包エンジンへ直結する最適化を適用します。

### 6.1 仮想座標（Virtual Lifting Coordinate）
点 $P = (x_1, \dots, x_D)$ に対する第 $D+1$ 座標 $x_{D+1} = \sum_{j=1}^D x_j^2$ を実メモリに事前確保せず、距離判定および法線計算のストライドループ内でオンザフライ計算する仮想イテレータを採用。メモリ消費量を約 $1 / (D+1)$ 削減。

### 6.2 Upper Hull Pruning（上側の面の構築スキップ）
- **数学的根拠:** Delaunay単体に射影されるのは、法線の第 $D+1$ 成分が負（下向き）である **Lower Hull** のみである。
- **最適化:** 
  - Horizon探索および新単体生成時、法線 $N$ の $N_{D+1} > +E_{\text{angle}}$（上向き）であることが自明な面について、Outside Set の振り分けと探索を即座に中断（プルーニング）。
  - 不要なUpper Hullの精緻化計算をスキップすることで、Qhull比で大幅な計算量削減を達成する。

---

## 7. API サーフェス & 型設計 (API Specification)

```rust
pub mod prelude {
    pub use crate::{ConvexHull, ConvexHullBuilder, ConvexHullError, DelaunayTriangulation};
}

/// 入力エラーおよび縮退エラーの完全な型情報
#[derive(thiserror::Error, Debug)]
pub enum ConvexHullError {
    #[error("Input points count ({actual}) is less than D + 1 ({required})")]
    InsufficientPoints { actual: usize, required: usize },

    #[error("Dimension degeneracy detected: points span only {actual_dim}D subspace within tolerance")]
    DegenerateDimension {
        actual_dim: usize,
        /// 縮退している空間のアフィン正規直交基底 (dim × actual_dim)
        affine_basis: Vec<f64>,
    },

    #[error("Arithmetic failure: numerical singularity or unresolvable topological collapse")]
    NumericalFailure(String),
}

/// メインBuilder
pub struct ConvexHullBuilder<'a> {
    dim: usize,
    points: &'a [f64], // Row-Major: [p0_x, p0_y, p0_z, p1_x, ...]
    distance_tolerance: Option<f64>,
    angle_tolerance: Option<f64>,
    parallel: bool,
    prune_upper_hull: bool, // Delaunay内部用
}

impl<'a> ConvexHullBuilder<'a> {
    pub fn new(dim: usize, points: &'a [f64]) -> Self { ... }
    pub fn tolerance(mut self, dist: f64, angle: f64) -> Self { ... }
    pub fn parallel(mut self, enable: bool) -> Self { ... }
    pub fn build(self) -> Result<ConvexHull, ConvexHullError> { ... }
}

/// Const Generics 特殊化ラッパー (D <= 6 用のゼロコストインターフェース)
pub struct StaticConvexHull<const D: usize> {
    inner: ConvexHull,
}

impl<const D: usize> StaticConvexHull<D> {
    pub fn build(points: &[[f64; D]]) -> Result<Self, ConvexHullError> {
        let flat_ptr = points.as_ptr() as *const f64;
        let flat_slice = unsafe { std::slice::from_raw_parts(flat_ptr, points.len() * D) };
        ConvexHullBuilder::new(D, flat_slice).build().map(|inner| Self { inner })
    }
}
```

---

## 8. テスト・検証 & CI 仕様 (Verification & Invariants)

外部実行環境（PythonやCコンパイラ）をCIに要求せず、決定論的かつ数学的に正しさを保証する二重の防壁を構築します。

### 8.1 幾何学的不変条件バリデータ (`verify_invariants`)
すべてのデバッグビルドおよびCIテストで走る純Rustの完全自己検査システム：

1. **オイラー＝ポアンカレの公式 (Euler-Poincaré Formula):**
   - 凸包境界の多面体複体に対し、$\sum_{k=0}^{D-1} (-1)^k F_k = 1 - (-1)^D$ が厳密に成立することを検証。
2. **全点包含性テスト (Global Point Containment):**
   - 全入力点 $P_i$ に対し、凸包の全Facetとの符号付き距離が $P_i \cdot N_f + d_f \le E_{\text{dist}}$ であることを検証（はみ出し点の完全排除）。
3. **境界隣接の対称性 (Neighbor Symmetry):**
   - Facet $A$ が Ridge $R$ を介して Facet $B$ と隣接している場合、Facet $B$ も同一の Ridge $R$ を介して Facet $A$ と隣接していなければならない。

### 8.2 事前生成オラクルデータセット (Static Oracles)
- Qhull / CGAL によって事前計算された正解データ（極小クラスタ、超立方体格子、同心球面、超高次元ランダム点群）を圧縮バイナリ（`tests/fixtures/*.bin`）としてリポジトリに内包。
- 頂点インデックスの包含集合の一致、および超体積（Volume）の一致を $10^{-12}$ の精度でアサーションする回帰テスト。

---

## 9. 実装ロードマップ (Implementation Roadmap)

```
[Phase 1: Foundations]
  ├── ChunkedLockFreeArena & Generational Index の実装
  ├── Stable SIMD (`pulp`) によるベクトル化距離計算カーネル
  └── ハイブリッド超平面ソルバー（解析的手書き外積 + faer QR）

[Phase 2: Sequential Core Quickhull]
  ├── 外接直方体からの初期単体選出 & SVD次元縮退検知
  ├── Triangulated Multi-facet 構造体と Union-Find
  └── 逐次版 即時＋遅延 Facet Merging ステートマシンの完成

[Phase 3: Parallelization & Independent Sets]
  ├── Outside Set 並列 Partitioning (Rayon)
  ├── Prioritized Horizon Reservation（先着予約独立バッチ選出器）
  └── 並列コミット & Arena チャンク払い出しの統合

[Phase 4: Generalization & Specialization]
  ├── Const Generics 特殊化 API (<const D: usize>, D <= 6)
  ├── Virtual Coordinates & Upper Hull Pruning による Delaunay / Voronoi 実装
  └── 幾何不変条件バリデータ & CIオラクル回帰テストスイートの整備
```