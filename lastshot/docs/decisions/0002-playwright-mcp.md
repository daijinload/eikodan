# 0002. Playwright MCP — 不採用（agent-browser と Playwright 本体に役割分担）

- **判断**: 不採用（2026-06-14）
- **代わりに採ったもの**:
  - 探索的操作・QA・live-reload 中の確認 → **agent-browser**
  - 安定した自動テスト・回帰・CI → **Playwright 本体**（`.spec.ts` + auto-wait + trace + codegen。
    lastshot での配線は [`../../browser/README.md`](../../browser/README.md)）

## 実測（warm・トリム後 / 2026-06-14 / Apple Silicon ローカル）

同一シナリオ（1ステップ＝「状態確認スナップショット → +1 クリック」）で比較。
ツール／サーバ側のレイテンシのみで、AI モデルのターンは含まない。

| 指標                  | Playwright MCP    | agent-browser        |
| --------------------- | ----------------- | -------------------- |
| 1ステップ wall-clock  | 533 ms            | **16.5 ms**          |
| 1ステップ出力バイト   | 923 B（≒230 tok） | **110 B（≒28 tok）** |
| cold 起動（一度きり） | 582 ms            | 783 ms               |

## なぜ採らなかったか

1. **トークンが約8倍重い（110 B vs 923 B/ステップ）。** MCP の `browser_snapshot` は a11y ツリー全体が
   既定で、さらにツール定義 ~25 個が常時コンテキストに載る。**これはページに依らない一般差**で、
   エージェント運用のコストに直接効く。`target`/`depth` で絞れはするが既定が冗長。
2. **1操作＝1モデルターンになる。** agent-browser は複数コマンドを1回の Bash にまとめられる（＝1ターン）。
   操作数が増えるほど体感差が開く。
3. **安定テストは Playwright 本体で足りる。** MCP を挟む必要がそもそも無い。

## 誤読しかけた点（記録として重要）

**「MCP の click が 530ms で遅い」は MCP 固有の欠点ではない。** 静的ページでは MCP の click は **~0.2ms**。
MCP がアクション後にページの settle（ネットワーク静穏化）を待つ挙動が、**Dioxus dev サーバの
hot-reload 由来の背景通信**と噛み合って毎回 ~500ms 効いていた。静的／本番ビルド相手なら click の速度差は
ほぼ消える。

しかもこの settle 待ちは、**安定テストの文脈では「待ってくれるので flaky になりにくい」長所**と表裏一体。
決定性はレイテンシと引き換え、という一般論に落ちる。**不採用の決め手はあくまでトークンコストと
ターン数**であって、この 530ms ではない。

## 計測方法（再現したくなったら）

同一シナリオを両ツールでウォーム状態のまま 30 回ループし、**1操作あたりの wall-clock と出力バイト数**を測る。
外れ値は「`cold`（初回起動）を除外 ＋ 上下10%トリム」で落とす。

- **MCP 側**: `@playwright/mcp` を **stdio JSON-RPC で直接駆動**し、`tools/call` の往復時間と結果本文バイト数を測る。
- **agent-browser 側**: `agent-browser <cmd>` を**毎回プロセス起動**して呼ぶ
  （CLI の起動コスト込み ＝ エージェントが実際に払うコスト）。
- バイト数は「エージェントが読む本文の実バイト」。トークンは概ね `bytes / 4` 程度。
- 各 bench は終了時に**最終 `count` を検証**する（`expected == 実測` ならクリックが空振りしていない証拠）。

## 評価中に踏んだ罠

- **`browser_click` の必須引数は `ref` ではなく `target`**（ref 文字列）。`ref` を渡すと毎回「引数エラー」で
  即時 0.1ms 失敗し、count が増えないのに**「速い」と誤読する**。→ 最終 count の検証はこれを防ぐために入れた。
- `@playwright/mcp` は `npx` 実行で独自のブラウザキャッシュを持ち、初回 `browser_navigate` で
  `chrome-for-testing is not installed` になる。`npx @playwright/mcp install-browser chrome-for-testing`
  （~260MB）が別途一度必要。
- MCP は navigate 直後だと WASM 未描画で、サーバ描画の「rebuilt」トーストしか取れない。
  `browser_wait_for {text:"+1"}` でアプリ描画を待ってからループに入る。
