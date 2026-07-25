# FAST-RUST — Rust 開発を加速するためにやったこと（総まとめ）

lastshot で採った高速化施策の一覧（各実験場での試行を踏まえた「今の結論」）。
**このファイルは施策の総覧＝目次で、実測値そのものは [`build-speed.md`](./build-speed.md) に集約**してある。
「効かなかった理由」は各リンク先、採否の一覧は [`decisions/`](./decisions/)。

## 1. 設計で稼ぐ（そもそもビルドしない）

1. **作業の7〜8割はビルドしない設計** — テンプレ / CSS / HTMX 属性の変更は保存で即反映（Rust ビルドゼロ）。
   詳細は [`hot-reload.md`](./hot-reload.md)。
2. **触った feature だけ再ビルド（package by feature）** — workspace 共通 `Cargo.toml` は触らない
   （`[workspace.dependencies]` やプロファイルを弄ると全クレート再ビルドが走る）。方針は [`CLAUDE.md`](../CLAUDE.md)。
3. **`sqlx::query!` 不使用 + codegen を `schema` クレートに隔離** — コンパイル時に DB 接続を要求せず、
   protoc は proto を触った時だけ走る。dev のビルド時依存を最小化。詳細は [`CLAUDE.md`](../CLAUDE.md) の「DB 作法」。
4. **Rust 変更時は約1秒が底（通常開発の増分ビルドの話）** — 「ファイル1個 touch して `cargo build -p app`」の
   増分ビルドが ~0.6〜0.74s + cold start 0.285s + ブラウザ再描画 ＝ 体感 ~1.2〜1.3s。内訳は
   [`cold-start.md`](./cold-start.md)。**これは増分の話でフルビルドではない**（フルビルドは warm ~5.8〜6.3s /
   true cold 10〜20s ＝ [`build-speed.md` ⑦](./build-speed.md)）。
5. **cold start ~285ms の正体は macOS の起動時セキュリティ検証** — リンカが ad-hoc 署名した「中身の違う新バイナリ」を
   毎リビルド exec する度に `syspolicyd` / `AMFI` / `trustd` が検証する。`codesign -f -s -` で**署名を付け直すと
   初回検証が軽くなって ~100ms 短縮**（検証を完全に消すわけではなく軽くなる）。**唯一効いた起動側の施策**。
   詳細は [`cold-start.md` §①](./cold-start.md)。

## 2. ビルドツール側で稼ぐ（ビルドが走る時に速く）

1. **dev profile は全クレート opt-level=0（フルビルドを最小化）** — `[profile.dev]` 自前=0 /
   `[profile.dev.package."*"]` 依存も=0、`codegen-backend` も両方 `cranelift` に統一。
   以前の「依存だけ opt-level=3」（dev でも実行を速くしたい狙い）は**実測で撤回**した ── フルビルド -45%
   を取り、dev の動作速度は捨てる。**dev profile の評価軸は反復速度のみ・動作速度は release / release-max が担保する**、
   と profile の役割を分けた。判断は [`decisions/0003`](./decisions/0003-dev-profile-opt-level.md)、
   数字は [`build-speed.md` ⑤](./build-speed.md)、release-max（opt-3 + LTO + cgu=1）は [`build-speed.md` ⑥](./build-speed.md)。
2. **開発時は Rust nightly を使う**（`rust-toolchain.toml` で固定。本番は stable）。nightly 限定の高速化を **2つ** opt-in:

   | 機能                                     | 何をする                                                                      | 効くシナリオ                                                                                                                        |
   | ---------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
   | **`codegen-backend = "cranelift"`**      | rustc の **コード生成段** を LLVM より高速化（自前も依存も cranelift に統一） | codegen が支配項になる構成（自前クレートが大きい構成。dev は全クレート opt-level=0 なのでフロント律速だが、それでも保険として残す） |
   | **`-Z threads=8`**（並列フロントエンド） | 1クレート内の rustc 処理（型チェック・マクロ展開・codegen）をスレッド分割     | 1クレートが巨大化したフルビルド（**実証済み: 約2倍速くなる**）                                                                      |

   どちらも **lastshot 規模では効かない**（自前クレートが小さく、並列化・codegen 短縮の余地が無いため）。
   効き方は場面で非対称で、**旨味は true cold（節目）に集中し、日常の warm / 差分ビルドではむしろ微弱に逆**:

   | 場面                    | nightly フル構成 | stable + 素のツール | 差                   |
   | ----------------------- | ---------------- | ------------------- | -------------------- |
   | true cold フルビルド    | 10.58s           | 12.79s              | **nightly -17%**     |
   | warm フルビルド（普段） | 6.05s            | 5.77s               | stable -5%（誤差圏） |
   | 差分ビルド（hot loop）  | 0.78s            | 0.63s               | **stable -19%**      |

   （5 試行 median。裏取りの全数値と理由は [`build-speed.md` ⑦](./build-speed.md)）
   それでも **保険として残す**: `-Z threads` は「巨大1クレートのフルビルドが約2倍速い」が
   [`build-speed.md` ③](./build-speed.md) で実証済みで、クレートが育てば勝手に効き始める。Cranelift は
   理屈上 codegen 支配な構成で効くが、lastshot ではまだ盤面が来ていない。

   **本番ビルドは stable で**やる: `./run release` と Dockerfile が `RUSTUP_TOOLCHAIN=stable` +
   `assets/strip-nightly.sh` で nightly 専用行（`cargo-features` / `codegen-backend`）を剥がす。**dev=nightly / 本番=stable** が掟。

   詳細は [`cold-start.md` §④ (`-Z threads`)](./cold-start.md) / [§⑤ (Cranelift)](./cold-start.md) + [`build-speed.md`](./build-speed.md) ②③。

3. **lld（macOS）／ mold（Linux）でリンク高速化** — `.cargo/config.toml` の target rustflags で配線。
   ただし「もっと速いリンカ」が更に効くわけではない（lld ≈ apple-ld）。詳細は [`cold-start.md` §③](./cold-start.md)。
4. **sccache で重い依存をキャッシュ** — 依存（axum / tokio / buffa…）をキャッシュから返して
   `cargo clean` / 新規 checkout / deps が変わるブランチ切替を短縮。ただし**新構成（全 opt-0）では旨味は当時より小さい**
   ── 依存自体が軽くなったので、30 クレート規模でフル再ビルド -13%、90 クレート規模では **incr ON がむしろ遅くなる**
   （+14%）。**フル再ビルドが多い環境（CI / worktree 切替多用 / 共有キャッシュ）なら `CARGO_INCREMENTAL=0` 込みで -40〜52%**。
   規模と場面で符号が変わる設定なので、採否は実測してから決める（[`build-speed.md` ④](./build-speed.md) 末尾）。
5. **incremental は既定 ON のまま** — 差分ビルドの本体は `incremental`（`CARGO_INCREMENTAL=0` にしない）。
   sccache とは別レイヤなので併用する。lastshot は warm が支配的なのでこの既定が正解（同 ⑦(i)）。

## 3. 採らなかったもの（効かないと実測で確認）

- **ホットパッチ（subsecond / dioxus）** — そもそも axum 素組では動かず、関数本体しか差し替えられない。
  構造変更（フィールド追加・シグネチャ変更・スキーマ変更）で結局フルビルドに戻る。
  判断は [`decisions/0001`](./decisions/0001-subsecond-hotpatch.md)。
- **systemfd で速さを稼ぐ** — 速さは変わらない、接続断を消すだけ。しかも現状の `bacon.toml` 配置では
  接続断も消えていない。詳細は [`cold-start.md` §②](./cold-start.md)。
- **`-Z threads=N` の N を増やす** — 15コア機でも N=12,16 は逆に悪化（cargo のクレート並列とオーバーサブスクライブ）。
  詳細は [`cold-start.md` §④](./cold-start.md)。
- **速いリンカへの置き換え** — lld が既に最速で詰めしろ無し。`wild` は Linux 専用。詳細は [`cold-start.md` §③](./cold-start.md)。
