# bench — Playwright MCP vs agent-browser 計測

同一シナリオ（`subsecond-demo` で 1ステップ = 状態確認スナップショット → +1 クリック）を
両ツールでウォーム状態のまま N 回ループし、**1操作あたりの wall-clock と出力バイト数**を測る。
外れ値は「`cold`（初回起動）を除外 ＋ 上下10%トリム」で落とす。

## 何を測っているか

- **対象はツール／サーバ側のレイテンシ**（AIモデルのターンは含まない）。
  - `mcp-bench.mjs` … `@playwright/mcp` を stdio JSON-RPC で直接駆動し、`tools/call` の往復時間と結果本文バイト数を測る。
  - `ab-bench.mjs` … `agent-browser <cmd>` を毎回プロセス起動して呼ぶ（CLIの起動コスト込み＝エージェントが実際に払うコスト）。
- バイト数は「エージェントが読む本文の実バイト」。トークンは概ね `bytes / 4` 程度。

## 実行

```sh
# 別ターミナルで対象アプリを起動
cd ../../subsecond-demo && dx serve --port 8080

cd ../playwright-sample/bench
node mcp-bench.mjs http://localhost:8080 30 > mcp.log
node ab-bench.mjs http://localhost:8080 30 > ab.log
node analyze.mjs "Playwright MCP" < mcp.log
node analyze.mjs "agent-browser" < ab.log
```

各 bench は終了時に最終 `count` を stderr に出す（`expected == 実測` ならクリックが空振りしていない証拠）。

## 計測時の落とし穴（実際にハマった点）

- `@playwright/mcp` の `browser_click` の必須引数は **`target`**（ref 文字列）であって `ref` ではない。
  `ref` を渡すと毎回「引数エラー」で即時 0.1ms 失敗し、count が増えないのに「速い」と誤読する。
  → 各 bench は最後に最終 count を検証する。
- MCP は navigate 直後だと Dioxus(WASM) 未描画で、サーバ描画の「rebuilt」トーストしか取れない。
  `browser_wait_for {text:"+1"}` でアプリ描画を待ってからループに入る。

## 測ってわかったこと（要約）

採用判断・比較表は [`../README.md`](../README.md)（「Playwright MCP を評価して外した経緯」）が正。
ここはその根拠となる生の計測メモ:

- **出力量（トークン）**: agent-browser が約 8 倍軽い。MCP の `browser_snapshot` は a11y ツリー全体を返す（既定）一方、
  `agent-browser snapshot -i` はインタラクティブ要素のみを返すため。これはページに依らず一般的に効く差。
- **速度**: subsecond-demo（dev サーバ）相手では MCP の 1 クリックが ~530ms で一定、agent-browser は ~12ms。
  ただし **静的ページでは MCP click は ~0.2ms**。530ms は MCP がアクション後にページの settle（ネットワーク静穏化）を待つ挙動が、
  dev サーバの hot-reload 由来の背景通信と噛み合って毎回 ~500ms 効いているため。
  → 「MCP は遅い」は dev サーバ特有の条件付き。静的/本番ビルドなら click 速度差はほぼ消える。
- **cold 起動**は両者 ~0.6–0.8s で同程度。ステップ間の差は初回だけの現象ではない（ウォームでも継続）。
- これとは別軸で、実運用では **MCP は1操作ごとにモデルのターンが要る**のに対し、
  agent-browser は複数コマンドを 1 回の Bash 呼び出しにまとめられる（=1ターン）。これは上の計測には出ないが、
  体感速度に大きく効く独立要因。
