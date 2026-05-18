# subsecond-demo

Dioxus 0.7 + subsecond によるRustコードのホットパッチ検証用デモ。
親リポジトリ「eikodan」（理想のWebシステムを模索する曳光弾）の subsecond 検証成果物。

rust-htmx は axum + cargo-watch 方式でフルプロセス再起動するのに対し、こちらは
**実行中のWASMバイナリにシンボル単位でパッチを当てる**方式を試す。

## 前提

- Rust toolchain
- wasm32-unknown-unknown ターゲット: `rustup target add wasm32-unknown-unknown`
- dioxus-cli (`dx`): `cargo install dioxus-cli --locked` （フルビルドで数分かかる）

## 起動

```bash
cd subsecond-demo
dx serve --hotpatch
```

初回のみ:
- `wasm-bindgen-cli` と `esbuild` が自動DLされる
- フルWASMビルド (target/ クリーン時) は約20〜30秒

起動後 http://localhost:8080 を開く。

## ホットパッチを試す

`src/main.rs` を編集して保存するだけ:

- `rsx!{}` 内のテキスト変更 → **Hotreloading**（再ビルドなし、数十ms）
- クロージャ本体 `move |_| count += 1` の数値変更 → **Hot-patching**（subsecond発火、200〜300ms）
- 新しい関数追加・既存関数の本体変更 → **Hot-patching**（200〜300ms）
- struct のフィールド追加・型変更 → フルリビルド必要

dx のログに `Hot-patching: ... took XXXms` が出れば subsecond が動いている証拠。

## 計測結果（macOS 26.4.1 / Apple Silicon / Rust 1.91.1）

詳細は [`../rust-htmx/readme.md`](../rust-htmx/readme.md) の「Rust側コードのホットリロード比較」を参照。

要約: 2回目以降 約 200〜300ms。axum + cargo-watch の約1.4秒に対し **約6倍速**。

## 主要な制約

公式ドキュメント記載分（[Dioxus 0.7 hot-reload](https://dioxuslabs.com/learn/0.7/essentials/ui/hotreload/) / [subsecond docs.rs](https://docs.rs/subsecond/0.7.9/subsecond/)）:

- **tip crate のみパッチ対象** — `main.rs` 配下の `mod` は対象。`lib.rs` 分割すると無効化
- **struct レイアウト変更は不可** — フィールド追加・型変更はフルリビルド
- **`--hotpatch` フラグは Dioxus 0.7 でも experimental** 扱い
- **iOS実機は未対応**（codesign 制約）。シミュレータは可
- **thread-local がパッチごとにリセット** される既知issue

## 2026/5時点のsubsecond対応範囲

一次ソース: [subsecond docs.rs v0.7.9](https://docs.rs/subsecond/0.7.9/subsecond/) / [Dioxus 0.7 hot-reload docs](https://dioxuslabs.com/learn/0.7/essentials/ui/hotreload/)

### 公式記載の「ホットパッチで処理 / フルビルドが必要」

#### subsecond クレート本体の限界

| 対象 | 挙動 | 公式記述 |
|---|---|---|
| struct のレイアウト・アライメント変更 | **✗ フルビルド** | "Subsecond currently does not support hot-reloading of structs. This is because the generated code assumes a particular layout and alignment of the struct." |
| tip crate 以外（依存crate, workspace member）の編集 | **✗ パッチ非対象** | "Rust hot-patching currently only tracks the 'tip' crate in your project. If you edit code in any of your dependencies … DX does not register that change" |
| `main.rs` + `lib.rs` 二段構成 | **✗ パッチ機能不全** | "Crate setups that have a main.rs importing a lib.rs won't patch sensibly since the crate becomes a library for itself" |
| static initializer 変更 | **✗ サイレント無視** | "Changes to static initializers will not be observed" |
| 新規 global / static 追加 | △ パッチ可だがデストラクタ呼ばれない | "You may add new globals at runtime, but their destructors will never be called" |
| global / static のリネーム | △ 別物として扱われる（状態ロス） | "Globals are tracked across patches, but renames are observed as introducing a new global" |
| `Cargo.toml` 変更（依存追加・feature・version） | **✗ フルビルド** | 直接明記なし。コード生成シードが変わるため当然 |

#### Dioxus 0.7 の `--hotpatch` 有効時に依然フルビルドな項目

| 対象 | 挙動 |
|---|---|
| コンポーネント signature 変更（`#[component]` props の追加/削除） | **✗ フルビルド** |
| 前回コンパイルに存在しなかった新しい変数・式 | **✗ フルビルド** |
| `use` 文 / モジュール構造変更 | **✗ フルビルド** |
| RSX 属性内の複雑な式（関数呼び出しを含むもの） | **✗ フルビルド** |

### 一般的なRustコード変更ごとの分類（qualitative 評価、発生頻度の数値は定性的印象）

公式に明記のない多くの編集について、subsecond の仕組み（シンボル単位のパッチ）から推定される挙動と、Webアプリ開発における体感的な発生頻度。**「頻度」列はデータではなく筆者の qualitative 印象**（プロジェクト・フェーズで偏る）。

| 変更の種類 | パッチ可否 | 一般Webアプリでの発生頻度 |
|---|---|---|
| 関数本体・制御フロー・エラーハンドリングの変更 | ✓ パッチ | **非常に高い**（編集の大半） |
| 文字列リテラル / 数値定数の変更 | ✓ パッチ | 高い |
| 新しいヘルパー関数の追加 | ✓ パッチ（新シンボル） | 高い |
| `rsx!` 内のテキスト・属性値変更 | ✓ Hotreloading（subsecond経由ではなくRSXパス） | 非常に高い |
| 新しいハンドラ/ルートの追加 | △ 関数本体はパッチ可。ルート登録自体は要再起動の可能性 | 中 |
| **struct への新フィールド追加** | **✗ フルビルド** | **中〜高（モデル拡張で必発）** |
| struct フィールドの型変更 | **✗ フルビルド** | 中 |
| enum 新バリアント追加 | docs明記なし。レイアウト依存の可能性あり | 中 |
| 関数シグネチャに新引数追加 | docs明記なし。要実機検証 | 中 |
| `Cargo.toml` に依存追加 | **✗ フルビルド** | 低（プロジェクト初期に集中） |
| トレイト定義・新 `impl` ブロック | docs明記なし。generics絡みは "cascade of codegen changes" 警告あり | 低〜中 |

### 実用上の運用判断

- **モデルが安定した運用フェーズ**: 編集の大半が関数本体変更になり subsecond の 200〜300ms が支配的 → Go の `go run` 再起動と同等以上の体感
- **データモデルを毎日いじる初期設計フェーズ**: struct 変更が頻発しフルビルド比率高 → 恩恵減
- **`--hotpatch` は2026/5時点で experimental**（公式表記）。プロジェクトの安定性要件次第で「日常運用に組み込むか / 補助的に使うか」が分かれる

### 代替・併用: playground crate 方式

subsecond の experimental ステータスを踏まえると、**Cargo workspace に空の playground crate を作って開発する**のが安定路線の選択肢として有力。

**狙い**

- subsecond のような実験的機能に依存せず、Cargo の incremental compilation だけでビルド速度を稼ぐ
- 重い依存（axum, tokio, minijinja 等）を本体crateに閉じ込め、playground crate は最小依存に保つ
- 新機能やロジックを playground 内で隔離して試作し、安定したら本体に移植

**構造例**

```
eikodan/
├── Cargo.toml          (workspace ルート)
├── rust-htmx/          (本体: axum binary)
├── subsecond-demo/     (Dioxus検証)
└── playground/         (実験用、最小依存)
    ├── Cargo.toml
    └── src/main.rs
```

`playground/Cargo.toml` は実験対象のドメインロジックだけを直接依存に持ち、Web/UI 系の重い依存は持たない。フルビルドも差分ビルドも数百msオーダーで済む。

**subsecondと組み合わせる場合**

- 試作フェーズ: playground crate 単独で関数本体をいじる → cargo の incremental だけで秒速
- 統合フェーズ: 本体に移植して subsecond で hot-patch
- どちらも experimental に賭けない・賭けるの選択肢を残せる

**この方式の限界**

- 本体crateとの結合（型を渡す、トレイト境界を共有する等）が増えると playground だけで完結しなくなる
- 結局本体のビルドが必要になる場面は減らない（ルーティング・テンプレ・DI周り）
- 「設計確認のための試作」までで、UIまで含めた統合確認は本体が要る

### 不確定領域（公式ドキュメントから断定できなかった項目）

実機で確認しない限り挙動を断定できないもの:

- enum バリアント追加（レイアウト変更扱いか別物か）
- 関数シグネチャ変更（引数追加、戻り値型変更、ライフタイム指定変更）
- generic monomorphization の増加（新しい型引数での具体化）
- 新規 `impl` ブロックの追加
- マクロ展開結果が変わる変更（特に手続きマクロ）
