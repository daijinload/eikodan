# 仕様: カウンター（ブラウザ駆動）

`spec_counter.ts` と 1:1 で対応する仕様書。**この md が契約の正本**で、テストはその実行可能な形。
テストを変えるときは先にここを変える（md が動かずテストだけ動く diff は事故の兆候 ── `../../docs/testing.md`）。

## この層は何をテストしているのか

lastshot の核は**スキーマ＝単一の真実**。同じ生成型 `CounterView` の**1インスタンス**が3つの出口に流れる。

1. 画面の数字（`#count` の DOM テキスト）
2. 応答に埋め込まれた `<!-- view-data -->` の JSON
3. Connect API（`GetCount` / `Increment`）の JSON

データ取得は1回・出口は複数なので、**この3つは常に一致する**はず。ここで検証するのは
**その一致が実ブラウザ・実 htmx.js・実 API でも成り立つこと**だけ。

### 担当すること

- **JS が実際に動いた後の DOM**。htmx.js による swap の結果を見る（HTTP 層は JS を実行しないので確認できない）。
- **3経路の値の突き合わせ**。1経路だけ壊れる ＝ 「データ取得は1回」が崩れた合図。

### 担当しないこと（どこが持っているか）

md を二重に持たないため、以下はここに書かない・ここでは検証しない。

| 事柄                                                     | 担当                                                            |
| -------------------------------------------------------- | --------------------------------------------------------------- |
| HTML・フラグメントの文字列としての不変条件               | [`../../tests-http/tests/spec_counter.md`](../../tests-http/tests/spec_counter.md) C-1〜C-3 |
| view-data コメントの埋め込み位置（フル=末尾 / 断片=先頭） | `crates/webcore/tests/unit_render_view.rs`                      |
| proto3 JSON が 0 値を省略すること                        | `crates/schema/tests/unit_json_contract.rs`                     |
| 探索的・その場限りのブラウザ操作                         | agent-browser（[`../../docs/decisions/0002-playwright-mcp.md`](../../docs/decisions/0002-playwright-mcp.md)） |

## 用語

`.proto` に現れる語は **`crates/schema/proto/counter.proto` が正典**。
「カウント値」「view-data」「フラグメント」は
[`../../tests-http/tests/spec_counter.md`](../../tests-http/tests/spec_counter.md) の定義に従う（用語集を二重に持たない）。
ここで補うのはブラウザ層にしか出てこない語だけ。

| 用語     | 定義                                                                                             |
| -------- | ------------------------------------------------------------------------------------------------ |
| **swap** | htmx が応答のフラグメントで対象要素の中身を差し替える動作。`hx-target="#count"` の `innerHTML` 置換 |

## 前提条件

- アプリが起動済みで `BASE_URL`（既定 `http://127.0.0.1:3000`）で応答する（`./run db-setup && ./run dev`）。
- **テストは直列に走る。** `playwright.config.ts` の `workers: 1` / `fullyParallel: false`。
  カウント値は `counter` テーブルの唯一の行なので、並列化すると値が取り違わる。
- 上の直列実行を根拠に、**この層だけは厳密な +1 を要求できる**（HTTP 層 C-2 は単調増加までしか言えない）。
  裏を返すと、**実行中に人間が画面の `+1` を押す・別セッションが叩くと落ちる。**
  落ちたときは「実装が壊れた」より先に「前提が崩れた」を疑うこと。
- `./run test` は `test-http` → `browser` の順に**逐次**実行する。並行させると B-2 が壊れる。

---

## B-1. トップページは3経路が同じ値を指す

`GET /` を実ブラウザで開いたとき、以下をすべて満たす。

- 応答が 2xx で、`#count` が可視である。
- `+1` ボタンが `hx-post="/increment"` と `hx-target="#count"` を持つ
  ＝ 部分更新の配線が DOM に出ている（属性が消えてもページは描画できてしまうので明示的に見る）。
- **埋め込み view-data の値 == 画面の数字。**
- **`POST /counter.v1.CounterService/GetCount` の値 == 画面の数字。**

**境界値**: カウント値が **0** のとき、埋め込み JSON も API の JSON も `value` キーを**持たない**
（proto3 JSON は 0 値を省略する ── `crates/schema/tests/unit_json_contract.rs` で固定）。
**キーの欠落は 0 とみなす**こと。`value` の存在を条件にしてはいけない。

## B-2. `+1` クリックで画面・フラグメント・API が揃って +1 する

`+1` ボタンをクリックしたとき、クリック前の表示値を `before` として:

- htmx が `POST /increment` を送り、その応答（フラグメント）の view-data の値が **`before + 1`**。
- **swap 後の `#count` のテキストが `before + 1`** ＝ htmx の配線が実際に機能している。
- 直後の `GetCount` が返す値も **`before + 1`** ＝ 値が DB に永続化され、HTML 経路と API 経路が同じ行を共有している。

**境界値**: ここだけ**厳密な +1** を要求する（HTTP 層の C-2 は単調増加）。差の理由は前提条件の直列実行。
**並列化・並行実行を入れたくなったら、先にこの契約を単調増加へ緩めること。**
逆順にすると原因の分かりにくい Flaky になる。

---

## この仕様が守っている設計判断

- **サーバは同梱せず「待機」する**: アプリ本体をビルド/リンクせず、起動済みサーバに対して叩くだけ
  （`tests-http` と同じ方針。アプリを変えてもこのパッケージは無関係＝疎結合で速い）。
- **ブラウザが要るのはこの層だけ**: htmx の「HTML 断片を返すただの POST」としての契約は HTTP で検証できるので
  HTTP 層に置き、ここは **JS 実行と swap が絡む部分だけ**に絞る。ブラウザ E2E を薄く保つほど壊れにくい。
- **回帰テストは Playwright 本体、探索は agent-browser**
  （[`../../docs/decisions/0002-playwright-mcp.md`](../../docs/decisions/0002-playwright-mcp.md)）。
