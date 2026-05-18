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
