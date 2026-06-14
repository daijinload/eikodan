# playwright-sample

Microsoft [Playwright](https://playwright.dev/) の E2E テストのサンプルです。
AI エージェントからのブラウザ操作は [agent-browser](https://github.com/vercel-labs/agent-browser)（ルート `README.md`）に役割分担し、
Playwright MCP は評価のうえ**不採用**としました（理由は[下の比較](#playwright-mcp-を評価して外した経緯)）。

## セットアップ

```sh
cd playwright-sample
npm install
npm run install:browsers   # Chromium をダウンロード（初回のみ）
```

## 実行コマンド

```sh
npm test            # ヘッドレスで全テスト実行（オフライン可）
npm run test:headed # ブラウザを表示して実行
npm run test:ui     # UI モード（対話的にテストを選んで実行）
npm run report      # 直近の HTML レポートを開く
npm run codegen     # 操作を記録してテストコードを自動生成
```

サンプルテストは `tests/example.spec.ts`。ネットワーク不要（`page.setContent` で DOM を組み立てて検証）なのでオフラインでも通ります。

### subsecond-demo を録画する

`subsecond-demo` を Playwright で操作し、webm 動画＋最終スクショを `videos/` に出力します（`scripts/drive-subsecond.mjs`）。
先に別ターミナルで dev サーバを起動しておくこと:

```sh
cd ../subsecond-demo && dx serve --port 8080   # 別ターミナルで起動したまま
npm run demo:subsecond                          # playwright-sample/ で実行 → videos/ に出力
```

## 使い分け（このリポジトリの結論）

- **安定した自動テスト / 回帰 / CI → Playwright 本体**（このサンプル。`.spec.ts`＋auto-wait＋trace＋codegen）。
- **探索的・QA・「開発しといて」的な反復 / live-reload 中の確認 → agent-browser**（速い・低トークン・複数操作を1ターンに束ねられる）。
- 今回 MCP を遅くした「アクション後の settle 待ち」は、安定テストでは“待ってくれて flaky になりにくい”長所と表裏一体。決定性はレイテンシと引き換え。

## Playwright MCP を評価して外した経緯

AI エージェントから Playwright を MCP 経由で操作する [`@playwright/mcp`](https://github.com/microsoft/playwright-mcp) を
一度 `.mcp.json` に登録し、`subsecond-demo` で検証しました。結論として **MCP は不採用**とし、ブラウザ操作は
agent-browser（探索・QA）と Playwright 本体（安定テスト）に役割分担します。理由は計測（[`bench/`](bench/)）の通り、
**出力トークンが重く、live-reload する dev サーバ相手では1クリックが遅い**こと、そして安定テストは
Playwright 本体（`.spec.ts`＋trace＋codegen）で十分カバーできるためです。

### 比較（warm 30回ループ・外れ値除外 = cold初回除外＋上下10%トリム）

同一シナリオ（`subsecond-demo` で 1ステップ＝「状態確認スナップショット → +1 クリック」）を agent-browser と比較。
**ツール／サーバ側のレイテンシのみ**で、AIモデルのターンは含みません（2026-06-14 / agent-browser 0.27.3 / `@playwright/mcp` / Apple Silicon・ローカル実測）。

| 指標（warm, trim後） | Playwright MCP | agent-browser |
| --- | --- | --- |
| 1ステップ wall-clock | **533 ms** | **16.5 ms** |
| ├ snapshot | 2.7 ms | 4.7 ms |
| └ click | **530 ms**（下記※） | 11.8 ms |
| 参考: click（静的ページ） | **0.2 ms** | 同程度 |
| 1ステップ出力バイト | **923 B**（≒230 tok） | **110 B**（≒28 tok） |
| ├ snapshot | 687 B（a11yツリー全体） | 101 B（`-i` interactiveのみ） |
| └ click 結果 | 236 B | 9 B |
| cold 起動（一度きり） | 582 ms | 783 ms |

- **トークン消費は agent-browser が約8倍軽い**（110 B vs 923 B/ステップ）。MCP の `browser_snapshot` は a11y ツリー全体が既定、かつツール定義 ~25 個もコンテキストに載るため。ページに依らない一般差。MCP 側も `target`/`depth` で絞れるが既定は冗長。
- **速度は「条件付き」**。subsecond-demo では MCP の click が ~530ms で一定、agent-browser は ~12ms（約32倍差）。ただし **※ これは MCP 固有の遅さではない** — 静的ページでは MCP click は **~0.2ms**。MCP がアクション後にページの settle（ネットワーク静穏化）を待つ挙動が、**Dioxus dev サーバの hot-reload 由来の背景通信**と噛み合って毎回 ~500ms 効くため。静的／本番ビルド相手なら click 速度差はほぼ消える。
- **cold 起動は両者 ~0.6–0.8s で同程度**。差は「初回だけ」ではなくウォームでも継続（＝「初回外れ値の疑い」は否定された）。
- **別軸（実運用で効く）**: MCP は1操作=1モデルターンだが、agent-browser は複数コマンドを1回の Bash にまとめられる（=1ターン）。多操作ほど体感差が開く。

> 評価中に踏んだ注意点（記録）:
> - `@playwright/mcp` は `npx` 実行で**独自のブラウザキャッシュ**を持ち、初回 `browser_navigate` で `chrome-for-testing is not installed` になる。`npx @playwright/mcp install-browser chrome-for-testing`（~260MB）が別途一度必要。
> - `browser_click` の必須引数は `target`（ref文字列）で `ref` ではない。誤ると毎回引数エラーで即時失敗し、count が増えないまま「0.1ms で速い」と誤読する（bench は最終 count を検証して防止）。
> - Dioxus dev サーバの「Your app is being rebuilt.」トーストが MCP 側ブラウザにだけ残ることがあるが、クリックは下のボタンに通り無害。
