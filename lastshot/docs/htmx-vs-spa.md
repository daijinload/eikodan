# HTMX × Rust 雑談メモ

rust-htmx サンプルを触りながらの会話で出た要点を、後から見返せる形で整理。
個別の出典は本文中にリンク。

---

## 1. なぜ debug build なのに白フラッシュが見えないのか

**結論：1フレーム（16.6ms）以内に描画が終わっているので、白いコマがそもそも画面に出力されない。**

- ディスプレイは60Hz＝16.6ms間隔でしか絵を更新しない
- 今回のフルリロードは localhost + 小さな HTML + CDN キャッシュ済み資産で **10ms前後** で完了
- 結果：白くなる瞬間が「次の更新前」に塗り替えられて消える

ブラウザの「Paint Holding」機能はリロードには適用されない（Chrome公式に明記）。
ref: https://developer.chrome.com/blog/paint-holding

比較：PHP製の旧システムが 800ms 応答 = 60Hzで約48フレーム白く、はっきり「待った」と感じる領域。

---

## 2. cargo install cargo-watch は Cargo.toml に書けない

**結論：書けない。`cargo install` は「PCにツールを入れる」コマンドで、プロジェクトの依存ではないため。**

- `[dependencies]` / `[dev-dependencies]` は「ビルド時にリンクするライブラリ」を書く場所
- `cargo install xxx` は `~/.cargo/bin/xxx` に独立した実行ファイルを置くだけ
- Cargo 公式 manifest reference にも「ツール宣言用セクションは存在しない」と確認
- 位置付け：cargoサブコマンド拡張＝VS Code拡張やgit拡張に近い

ref: https://doc.rust-lang.org/cargo/commands/cargo-install.html

代替：README記載 / justfile / Makefile / xtaskパターン。

---

## 3. HTMX はバックエンド非依存（Rustだけのものではない）

**結論：HTMXはブラウザ側のJSライブラリで、サーバ側の言語は何でもいい。**

公式が「HTMLを返せるサーバなら何でもOK」と明記。
ref: https://htmx.org/docs/

| バックエンド | 1リクエストの典型応答時間 | 白フラッシュ消える？ |
|---|---|---|
| Rust (Axum) / Go | 〜1ms | 余裕で消える |
| Node (Fastify) | 3〜10ms | 消える |
| Node (Express) | 5〜15ms | だいたい消える |
| Python (FastAPI) | 5〜20ms | ギリギリ |
| Python (Django) / Ruby (Rails) / PHP (Laravel) | 30〜500ms | 見える |

ベンチ数値は TechEmpower 等の桁感、実装次第で変動。

---

## 4. SPA の構造的弱点：1画面で複数API

汎用REST設計のSPAは「画面1枚 = 3〜10リクエスト」になりがち。

```
GET /api/products/123          ← 商品
GET /api/products/123/reviews  ← レビュー
GET /api/users/me              ← ユーザ
GET /api/users/me/cart         ← カート
GET /api/products/123/related  ← 関連商品
```

各往復に：認証ヘッダ + JWT検証 + DBクエリ + JSONシリアライズ + ネット往復 + JSONパース + キャッシュ判定 + 再レンダー。
**依存関係があると並列化できず waterfall になる**（30ms×3段 = 90ms がベースライン）。

HTMX は「サーバ側でJOINして HTML で返す」1往復モデルなので、この往復の積み重ねが構造的に発生しない。

---

## 5. SPA 4大対処策は「複雑さの削減」ではなく「移動」

| 対処 | 削った複雑さ | 増えた複雑さ |
|---|---|---|
| GraphQL | フロントの複数fetch | スキーマ / resolver / DataLoader / N+1対策 / キャッシュ無効化 / コード生成 |
| BFF | フロントの集約ロジック | 中間サーバ / デプロイ単位増 / 認証伝播 / バージョニング |
| RSC | 一部のJSON往復 | Server/Client境界 / "use server"の使い分け / ハイドレーション不整合 |
| tRPC + batching | 型のズレ | TypeScript必須 / サーバ・クライアント密結合 / FW選定固定化 |

**減算ではなく置換**。HTMX だけが層そのものを抜いている。

### RSC のセキュリティ事例（GitHub Security Advisories で確認）

- GHSA-wfc6-r584-vfw7：RSCレスポンスのキャッシュポイズニング
- GHSA-vfv6-92ff-j949：RSCキャッシュバスター衝突によるポイズニング
- GHSA-267c-6grr-h53f：segment-prefetch経由のミドルウェアバイパス（High）
- GHSA-492v-c6pp-mqqv：動的ルートパラメータ注入によるバイパス（High）
- CVE-2025-29927：Next.jsミドルウェアバイパス

「実装ミス」ではなく**仕組みが複雑すぎて正解の実装が難しい**という構造由来。

---

## 6. HTMX は「Web 1.0回帰」ではない

SPA推進派のレッテル。実態は2010年代後半の現代ブラウザ機能の組み合わせ：

| 機能 | 使っているもの | 登場 |
|---|---|---|
| 非同期通信 | `fetch` API | 2015〜 |
| URL書き換え | `history.pushState` | 2010〜 |
| DOM差分挿入 | `MutationObserver` + `Range.createContextualFragment` | 2014〜 |
| イベント拡張 | カスタムイベント / `hx-trigger` | 2010年代 |
| プッシュ | SSE / WebSocket拡張 | 2010〜 |

**SPAの便利さをHTMLの読みやすさを捨てずに取り戻す**＝後退ではなく別ルートでの前進。

---

## 7. AI生成との親和性は構造的に大きい

1. **学習データの偏り**：HTML+属性パターンはWeb史30年分。RSC系はここ1〜2年で仕様が変動 → 訓練データ少なく古い情報混入
2. **静的解析しやすさ**：`<button hx-post="/todos">` を見れば挙動が1行で完結。React は呼び出し階層を辿らないと挙動が読めない
3. **Locality of Behaviour**（HTMX作者の用語）：振る舞いの記述場所＝表示される場所。AIエラーの多くが「見えていないファイルへの影響」由来なので、ローカリティの高さがエラー率を下げる

---

## 8. Rust で hot reload が成立する理由

「Rust = ビルド遅い = フロント反復に向かない」は今回の構成では当てはまらない：

| 変更箇所 | 反映方法 | 体感速度 |
|---|---|---|
| HTML/CSS (templates/) | minijinja-autoreload + tower-livereload | 即時 |
| Rustロジック (src/) | cargo-watch + incremental build | 〜0.3秒 |

画面いじりの9割はテンプレ編集 → Rustに触らない。
「Rustが遅い」ではなく「**Rust部分を触る回数自体が少ない設計**」になっている。

---

## 9. HTMX の守備範囲外と、その時の選択肢

### 向かない領域

サーバ往復（10〜50ms）が間に合わない処理：

- マウスドラッグ追従（Figma） → 16ms以下が必要
- 文字入力即時反映（Docs） → 1文字ごとにHTTPは無理
- ゲーム / Canvas / WebGL
- オフライン動作

### 選択肢は React だけではない

React/Vue/Angular は**強制ではなく流行**。

| ライブラリ | サイズ | 用途 |
|---|---|---|
| React + ReactDOM | 45KB | フルSPA |
| Alpine.js | 15KB | **HTMXとの定番ペア** |
| Stimulus | 10KB | Hotwire/Rails公式 |
| Lit | 5KB | Web Components |
| Solid.js | 7KB | reactive、Reactより速い |
| Svelte | 2KB | コンパイル時解決 |

**HTMX + Alpine（合計30KB）** で「Reactで作る軽いUI」と同等以上のことができる：

```html
<div x-data="{ open: false }">
  <button @click="open = !open">開閉</button>
  <div x-show="open">
    <div hx-get="/details" hx-trigger="revealed">読み込み中...</div>
  </div>
</div>
```

- 開閉トグル：Alpineが即時処理（往復ゼロ）
- 中身取得：HTMXが必要な時だけサーバへ

### 個別領域の vanilla 選択肢

- グラフ → Chart.js / D3
- ドラッグ&ドロップ → SortableJS（3KB）
- リッチテキスト → TipTap / Quill / CodeMirror
- お絵かき/Canvas → Konva / Fabric.js
- 3D → Three.js
- 共同編集 → Yjs / Automerge（CRDT、FW非依存）
- データグリッド → Handsontable / AG Grid（vanilla版あり）

### React が本当に必要な領域

- Notion / Linear クラスの「アプリそのものがブラウザ内で動く」系
- 複雑なフォーム＋複雑な状態遷移

それ以外、特に **CRUD・管理画面・コンテンツ表示** は HTMX + Alpine が読みやすく速い。

---

## 10. キャッシュ無しリロードでも16ms以下に収まる仕組み

**結論：ブラウザが並列＋投機的に取得して、接続コストも再利用するので、CSS/JSを並列に7ms程度で取り切る。**

DevToolsの「Disable cache」はHTTPキャッシュだけを切るもので、それ以外の最適化は生きている：

| 層 | キャッシュOFFで残るか | 効果 |
|---|---|---|
| HTTPキャッシュ（ディスク/メモリ） | × 切れる | リソース再ダウンロードが必要 |
| DNSキャッシュ（OS/ブラウザ） | ◯ 残る | IP引きが0ms |
| TLSセッション再開 | ◯ 残る | ハンドシェイクが1RTT短縮 |
| HTTP/2コネクション再利用 | ◯ 残る | TCP+TLS確立コストゼロ |
| Preload Scanner（投機的取得） | ◯ 関係なし | HTMLパース中に `<link>`/`<script>` を見つけ次第fetch開始 |

つまり「ダウンロードバイト数」だけリセットされて、**接続コストはすでに払い済み**。これが7msの正体。

### Preload Scanner（地味にすごい）

普通のパーサ：HTML読む → `<script>` 見つけた → fetch → 待つ → 続き読む
現代ブラウザ：
```
[メインパーサ]   <html>...<head>...
[Preload Scanner] 先読みで <link rel=stylesheet> 発見 → 即fetch開始
                  並列で <script src=...> 発見 → 即fetch開始
[メインパーサ]   ...DOMtreeを組み立てつつ受信完了を待つ
```
Chrome 2008年、Safari 2010年代から標準動作。

### 含意

**SPAで巨大バンドルを作って独自にコード分割するより、ブラウザに素直に並列取得させた方が速い**ケースが多い。HTMX + CDN構成が思いのほか速いのは、「**ブラウザに仕事を任せる**」設計の効果でもある。

### 本当の「コールドスタート」を測るには

シークレットウィンドウ + DNSキャッシュフラッシュ（macOS: `sudo dscacheutil -flushcache`）+ ネット切断/再接続。それでもHTTP/3対応CDN（jsdelivrは対応）なら0-RTT再開で1往復目から速い。

---

## 11. コード量が最低限で済む

**結論：このサンプル、設定ファイル30行・コード572行で完結。Reactで同等の物を作ると「本質じゃない設定・boilerplate・意味不明なconfig」が多くを占める。**

### このプロジェクトの実測（2026-05-17時点）

```
src/main.rs                          226  (テスト含む)
src/controller.rs                     93
src/usecase.rs                        57
src/service.rs                        64
src/model.rs                           8
templates/base.html                   18
templates/index.html                  36
templates/partials/todo_row.html      21
templates/partials/todo_edit_row.html 21
Cargo.toml                            28
.gitignore                             2
────────────────────────────────────
合計                                  574 行
```

**設定ファイルは Cargo.toml と .gitignore の2つだけ。**
webpack/vite/babel/postcss/eslint/prettier/tsconfig/jest/playwright/storybook... 全部不要。

### React+Next.js で同等品を作ると典型的に発生するもの

| 種別 | 何が必要か |
|---|---|
| 設定ファイル | `next.config.js` / `tsconfig.json` / `.eslintrc` / `.prettierrc` / `postcss.config.js` / `tailwind.config.js` / `next-env.d.ts` |
| 依存パッケージ | `package.json` に50〜100個、`node_modules` 数百MB |
| ビルドツール | webpack/vite/turbopackの選定と設定 |
| 状態管理 | Zustand / Redux / Jotai のいずれかとそのboilerplate |
| データ取得 | TanStack Query / SWR のセットアップ |
| 型生成 | OpenAPI/GraphQL からの型自動生成パイプライン |
| フォーム | react-hook-form / zod のセットアップ |
| テスト | jest / vitest / playwright / msw のセットアップ |

これらの**ほとんどが「本質的なTODO機能」ではなく「フレームワークと共存するための儀式」**で、コードレビューでも実装本体より設定の議論に時間が取られがち。

### Rust + HTMX の構造的な「コードが減る」理由

1. **JSONシリアライズ/デシリアライズが要らない** → DTO型・スキーマ変換コードが消える
2. **クライアント側の状態管理が要らない** → サーバが真実、ブラウザは表示器
3. **ビルドツール設定が要らない** → cargoが全部やる、CDNが配信
4. **コンポーネントprop drilling が要らない** → サーバでHTMLを完成させて返すだけ
5. **型同期パイプラインが要らない** → クライアント/サーバが同じ言語の別ファイルではなく、HTTP越しの別世界

### コード量が少ないと何が良いか

- **読解時間が短い** → 新規参画者の立ち上がりが速い
- **バグの絶対数が減る** → 行数とバグ数は強い正の相関（業界経験則）
- **AIに渡す context が小さい** → AIエージェントが全体を把握しやすい
- **書く時間より考える時間が増える** → 本質的設計に集中できる

「設定で時間を溶かす」「謎エラーで時間を溶かす」体験が少ないのは、長期的なメンテナンスコストにも効いてくる。

---

## まとめ

- **速さ**：HTMX + Rust は「localhost なら1フレーム以下」で白フラッシュ消失。CDN資源もキャッシュOFFで7ms並列取得（Preload Scanner + 接続再利用の恩恵）
- **シンプルさ**：SPA系の対処策（GraphQL/BFF/RSC/tRPC）が複雑さを別の場所へ移すのに対し、HTMX は層そのものを抜く
- **コード量**：本サンプル574行・設定ファイル2つ。Reactで同等品を作る際に発生する「本質じゃない設定/boilerplate」が構造的に発生しない
- **AI親和性**：ローカリティの高さ・学習データの厚み・小さいcontextで生成成功率が高い
- **守備範囲**：CRUD・管理画面が本領。リアルタイム重UIには React 等が必要だが、軽UIなら Alpine + HTMX（30KB）で十分
- **思想**：HTML+HTTPという30年枯れた仕様に乗ることで、長期メンテナンス性も得られる
