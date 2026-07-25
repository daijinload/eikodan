# browser — ブラウザ駆動E2E（Playwright）

lastshot の核（**スキーマ＝単一の真実**）を実ブラウザで検証する。同じ生成型 `CounterView` が

1. 画面の数字（`#count`）
2. 末尾に埋め込まれた `<!-- view-data -->` の JSON
3. Connect API（`GetCount` / `Increment`）の JSON

の3経路に流れる。データ取得は1回・出口は複数なので3つは常に一致するはず ── それを
**実ブラウザ + 実 HTMX swap + 実 API** で突き合わせるのがこのテスト。

**契約の正本は [`tests/spec_counter.md`](./tests/spec_counter.md)**、`tests/spec_counter.ts` はその実行可能な形
（この層が何を検証し、何を検証しないかも md にある）。**期待値を変えるときは先に md を変えること。**
配置規約の全体像は [`../docs/testing.md`](../docs/testing.md)、機械チェックは `./run spec-check`。

> ファイル名は Playwright 既定の `*.spec.ts` ではなく **`spec_<name>.ts`**（Rust 側と同じプレフィックス）。
> `playwright.config.ts` の `testMatch` で拾っているので、**この形を外した `.ts` は実行されない**。

採否の判断は [`../docs/decisions/0002-playwright-mcp.md`](../docs/decisions/0002-playwright-mcp.md)（Playwright 採用・MCP 不採用）。
AI からの探索的操作は agent-browser に役割分担し、**安定した回帰テストは Playwright 本体**で書く方針。

## 位置づけ（tests-http との違い）

| | tests-http/ | browser/（これ） |
|---|---|---|
| 何で叩くか | reqwest（HTTP 直） | 実ブラウザ（Chromium） |
| 検証対象 | HTML 文字列・API JSON | **HTMX の実 swap・DOM 表示**・view-data・API の一致 |
| JS 実行 | しない | する（htmx.js が実際に動く） |

どちらも **「サーバは別で起動しておき、それに対して叩く」**（アプリ本体をビルド/同梱しない＝疎結合・速い）。

## セットアップ（初回だけ）

```sh
./run browser-setup        # = (cd browser && npm install && npx playwright install chromium)
```

## 実行

サーバを先に起動しておくこと（DB 必須）:

```sh
./run db-start && ./run db-setup && ./run dev   # 別ターミナルで起動したまま
./run browser                                    # = (cd browser && npm test)
```

接続先は既定 `http://127.0.0.1:3000`。別ポート/CI は `BASE_URL` で上書き:

```sh
BASE_URL=http://127.0.0.1:3001 npm test
```

その他のコマンド:

```sh
npm run test:headed   # ブラウザを表示して実行
npm run test:ui       # UI モード（対話的に選んで実行）
npm run report        # 直近の HTML レポートを開く
```
