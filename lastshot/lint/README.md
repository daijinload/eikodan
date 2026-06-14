# lint/ — lastshot の lint / format ゲート

種別ごとに最適な lint / format ツールを当てる構成（1本で全部は賄えない）。
ツール選定の**根拠と単体デモは [`../../lint-format/`](../../lint-format/) showcase**にあり、
ここはその結論を **lastshot の実ファイルに当てる本番配線**。

## 何に何を当てるか

| 対象             | lastshot の対象ファイル                         | Formatter                  | Linter               | 設定ファイル                                                         |
| ---------------- | ----------------------------------------------- | -------------------------- | -------------------- | -------------------------------------------------------------------- |
| Rust             | `crates/**/*.rs`                                | **rustfmt**                | **clippy**           | [`../rustfmt.toml`](../rustfmt.toml)（workspace ルート）             |
| TOML/YAML/MD/CSS | `Cargo.toml` / `compose.yml` / `*.md` / `*.css` | **oxfmt**                  | —                    | [`.oxfmtrc.json`](.oxfmtrc.json)                                     |
| HTML/Jinja       | `crates/**/templates/*.html`                    | **oxfmt**（Tailwind 整列） | （描画テストで担保） | 同上                                                                 |
| proto            | `crates/schema/proto/counter.proto`             | **buf format**             | **buf lint**         | [`../crates/schema/proto/buf.yaml`](../crates/schema/proto/buf.yaml) |
| shell            | `run` / `assets/*.sh` / `lint/*.sh`             | **shfmt**（`-i 2 -ci`）    | **shellcheck**       | （フラグのみ）                                                       |
| SQL              | `migrations/*.sql`                              | **sqlfluff**               | **sqlfluff**         | [`.sqlfluff`](.sqlfluff)                                             |

> JS/TS の linter（Oxlint/Biome）は入れない。Rust 中心の構成では空振りになるため、品質担保の中心は
> **型チェック + clippy**（showcase の結論）。`browser/`（Playwright）は自己完結の別関心事なので oxfmt の対象外。

### 既存の意図的設計に合わせた除外

ツール標準ルールが lastshot の**意図的な設計と衝突する点**は、設計側を正としてルールを外している:

- **proto（buf）**: `CounterView` を `GetCount`/`Increment` の両レスポンスで共有する（スキーマファーストの掟）。
  buf STANDARD の `RPC_REQUEST_RESPONSE_UNIQUE` / `RPC_RESPONSE_STANDARD_NAME` はこれを禁じるので `buf.yaml` で除外。
- **SQL（sqlfluff）**: `counter.value` 列名は proto の `CounterView.value`（単一真実）に揃えるのが要件。
  `RF04`（キーワードを識別子に使うな）はこれと衝突するので `.sqlfluff` の `exclude_rules` で除外。

## 使い方

```sh
./run lint-setup   # ツール取得（初回。oxfmt を lint/.lint-tools にローカル固定、不足分は brew）
./run lint         # 全種別を通しゲート（読み取り専用。1つでも落ちたら非ゼロ終了 = CI 兼用）
```

`./run lint` は **push 前に節目で手動**で回す（`./run css-check` と同じ運用。pre-commit は使わない）。

## 整形を当てる（書き込み）

使い分けがキモ。`cargo fmt` は **差分のあるファイルだけ書き戻す**（整形済みは1バイトも触らない＝mtime
据え置き＝そのクレートは再ビルドされない）。なので「整形＝毎回フルビルド」ではなく、**初回 or 実差分のある
クレートだけ・1回**の話。

- **dev ループ中**は触ったクレート/ファイルだけ個別に（触っていないクレートまで再ビルドさせないため）:
  ```sh
  cargo fmt -p <crate>                                       # 触った crate だけ（--all は避ける）
  lint/.lint-tools/node_modules/.bin/oxfmt -c lint/.oxfmtrc.json <path>
  ( cd crates/schema/proto && buf format -w )
  shfmt -i 2 -ci -w <path.sh>
  sqlfluff fix --config lint/.sqlfluff migrations/<file>.sql
  ```
- **push 前**は一括でよい（どうせ release ビルド/最終確認で全体を1回ビルドするので相乗りさせる）:
  ```sh
  ./run fmt    # 全種別を一括整形（lint/fmt.sh）→ ./run lint を緑にする
  ```

## 設定の置き場（directory-layout.md 準拠）

「フォルダにまとめる」と「ツールの自動探索を壊さない」を両立させるため、**自動探索アンカーだけは
その関心事のルートに残し、それ以外をこの `lint/` に集約**している。

- `../rustfmt.toml` … workspace ルート（直下）。`cargo fmt` / rust-analyzer が crate→workspace を
  上方向に探索するため、ここを動かすと保存時整形まで効かなくなる。**直下に置く唯一のファイル**。
- `../crates/schema/proto/buf.yaml` … proto と同居（proto モジュールの自己完結アンカー）。
- `.oxfmtrc.json` / `.sqlfluff` … 自動探索に頼らず `check.sh` から `-c` / `--config` で明示指定するので
  `lint/` 内に置ける。
- `setup.sh` / `check.sh`（読み取り検査）/ `fmt.sh`（書き込み）… この関心事のスクリプト。
  `lint/.lint-tools/`（oxfmt 本体）は `.gitignore` 済み。
