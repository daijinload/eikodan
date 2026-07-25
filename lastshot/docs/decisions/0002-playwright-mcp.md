# 0002. Playwright MCP — 不採用（agent-browser と Playwright 本体に役割分担）

- **判断**: 不採用（2026-06-14）
- **検証場所**: [`../../../playwright-sample/bench/`](../../../playwright-sample/bench/)（warm 30回ループ・外れ値除外）
- **代わりに採ったもの**:
  - 探索的操作・QA・live-reload 中の確認 → **agent-browser**（[`../../browser/README.md`](../../browser/README.md)）
  - 安定した自動テスト・回帰・CI → **Playwright 本体**（`.spec.ts` + auto-wait + trace + codegen）

## 実測（warm・トリム後 / 2026-06-14 / Apple Silicon ローカル）

同一シナリオ（1ステップ＝「状態確認スナップショット → +1 クリック」）で比較。
ツール／サーバ側のレイテンシのみで、AI モデルのターンは含まない。

| 指標 | Playwright MCP | agent-browser |
|---|---|---|
| 1ステップ wall-clock | 533 ms | **16.5 ms** |
| 1ステップ出力バイト | 923 B（≒230 tok） | **110 B（≒28 tok）** |
| cold 起動（一度きり） | 582 ms | 783 ms |

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

## 評価中に踏んだ罠

- `@playwright/mcp` は `npx` 実行で独自のブラウザキャッシュを持ち、初回 `browser_navigate` で
  `chrome-for-testing is not installed` になる。`npx @playwright/mcp install-browser chrome-for-testing`
  （~260MB）が別途一度必要。
- `browser_click` の必須引数は `ref` ではなく **`target`**（ref 文字列）。誤ると毎回引数エラーで即時失敗し、
  **「0.1ms で速い」と誤読する**。ベンチは最終 count を検証して防いだ。

詳細な比較表と計測条件は [`playwright-sample/README.md`](../../../playwright-sample/README.md) に残してある。
