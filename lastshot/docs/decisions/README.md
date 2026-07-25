# 採否の記録（何を採って、何を採らなかったか）

**不採用の判断も一級の知見**として残す。理由と実測値がないと、半年後に同じことをもう一度試すため。

「これ試したっけ？」と思ったら**まずこの表**を見る。詳細な数字は右端のリンク先にある。

---

## 不採用（試した／調べた上で採らなかった）

| #   | 対象                                                    | なぜ採らなかったか                                                                                                                                                                                          | 根拠                                                               |
| --- | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| 1   | **subsecond ホットパッチ**                              | axum 素組では動かず、使うには Dioxus への移行が要る。制約も重い（struct レイアウト変更は不可・tip crate のみ・experimental）                                                                                | [`0001-subsecond-hotpatch.md`](./0001-subsecond-hotpatch.md)       |
| 2   | **Playwright MCP**                                      | 出力トークンが agent-browser の約8倍。安定テストは Playwright 本体で足りる                                                                                                                                  | [`0002-playwright-mcp.md`](./0002-playwright-mcp.md)               |
| 3   | **dev profile の opt 非対称**（依存=opt3 / 自前=opt0）  | 再計測したら dev の評価軸は反復速度のみで足り、本番速度は release が担保する。全クレート opt-0 に統一（フルビルド -45%）                                                                                    | [`0003-dev-profile-opt-level.md`](./0003-dev-profile-opt-level.md) |
| 4   | **rustup toolchain の CI キャッシュ**                   | 復元 ~4s の得に対し**保存 Post が 48s**。さらに 10GB 枠を圧迫して本命の rust-cache を押し出す＝逆効果                                                                                                       | [`../ci-performance.md`](../ci-performance.md) §6                  |
| 5   | **semgrep の pip / pipx venv キャッシュ**               | cold 17s → warm 16s でほぼ縮まない。重さは wheel DL でなく venv 展開。`uv tool install`（~2.5s）で解決済み                                                                                                  | [`../ci-performance.md`](../ci-performance.md) §2                  |
| 6   | **apt パッケージの CI キャッシュ**                      | payload が小さく restore コストで相殺                                                                                                                                                                       | [`../ci-performance.md`](../ci-performance.md)                     |
| 7   | **`sqlx::query!` 系マクロ**                             | コンパイル時に DB 接続を要求し、「DB が無くてもビルドできる」を壊す。ビルド速度が肝なので不採用                                                                                                             | [`../../CLAUDE.md`](../../CLAUDE.md) DB 作法                       |
| 8   | **`wild` リンカ / さらに速いリンカ探し**                | lld が既に最速で詰めしろ無し（lld ≈ apple-ld）。`wild` は Linux 専用                                                                                                                                        | [`../cold-start.md`](../cold-start.md) §③                          |
| 9   | **playground crate 方式**                               | 本体との結合が増えると playground だけで完結しなくなる。package by feature で同等の効果が得られた                                                                                                           | [`0001-subsecond-hotpatch.md`](./0001-subsecond-hotpatch.md)       |
| 10  | **compose 分割 vs 全部入り single の CI 比較**          | 保留。単一コンテナが速い理由は tmpfs + `fsync=off` でありコンテナ構成とは無関係、と分かった時点で優先度が落ちた                                                                                             | [`../container-ops.md`](../container-ops.md)                       |
| 11  | **Oxlint / Biome の linter**                            | JS/TS 専用で Rust 中心の構成では空振りする。品質担保は型チェック + clippy に集約                                                                                                                            | [`../lint-format.md`](../lint-format.md)                           |
| 12  | **`make` / `just` などのタスクランナー**                | `./run`（bash 関数 + ディスパッチャ）で足り、追加インストールがゼロになる                                                                                                                                   | [`../task-runner.md`](../task-runner.md)                           |
| 13  | **worktree ポートの自動採番**（空き探索／名前ハッシュ） | 空き探索は起動のたびに `dan1` のポートが変わり「今どれだっけ？」になる。名前ハッシュは固定だが `dan1`→3725 と覚えられず、衝突もありうる。worktree 名の末尾数字を必須にし、無ければ fail-fast する形を採った | [`../../README.md`](../../README.md) worktree 並列起動             |

## 採用（比較検討の末に選んだもの）

| 対象                                                                  | 採った理由                                                                                                                                                                               | 根拠                                                 |
| --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| **HTMX（SPA ではなく）**                                              | Rust 側 MiniJinja でテンプレを持てば UI 変更がビルドゼロ。白フラッシュも 1 フレーム内で見えない                                                                                          | [`../htmx-vs-spa.md`](../htmx-vs-spa.md)             |
| **ネイティブ PG + unix ソケット（開発時）**                           | in-memory 系（PGlite / pg-mem）より実効速度が速く、本番と同じ Postgres が使える                                                                                                          | [`../postgres.md`](../postgres.md)                   |
| **CSS は日常 CDN / 最終確認だけビルド**                               | 日常の Tailwind watch を丸ごと不要にできる。パージ事故は push 前の semgrep ゲートで捕まえる                                                                                              | [`../hot-reload.md`](../hot-reload.md)               |
| **CI runner を `ubuntu-24.04-arm` に**                                | 総時間はほぼ互角だが、ローカル（Apple Silicon）とのアーキ一致がノーコストで得られる                                                                                                      | [`../ci-performance.md`](../ci-performance.md) §1    |
| **`Swatinem/rust-cache`**                                             | build ~76s → ~15s。最大の効き。「少数の太いキャッシュ」の本命                                                                                                                            | [`../ci-performance.md`](../ci-performance.md) §5    |
| **Playwright Chromium の CI キャッシュ**                              | 当初却下 → 再計測で採用。正味 ~10s + CI 総時間のブレの主因（Chromium DL 変動）を除去                                                                                                     | [`../ci-performance.md`](../ci-performance.md) §8    |
| **agent-browser（探索・QA 用）**                                      | 1ステップ 16.5ms / 110B。複数操作を1ターンに束ねられる                                                                                                                                   | [`0002-playwright-mcp.md`](./0002-playwright-mcp.md) |
| **`./run`（自作 bash ディスパッチャ）**                               | ツール追加ゼロで fail-fast・依存の重複排除まで入る。一般論では `just` が最有力                                                                                                           | [`../task-runner.md`](../task-runner.md)             |
| **種別ごとに lint/format を使い分け**                                 | 1本で全種別は賄えない。rustfmt+clippy / oxfmt / buf / shfmt / sqlfluff の組み合わせ                                                                                                      | [`../lint-format.md`](../lint-format.md)             |
| **Flyway でマイグレーション管理**                                     | ローカルに JRE/CLI を入れず docker image で完結。タイムスタンプ版数でブランチ並行時の衝突を避けられる                                                                                    | [`../../CLAUDE.md`](../../CLAUDE.md) DB 作法         |
| **`./run ci` の停止はポート基準の `kill -9`**（PID ファイルではなく） | ポートは worktree 名から一意に決まるので PID を覚える必要が無く、前回の残骸も掃除できる。CI はこのタスクを使わず、dev と ci を同時に起動もしないので、取り違えも graceful 停止も要らない | [`../../README.md`](../../README.md) CI / コンテナ   |

---

## 書き方

- **短い判断は上の表の1行で完結させる。** 根拠列から既存ドキュメントの節にリンクすれば十分。
- **個別ファイルを切るのは、根拠が長い／その判断のためだけに計測した場合。** `NNNN-slug.md` で連番。
- **判断は更新されうる。** 一度却下したものが再計測で採用に転じることがある（Playwright キャッシュが実例）。
  ひっくり返ったら**元の判断を消さず、経緯を追記する**。
