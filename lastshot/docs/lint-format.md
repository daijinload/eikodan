# lint / format ツールの選定根拠

**「対象ファイル種別ごとに最適な lint / format ツールは違い、1本で全部は賄えない」**という結論に至るまでの判断記録。
lastshot での**実際の配線**は [`../lint/README.md`](../lint/README.md)（どのファイルに何を当てるか・設定の置き場）。
ここは**なぜそのツールを選んだか**を残す。

> 出自: `lint-format/` showcase（種別ごとに1サンプルファイルを置いて各ツールの挙動を単体確認した実験場）。
> 結論は全てここに取り込み済み。

## 何に何を当てるか（結論）

| 対象             | Formatter                  | Linter               | 取得                                 |
| ---------------- | -------------------------- | -------------------- | ------------------------------------ |
| Rust             | **rustfmt**                | **clippy**           | toolchain 同梱                       |
| TOML/YAML/MD/CSS | **oxfmt**                  | —                    | node（`.lint-tools` にローカル固定） |
| HTML/Jinja       | **oxfmt**（Tailwind 整列） | （描画テストで担保） | 同上                                 |
| proto            | **buf format**             | **buf lint**         | brew（単一バイナリ）                 |
| shell            | **shfmt**                  | **shellcheck**       | brew（単一バイナリ）                 |
| SQL              | **sqlfluff**               | **sqlfluff**         | brew（Python・依存ごと配布）         |

## 判断とその理由

- **lint の主役は clippy。Oxlint / Biome の linter は入れない。**
  両者の linter は **JS/TS 専用**で、Rust 中心の構成では空振りになる（Rust は lint できない）。
  品質担保の中心は **Rust の型チェック + clippy** に据える。
- **oxfmt は Rust 以外をほぼ1本で整形できる**（TOML/YAML/MD/CSS、Tailwind クラス整列が内蔵）。
  napi 製で node が要るが、**開発専用ツールでアプリの実行/ビルド経路には入らない**（`.lint-tools/` は `.gitignore` 済み）。
- **HTML/Jinja も oxfmt に含める**（Tailwind クラス整列が目当て）。oxfmt は Jinja を公式サポートしていないが、
  実テンプレ相当（whitespace 制御マーカー不使用・タグは属性外）のサンプルで**逐語保全・冪等**を確認した。
  `{% if %}` / `{{ ... }}` は触らず、`class="..."` の中だけ並べ替わる。論理検査は描画テストに任せる。
- **oxfmt が埋められない proto / shell** は単一バイナリの buf / shfmt + shellcheck で補う。
  shfmt は**既定がタブ**なので、2スペースに合わせて `-i 2` を付ける。
- **SQL は sqlfluff（format + lint 兼用）。** oxfmt は SQL 非対応なので別建て。`sqlfluff lint` は整形逸脱も
  lint ルールとして検出するので、**ゲートは lint 1本で「整形済み かつ ルール準拠」を確認できる**（書き込みは `sqlfluff fix`）。
  方言は **postgres** 固定。Python 製だが brew が依存ごと配布する。

### md テーブルの整列は diff を太らせる（承知の上で採る）

oxfmt（Prettier 互換）は markdown のテーブルを**列の最大幅にパディングして整列**する。無効化オプションは無い。
`docs/README.md`（10 行の表）で実測すると:

| 編集の種類                    | 再整形後の diff 行数 |
| ----------------------------- | -------------------- |
| セル 1 個を**短く**書き換え   | 2 行（その行だけ）   |
| **既存より長い行**を 1 行追加 | **21 行（表全体）**  |

列幅が変わらない編集はその行しか動かない。**列幅を変える編集をしたときだけ表全体が diff に出る**。
「1 行足したのに表全体が赤緑になる」ので、レビューでは**表の追加行だけを見れば足りる**（残りはパディング）。
これを嫌って md を oxfmt の対象外にする手もあるが、そうすると**表の桁が揃わない状態が放置される**ので
整列を採っている。

## ツール標準ルールを外している箇所（設計側を正とする）

ツールの標準ルールが lastshot の**意図的な設計と衝突する**ときは、設計を正としてルールを除外する。
配線の詳細は [`../lint/README.md`](../lint/README.md)。

- **proto（buf）**: `CounterView` を `GetCount` / `Increment` の両レスポンスで共有するのがスキーマファーストの掟。
  buf STANDARD の `RPC_REQUEST_RESPONSE_UNIQUE` / `RPC_RESPONSE_STANDARD_NAME` はこれを禁じるので `buf.yaml` で除外。
- **SQL（sqlfluff）**: `counter.value` 列名は proto の `CounterView.value`（単一真実）に揃えるのが要件。
  `RF04`（キーワードを識別子に使うな）はこれと衝突するので `.sqlfluff` の `exclude_rules` で除外。

## 運用

`./run lint` は **push 前に節目で手動**で回す（`./run css-check` と同じ運用。pre-commit は使わない）。
理由は軽量な dev ループと最終確認を分けるため ── [`../CLAUDE.md`](../CLAUDE.md) 「イテレーションの回し方」。

**同じ `./run lint` を CI でも回す**（`.github/workflows/lastshot-ci.yml` の `lint` ジョブ）。
手元で回し忘れても main に入らないようにするための最後の網で、**手動運用を置き換えるものではない**
（CI で気付くのは遅すぎる ── 手元で通してから push する前提は変わらない）。

- **`test` ジョブとは `needs` で繋がず並列**にしている。lint は DB もアプリ起動も要らず、
  落ちたときに知りたいことが test と別物だから。直列に積むと「整形ミス1つで E2E まで待たされる」
  「E2E が落ちると整形の結果が見えない」の両方が起きる。
- **ツールのバージョンは CI 側でピン留めする。** ローカルは brew の最新、CI は明示バージョン。
  揃えないと「ローカルは緑なのに CI だけ赤」になるので、**上げるときは両方一緒に上げる**。
- **rustfmt / clippy 以外は brew 配布物を Linux で使えない**ので、CI はリリース配布物を直接置く
  （buf / shfmt / shellcheck）か `uv tool install`（sqlfluff）にしている。oxfmt だけは
  `lint/setup.sh` の `OXFMT_VERSION` が単一の真実なので、CI からも `./run lint-setup` を呼んで重複を避ける。
