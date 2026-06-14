# ホットリロードの仕組み（変更がブラウザに反映されるまで）

> **前提: 日常開発はCDN（`cargo run -p app` だけ・1プロセス）。** このときCSSはブラウザJITが即生成するので、
> ①Tailwind `--watch` も `static/` 監視も `/static/app.css` 配信も**使われない**。効くのは ②テンプレ監視 +
> minijinja-autoreload だけ。下記①と「built（最終確認/本番）モード」の話は **`CSS=built` で起動したときだけ** 当てはまる
> （`assets/check-css.sh` 後の目視や release）。日常CSSの崩れ検査はpush前ゲートに集約する（→ 末尾の★）。

fastweb の「保存したら勝手に反映」は、**2つの独立した監視**の合わせ技で成り立つ。
両者は別プロセス／別タスクで動き、互いを直接知らない。**ファイルの書き込みを介して間接的に連携**する。

| # | 監視役 | 何を監視 | 何をする | いつ |
|---|---|---|---|---|
| ① | **Tailwind CLI `--watch`**（別プロセス） | `assets/input.css` と `@source` のテンプレHTML | `crates/app/static/app.css` を作り直す | builtモードのみ |
| ② | **notify + tower-livereload**（app内のtokioタスク） | `template_dirs()`（+ builtでは `crates/app/static/`） | 変更を拾って**ブラウザを丸ごと再読込** | 常時 |

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

### B. テンプレに Tailwind クラスを新規追加した
- **日常（CDN）**: ② が reload → ブラウザJITが新クラスを即生成。**1回の reload で確定**、待ち時間なし。
- **builtモード（`CSS=built`）**: 下のように reload が2回走り得る（最終確認時のみ意識すればよい）:
```
保存 ─┬→ ② notify(template) → reload #1 → 再読込
      │        （この時点では app.css 未再生成 → 新クラスが一瞬当たらない）
      └→ ① Tailwind が @source で検知 → 数十ms後 app.css 再生成
                → ② notify(static) → reload #2 → 再読込（新CSS反映で確定）
```
順序上、1回目は無スタイルの一瞬→2回目で正。テキストだけ・既存クラスの変更なら出力が変わらず2回目は起きない。

### C. `assets/input.css`（テーマ等）を変えた
`input.css` は notify の監視外。① が検知 → `app.css` 再生成 → `static/` の変更で ② が reload。きれいに1経路。

### D. Rustハンドラ/ロジックを変えた
このループの**外**。再コンパイルが必要（`bacon run` / `bacon serve` がサーバ再起動）。
再起動で long-poll が切れ、tower-livereload が復帰時に reload する。

---

## ★ パージ崩れ（消える方向）は push前ゲートで確定検査する

日常はCDN（ブラウザJIT）なので、開発中は**何でも生成される＝パージで消えるクラスが見えない**。
「`text-{{ color }}-500` のような動的合成」「`.rs` に書いたクラス」は本番のCLIパージで黙って消えるが、
CDNでは動いてしまう。この**ズレを手元を離れる前に確定検査する**のが `assets/check-css.sh`（pre-commitは使わず、push前）:

1. **クリーンビルド（`--watch` なし）** … `app.css` をフル生成。`--watch` の差分ビルドは追記的で
   消したクラスがセッション中残る（＝削除方向を信用できない）が、ワンショット生成は毎回パージ確定なので
   **追加も削除も正しい**。
2. **semgrep（`assets/semgrep/`）** … 上記の危険パターン（class属性内の `{{ }}` 動的合成 / `.rs` のクラス文字列）を
   静的検出。ビルドが黙って消すものを「エラー」として可視化する。

release ビルドも必ずCLIクリーン生成を通るので、最終成果物は常に正しい。任意で `assets/hooks/pre-push` を入れると
fastweb変更を含む push でだけ自動でこのゲートが走る。

---

## キャッシュ ── 3層に分けて整理

日常CSSはCDN配信なので下記の「サーバ側CSS / ブラウザ側CSS」は**builtモードのときだけ**の話。テンプレ層は常時。

| 層 | 挙動 | stale risk |
|---|---|---|
| **サーバ側テンプレ** | minijinja-autoreload が毎リクエストで変更チェック→差分再読込（`set_fast_reload(true)`） | なし |
| **サーバ側CSS**（built） | `app_css` が毎リクエスト `tokio::fs::read` でディスク直読み（サーバ側キャッシュゼロ） | なし |
| **ブラウザ側CSS**（built） | `app.css` に **debugビルド時のみ `Cache-Control: no-cache`** を付与 → reload で必ず取り直す | なし（下記で解消済み） |

- **sccache / incremental はこの経路に一切絡まない。** あれは Rust *コンパイル* のキャッシュで、
  CSS・テンプレ変更ではコンパイル自体が走らない。効くのは D（Rust変更）のときだけ。
- **`Cache-Control: no-cache` を入れた理由**: 付けないと `app.css` は validator も無く、ブラウザが
  鮮度を確定できず挙動がブラウザ/プロキシ依存になる。debug時に `no-cache` を明示し、reload のたびに
  確実に最新CSSを取らせる（`crates/app/src/main.rs` の `app_css`、`#[cfg(debug_assertions)]`）。
  本番は validator/長期キャッシュが望ましいので、テンプレ埋め込み対応時にまとめて決める。

---

## まとめ（最短経路）
- **日常（CDN）** … テンプレ/HTMX保存 → ② がブラウザを reload。CSSはブラウザJITが即生成。Rustビルドゼロ・1プロセス。
- **最終確認（built）** … `CSS=built` で起動。① がCSSを作り直し、② がブラウザを reload。push前は `check-css.sh` で確定検査。
- **Rust** … この機構の外。再コンパイル＋サーバ再起動 → reload。
- **パージ崩れ（消える方向）** … 開発では見えない。push前ゲート（クリーンビルド + semgrep）で捕まえる（★参照）。
