# ホットリロードの仕組み（変更がブラウザに反映されるまで）

fastweb の「保存したら勝手に反映」は、**2つの独立した監視**の合わせ技で成り立つ。
両者は別プロセス／別タスクで動き、互いを直接知らない。**ファイルの書き込みを介して間接的に連携**する。

| # | 監視役 | 何を監視 | 何をする |
|---|---|---|---|
| ① | **Tailwind CLI `--watch`**（別プロセス） | `assets/input.css` と `@source` のテンプレHTML | `crates/app/static/app.css` を作り直す |
| ② | **notify + tower-livereload**（app内のtokioタスク） | `template_dirs()` + `crates/app/static/` | 変更を拾って**ブラウザを丸ごと再読込** |

加えてサーバ側には:
- **minijinja-autoreload**（`crates/webcore/src/lib.rs`）: リクエストのたびにテンプレの変更を見て差分再読込（**再コンパイル不要**）。
- **CSS配信ハンドラ**（`crates/app/src/main.rs` の `app_css`）: リクエストのたびに `app.css` を**ディスク直読み**（サーバ側キャッシュなし＝常に最新）。

---

## tower-livereload の中身（実装確認済み）

注入された `polling.js` が `/tower-livereload/.../long-poll` へ **fetch を張りっぱなし**にする
（サーバ側 `LongPollBody` が応答を保留＝接続維持）。`reloader.reload()` が呼ばれると保留が解け、
fetch が解決 → `backUp` が応答するのを待って **`window.location.reload()`（フルリロード）**。

- HTMXの部分更新（`hx-request` ヘッダ付き）は `not_htmx_predicate` でリロード対象から除外。
- サーバ再起動（Rust変更時）でも long-poll 接続が切れ、復帰時に再読込が走る。

notify 側は `template_dirs()` と `static/` の**両方を1つのチャネルに集約**して `reload()` を呼ぶ。
fsイベントが複数飛べば `reload()` も複数回呼ばれる（→ 後述のダブル発火）。

---

## 変更の種類別フロー

### A. テンプレHTMLのテキスト/HTMX属性だけ変えた
```
保存 → notify(template_dirs) → reload()
     → ブラウザ location.reload()
     → リクエスト時に minijinja-autoreload が変更を検知して再読込（再コンパイルなし）
```
CSSの生成物は変わらないので ① は実質ノータッチ。1回の reload で確定。

### B. テンプレに Tailwind クラスを新規追加した（一番込み入る）
```
保存 ─┬→ ② notify(template) → reload #1 → 再読込
      │        （この時点では app.css 未再生成 → 新クラスが一瞬当たらない）
      └→ ① Tailwind が @source で検知 → 数十ms後 app.css 再生成
                → ② notify(static) → reload #2 → 再読込（新CSS反映で確定）
```
**クラス追加時だけ reload が2回**走り得る。順序上、1回目は無スタイルの一瞬→2回目で正。
テキストだけ・既存クラスの変更なら Tailwind の出力が変わらず、2回目は起きない。

### C. `assets/input.css`（テーマ等）を変えた
`input.css` は notify の監視外。① が検知 → `app.css` 再生成 → `static/` の変更で ② が reload。きれいに1経路。

### D. Rustハンドラ/ロジックを変えた
このループの**外**。再コンパイルが必要（`bacon run` / `bacon serve` がサーバ再起動）。
再起動で long-poll が切れ、tower-livereload が復帰時に reload する。

---

## キャッシュ ── 3層に分けて整理

| 層 | 挙動 | stale risk |
|---|---|---|
| **サーバ側テンプレ** | minijinja-autoreload が毎リクエストで変更チェック→差分再読込（`set_fast_reload(true)`） | なし |
| **サーバ側CSS** | `app_css` が毎リクエスト `tokio::fs::read` でディスク直読み（サーバ側キャッシュゼロ） | なし |
| **ブラウザ側CSS** | `app.css` に **debugビルド時のみ `Cache-Control: no-cache`** を付与 → reload で必ず取り直す | なし（下記で解消済み） |

- **sccache / incremental はこの経路に一切絡まない。** あれは Rust *コンパイル* のキャッシュで、
  CSS・テンプレ変更ではコンパイル自体が走らない。効くのは D（Rust変更）のときだけ。
- **`Cache-Control: no-cache` を入れた理由**: 付けないと `app.css` は validator も無く、ブラウザが
  鮮度を確定できず挙動がブラウザ/プロキシ依存になる。debug時に `no-cache` を明示し、reload のたびに
  確実に最新CSSを取らせる（`crates/app/src/main.rs` の `app_css`、`#[cfg(debug_assertions)]`）。
  本番は validator/長期キャッシュが望ましいので、テンプレ埋め込み対応時にまとめて決める。

---

## まとめ（最短経路）
- **画面まわり（テンプレ/CSS/HTMX）** … Rustビルドゼロ。① がCSSを作り直し、② がブラウザを reload。
- **Rust** … この機構の外。再コンパイル＋サーバ再起動 → reload。
- **キャッシュで悩むのはブラウザCSSだけ** で、debugの `no-cache` で解消済み。sccache等は無関係。
