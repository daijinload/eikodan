# FAST-RUST — Rust 開発を加速するためにやったこと（総まとめ）

eikodan の各サブプロジェクト（fastweb / connectweb / pg-bench / subsecond-demo）の試行を踏まえて
lastshot で採った高速化施策の一覧。実測根拠と「効かなかった理由」は各リンク先に。

## 1. 設計で稼ぐ（そもそもビルドしない）

1. **作業の7〜8割はビルドしない設計** — テンプレ / CSS / HTMX 属性の変更は保存で即反映（Rust ビルドゼロ）。
   詳細は [`fastweb/HOTRELOAD.md`](../fastweb/HOTRELOAD.md)。
2. **触った feature だけ再ビルド（package by feature）** — workspace 共通 `Cargo.toml` は触らない
   （`[workspace.dependencies]` やプロファイルを弄ると全クレート再ビルドが走る）。方針は [`CLAUDE.md`](./CLAUDE.md)。
3. **`sqlx::query!` 不使用 + codegen を `schema` クレートに隔離** — コンパイル時に DB 接続を要求せず、
   protoc は proto を触った時だけ走る。dev のビルド時依存を最小化。詳細は [`CLAUDE.md`](./CLAUDE.md) の「DB 作法」。
4. **Rust 変更時は約1秒が底（通常開発の増分ビルドの話）** — 「ファイル1個 touch して `cargo build -p app`」の
   増分ビルドが ~0.6〜0.74s + cold start 0.285s + ブラウザ再描画 ＝ 体感 ~1.2〜1.3s。フルビルドではない。
   参考: フルビルド（`cargo clean` 後）は **sccache warm（普段の状態）で ~6s** ／ **sccache cold（節目）で ~14〜15s**、
   no-op は **~0.1s**。warm/cold の分かれ目は「sccache のキャッシュキーが世代交代したか」（新規 clone / 別 toolchain
   で焼いた直後 / `cargo clean` 直後 等で cold になる）。実測内訳は [`COLD-START.md`](./COLD-START.md) と
   [`fastweb/BENCHMARK.md` ⑦](../fastweb/BENCHMARK.md)。
5. **cold start ~285ms の正体は macOS の起動時セキュリティ検証** — リンカが ad-hoc 署名した「中身の違う新バイナリ」を
   毎リビルド exec する度に `syspolicyd` / `AMFI` / `trustd` が検証する。`codesign -f -s -` で**署名を付け直すと
   初回検証が軽くなって ~100ms 短縮**（検証を完全に消すわけではなく軽くなる）。**唯一効いた起動側の施策**。
   詳細は [`COLD-START.md` §①](./COLD-START.md)。

## 2. ビルドツール側で稼ぐ（ビルドが走る時に速く）

1. **dev profile は全クレート opt-level=0（フルビルドを最小化）** — `[profile.dev]` 自前=0 / 
   `[profile.dev.package."*"]` 依存も=0 に統一。以前は依存だけ opt-level=3 で焼いていた（dev でも実行を
   速くしたい狙い／"効いてる本命"と書いていた）が、実測でコスト/便益が割に合わず撤回した:
   フルビルド +約11秒 / -45% のコストを `cargo clean` / 新規 checkout / deps 違いブランチ切替のたびに
   払う一方で、dev 動作の便益(+17% throughput / +0.2ms p50)は手元の操作頻度では体感ゼロ・**負荷試験は
   release（opt-3）／ 本番デプロイ相当を測るなら release-max（opt-3 + LTO + cgu=1, [`fastweb/BENCHMARK.md` ⑥](../fastweb/BENCHMARK.md)）でやるので不要**。dev profile の評価軸は反復速度のみ・動作速度は release / release-max が担保する、と
   profile の役割を明確に分けた。`codegen-backend` は自前も依存も `cranelift` に統一（追加検証で依存も
   cranelift に切り替えた・wall 差 ±0.2s = 誤差、fallback warning 無し）。実測根拠は [`fastweb/BENCHMARK.md`](../fastweb/BENCHMARK.md) ⑤。
2. **開発時は Rust nightly を使う**（`rust-toolchain.toml` で固定。本番は stable）。nightly 限定の高速化を **2つ** opt-in:

   | 機能                                       | 何をする                                                                 | 効くシナリオ                                                       |
   | ------------------------------------------ | ------------------------------------------------------------------------ | ------------------------------------------------------------------ |
   | **`codegen-backend = "cranelift"`**        | rustc の **コード生成段** を LLVM より高速化（自前も依存も cranelift に統一） | codegen が支配項になる構成（自前クレートが大きい構成。dev は全クレート opt-level=0 なのでフロント律速だが、それでも保険として残す） |
   | **`-Z threads=8`**（並列フロントエンド）    | 1クレート内の rustc 処理（型チェック・マクロ展開・codegen）をスレッド分割 | 1クレートが巨大化したフルビルド（**実証済み: 約2倍速くなる**）       |

   どちらも **lastshot 規模では実測差ほぼゼロ**（自前クレートが小さく、並列化・codegen 短縮の余地が無いため。
   **warm sccache で nightly フル構成 6.0s / stable + 素のツール 5.87s = ±2%**, cold sccache でも +17% 程度 ──
   裏取り実測は [`fastweb/BENCHMARK.md` ⑦](../fastweb/BENCHMARK.md)）。今は効かないが、**大きいプロジェクトに
   育てば効くはず**の **保険として残す**: `-Z threads` の方は「巨大1クレートでフルビルド約2倍」が
   `fastweb/BENCHMARK.md` ③ で実証済み、Cranelift の方は理屈上 codegen 支配な構成で効くはずだが lastshot では
   まだ盤面が来ていない。

   **本番ビルドは stable で**やる: `./run release` と Dockerfile が `RUSTUP_TOOLCHAIN=stable` +
   `assets/strip-nightly.sh` で nightly 専用行（`cargo-features` / `codegen-backend`）を剥がす。**dev=nightly / 本番=stable** が掟。

   詳細は [`COLD-START.md` §④ (`-Z threads`)](./COLD-START.md) / [§⑤ (Cranelift)](./COLD-START.md) + [`fastweb/BENCHMARK.md`](../fastweb/BENCHMARK.md) ②③。
3. **lld（macOS）／ mold（Linux）でリンク高速化** — `.cargo/config.toml` の target rustflags で配線。
   ただし「もっと速いリンカ」が更に効くわけではない（lld ≈ apple-ld）。詳細は [`COLD-START.md` §③](./COLD-START.md)。
4. **sccache で重い依存をキャッシュ** — 依存（axum / tokio / buffa…）をキャッシュから返して
   `cargo clean` / 新規 checkout / deps が変わるブランチ切替を短縮。**新構成（全 opt-0）で再計測済み**
   ([`fastweb/BENCHMARK.md`](../fastweb/BENCHMARK.md) ④ 末尾): 旨味は当時より小さく、30 クレート規模でフル再ビルド -13%、
   90 クレート規模では **incr ON が baseline より遅くなる**（+14%、依存=opt0 で軽くなった分を sccache 往復が食う）。
   **`CARGO_INCREMENTAL=0` まで切るとフル再ビルドが規模に関わらず -40〜52% で、CI / worktree 切替多用 / `cargo clean`
   多用 / 共有キャッシュ運用など「フル再ビルドが多い環境」では旨みが大きい**（代償は日常ループの税 check +0.1〜0.2s /
   build +0.2〜0.5s、規模で増える ── が、その環境ならフル再ビルドの節約で取り返せる）。lastshot 規模で増分ループ
   中心の場合は局所的に少し速くなる/遅くなるの誤差圏。

5. **incremental は既定 ON のまま** — 差分ビルドの本体は `incremental`（`CARGO_INCREMENTAL=0` にしない）。
   sccache とは別レイヤなので併用する。

## 3. 採らなかったもの（効かないと実測で確認）

- **ホットパッチ（subsecond / dioxus）** — 関数本体しか差し替えられず、構造変更（フィールド追加・シグネチャ変更・
  スキーマ変更）で結局フルビルドに戻る。AI 高速開発の「黙ってフォールバック」は事故のもと。
  詳細は [`COLD-START.md`](./COLD-START.md) ホットパッチ節。
- **systemfd で速さを稼ぐ** — 速さは変わらない、接続断を消すだけ。しかも現状の `bacon.toml` 配置では
  接続断も消えていない。詳細は [`COLD-START.md` §②](./COLD-START.md)。
- **`-Z threads=N` の N を増やす** — 15コア機でも N=12,16 は逆に悪化（cargo のクレート並列とオーバーサブスクライブ）。
  詳細は [`COLD-START.md` §④](./COLD-START.md)。
- **速いリンカへの置き換え** — lld が既に最速で詰めしろ無し。`wild` は Linux 専用。詳細は [`COLD-START.md` §③](./COLD-START.md)。
