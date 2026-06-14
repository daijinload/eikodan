# lint-format

**「対象ファイル種別ごとに最適な lint / format ツールは違い、1本で全部は賄えない」** を、
種別ごとのサンプルファイルで実演する showcase。実プロジェクトに組み込む前に、
ここで各ツールの設定と挙動（整形結果・lint 指摘）を単体で確かめられる。

`samples/` に1種別1ファイルずつ置き、ルートの設定ファイルとゲートスクリプトで一括検査する。

## 何に何を当てるか

| 対象             | サンプル                         | Formatter                  | Linter                           | 取得                                 |
| ---------------- | -------------------------------- | -------------------------- | -------------------------------- | ------------------------------------ |
| Rust             | `samples/rust/`                  | **rustfmt**                | **clippy**                       | toolchain 同梱                       |
| TOML/YAML/MD/CSS | `samples/config.{toml,yml}` ほか | **oxfmt**                  | —                                | node（`.lint-tools` にローカル固定） |
| HTML/Jinja       | `samples/page.html`              | **oxfmt**（Tailwind 整列） | （実プロジェクトでは描画で担保） | 同上                                 |
| proto            | `samples/proto/greet.proto`      | **buf format**             | **buf lint**                     | brew（単一バイナリ）                 |
| shell            | `samples/script.sh`・本ゲート    | **shfmt**                  | **shellcheck**                   | brew（単一バイナリ）                 |

## 使い方

```sh
bash setup-lint.sh   # ツール取得（rustfmt/clippy 確認、oxfmt を .lint-tools にローカル固定、buf/shfmt/shellcheck を brew）
bash check-lint.sh   # 全種別を通しで検査（1つでも落ちたら非ゼロ終了。CI にもそのまま使える）
```

整形を当てる（書き込み）には各ツールを個別に:

```sh
( cd samples/rust && cargo fmt --all )
.lint-tools/node_modules/.bin/oxfmt .
( cd samples/proto && buf format -w )
shfmt -i 2 -w ./*.sh samples/*.sh
```

## 設計上の判断

- **lint の主役は clippy。** Oxlint / Biome の linter は **JS/TS 専用**で、Rust 中心の構成では空振りになる
  （Rust は lint できない）。Rust の型チェック + clippy を品質担保の中心に据え、**Oxlint/Biome は入れない**。
- **oxfmt は Rust 以外をほぼ1本で整形**できる（TOML/YAML/MD/CSS、Tailwind クラス整列内蔵）。napi 製で
  node が要るが、**開発専用ツールでアプリの実行/ビルド経路には入らない**（`.lint-tools/` は `.gitignore` 済み）。
- **HTML/Jinja も oxfmt に含める**（Tailwind クラス整列目当て）。oxfmt は Jinja を公式サポートしていないが、
  実テンプレ相当（whitespace 制御マーカー不使用・タグは属性外）の `samples/page.html` で**逐語保全・冪等**を確認できる。
  `{% if %}`/`{{ ... }}` は触らず、`class="..."` の中だけ並べ替わる。論理検査は実プロジェクト側の描画テストに任せる。
- **oxfmt が埋められない proto / shell** は単一バイナリの buf / shfmt+shellcheck で補う。
  shfmt は既定がタブなので**2スペースに合わせ `-i 2`** を付ける。

## 設定の置き場

- Rust 整形ルール（edition / import 整理。unstable オプションは nightly 必須）: `rustfmt.toml`
- oxfmt（Tailwind 整列 / 除外パターン）: `.oxfmtrc.json`
- proto の lint / format ルール: `samples/proto/buf.yaml`
- Rust サンプルの toolchain 固定（nightly）: `samples/rust/rust-toolchain.toml`
- ツール取得 / ゲート: `setup-lint.sh` / `check-lint.sh`
