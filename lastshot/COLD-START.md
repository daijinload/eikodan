# COLD-START — Rust変更の反映時間と cold start 高速化の検証

開発ループ層2（ハンドラ・service 層など **Rust の変更**）が画面に反映されるまでの時間を実測し、
「**1秒以内に反映できるか / cold start を縮められるか**」を調べた記録。
層1（テンプレ・CSS・HTMX属性）はビルドゼロで即反映なので対象外（[`README.md`](./README.md) 開発ループ参照）。

実測根拠重視（[`CLAUDE.md`](./CLAUDE.md) 「推測で埋めない」）。環境は macOS 26 (Darwin 25.5.0) / Apple Silicon
(aarch64-apple-darwin) / nightly + lld + sccache。

## 結論（先に）

- **約1秒の再起動ループが現実的な底**。Rust 変更はビルド + プロセス再起動が要り、ここは詰め切れない。
- 効いたのは **codesign だけ**（サーバ起動を約100ms短縮 → サーバ側の反映が1秒を切る）。
- **systemfd は速さを変えない**。再起動中の接続拒否を消すだけ。しかも**現状の `bacon.toml` 配置では効いていない**。
- **リンカ変更・ホットパッチは採らない**（前者は効果ゼロ、後者は適用範囲が狭く AI 開発と相性が悪い）。

## 反映時間の内訳（実測）

**前提**: ここで言う「ビルド」は**日常の開発ループ ＝ ファイル1個 touch して `cargo build -p app` する増分ビルド**の話。
フルビルド（`cargo clean` 後）は別物（下の参考表）。

| 区間                                        | 時間                       | 備考                                      |
| ------------------------------------------- | -------------------------- | ----------------------------------------- |
| 体感 端から端（保存 → ブラウザ再描画）      | ~1.2〜1.3s                 | ブラウザ側 livereload 再接続+再描画を含む |
| └ ビルド（touch → 再リンク）                | ~0.74s                     | rustc コンパイルが支配的                  |
| └ サーバ cold start（exec → `"listening"`） | ~0.285s                    | macOS 初回起動セキュリティ検証（下記）    |
| └ 残り                                      | ブラウザ側の再接続・再描画 |                                           |

### 参考: フルビルドの実測（3 構成 × sccache cold/warm）

`cargo clean` 後の `cargo build [--release] -p app` を 3 構成 × cold/warm で計測（lastshot, 2026-06-15）。
sccache 有効。"stable + 素のツール" = `RUSTUP_TOOLCHAIN=stable RUSTFLAGS=""` で nightly rustflag(`-Z threads`/lld) を
無効化 + `strip-nightly.sh` で `cargo-features` / `codegen-backend` 行を剥がす（Docker と同条件）:

| 構成                                                                   | cold (sccache miss)  | warm (sccache hit、普段の体感) |
| ---------------------------------------------------------------------- | -------------------- | ------------------------------ |
| **dev** (nightly + cranelift + `-Z threads=8` + lld + sccache + opt-0) | ~14.6s               | **~6.0s**                      |
| **dev** (stable + `RUSTFLAGS=""` + sccache + opt-0、素のツール)        | ~17.1s               | **~5.87s**                     |
| **release** (stable + `RUSTFLAGS=""` + sccache + opt-3)                | ~12.5s ※             | **~6.1s**                      |

※ release cold は同セッションで dev stable を先に焼いた直後の値（stable+opt-0 の proc-macro が sccache に
残っていた分やや甘い）。真に cold なら +1〜2s 上振れ見込みで dev cold と並ぶ。

読み方:
- **warm（普段の体感）はどれも約 6s で誤差圏** ── 重い依存の opt-3 LLVM 仕事を sccache が返すので、
  自前クレートの仕事しか残らない。**dev opt-0 と release opt-3 が並ぶのも、nightly フル構成と stable + 素のツールが
  並ぶのも、全部この理由**。`./run release` の体感が dev とほぼ変わらない＝節目で本番相当を気楽に確認できる。
- **cold は 13〜17s** ── 新規 clone / `cargo clean` 直後 / worktree 新規追加 / toolchain 切替直後 / 大きい
  deps 更新で当たる。cold 状態だけ nightly が `-Z threads=8` の並列フロントで +17% 速い。普段のループには
  乗らない差。
- **「`cargo clean` 後 = 常に 13〜15s」と読むのは間違い** ── sccache キャッシュキーが世代交代したかで意味が
  変わる。普段はほぼ warm に収まる。
- 詳細な分解（cranelift / -Z threads / lld 個別の効きや sccache warm 同士の差の理由）は
  [`fastweb/BENCHMARK.md` ⑦](../fastweb/BENCHMARK.md)。

### 参考: 増分・no-op の実測（フルビルドとの対比）

| シナリオ                                          | 時間  | 備考                                          |
| ------------------------------------------------- | ----- | --------------------------------------------- |
| **増分ビルド**（`app/main.rs` を1ファイル touch） | ~0.6s | 上の表の「ビルド 0.74s」と同区間（ばらつきあり） |
| **no-op**（変更なしで `cargo build`）             | ~0.1s | cargo のフィンガープリント検査だけ              |

**ポイント**: 日常の dev ループは増分ビルド側（~0.6〜0.7s）で回るので、約1秒の底もそちらの話。フルビルドが
warm 6s に収まる事実は「節目で動作確認するときに気楽に待てる」上乗せであって、増分・check（package by feature）の
hot loop の主役交代ではない（package by feature の効きは [`fastweb/BENCHMARK.md` ③⑦(e)](../fastweb/BENCHMARK.md)）。

## cold start の正体（なぜ exec→listening に 0.28s かかるか）

**macOS の初回起動セキュリティ検証**。リンカが ad-hoc 署名した「新しい中身」のバイナリを初めて exec する際、
`syspolicyd`(Gatekeeper) / `AMFI` / `trustd` 等が検証する。**約270ms前後・サイズ非依存・content-hash でキャッシュ**
（＝同じ中身の2回目は速い）。**毎リビルド＝新しい中身＝毎回フル検証**になる。

- 根拠: 同一バイナリの2回目起動は速く、中身を変えるたびに遅い、を実測で確認。
- `DYLD_PRINT_STATISTICS` は macOS 26 で出力抑止されログ取得不可のため、`main.rs` 側の時刻計測で確認した。
- これは `db::connect()` ではない（DB プール生成は ~10ms 程度）。cold start は pre-main のセキュリティ検証。

## 検証した施策一覧

### ① codesign `-f -s -`（build 後に署名し直す）→ ◎ 効く

`exec → "listening"`（cold start のセキュリティ検証が乗る区間）、各5試行の実測:

|               | 起動区間（5試行）                       | 定常値      |
| ------------- | --------------------------------------- | ----------- |
| codesign 無し | 0.493 / 0.284 / 0.286 / 0.286 / 0.285 s | **~0.285s** |
| codesign 有り | 0.357 / 0.182 / 0.181 / 0.184 / 0.182 s | **~0.182s** |

（各 trial1 は暖機の外れ値）。**約100ms短縮（起動区間がほぼ半減）**。

- 反映総時間（サーバ側 = ビルド + 起動）: **約1.03s → 約0.96s（サーバ側は1秒切り）**。
- ただし**体感 端から端（ブラウザ込み）は 1.2〜1.3s → 1.1〜1.2s で、まだ1秒超**。codesign で1秒を切るのはサーバ側まで。
- 仕組み: リンカが付ける ad-hoc 署名より `codesign -f -s -` で付け直した署名のほうが初回検証が軽い。
  codesign 自体のコストは ~40ms なので差し引きプラス。

### ② systemfd（ソケット引き継ぎ）→ △ 配置次第。速さは不変、接続断を消すだけ

再起動の最中にクライアントが叩き続けて「接続拒否(REFUSED)」を数えた比較（2.5秒ポーリング）:

| 構成                                                                                             | REFUSED              |
| ------------------------------------------------------------------------------------------------ | -------------------- |
| ただ kill → 再起動                                                                               | 42                   |
| **現状の `bacon.toml` `[jobs.serve]`**（systemfd を bacon ジョブの中に置き `kill_then_restart`） | **42（＝効果ゼロ）** |
| systemfd を最外で常駐させ内側のアプリだけ差し替え                                                | **0**                |

- 理由: `kill_then_restart` は**ジョブのプロセスツリー丸ごと（systemfd 含む）を殺す** → ソケットも一緒に閉じる。
- ⚠️ **現状の `bacon.toml` / 旧 `README` の「ソケットを引き継いで再起動（接続が切れない）」は、この配置では成立しない。**
  効かせるには systemfd を bacon の**外側**に常駐させ、内側で app だけを再起動する必要がある（例: `systemfd --no-pid -s http::3000 -- bacon -j run`）。
- 価値は速さではなく「**再起動中にブラウザへ接続拒否が点滅しない（livereload がコケない）**」こと。

### ③ 速いリンカ / ビルド側 → ✗ レバーにならない

インクリメンタル再リンク時間: **lld ~0.70s ≈ apple-ld ~0.69s**、ld-classic ~0.80s（むしろ遅い）、`wild` は Linux 専用。
ビルド時間は rustc のコンパイルが支配的で、**lld は既に最速。変える余地なし**。

### ④ 並列フロントエンド `-Z threads=N` の最適値 → ✗ レバーにならない（8で十分／むしろ大は悪化）

**問い**: 15コア機（Apple Silicon M4 Max 相当、perf 5 + eff 10）なら `threads=8` より大きくしたら速くなるのか。
**答え**: ならない。incremental は完全フラット、full rebuild は **12,16 で逆に悪化**する。

`crates/app/src/main.rs` を毎回マーカー追記して `cargo build -p app` を 3 試行ずつ、`RUSTFLAGS` で `threads=N` を振った実測:

| threads | incremental avg (3試行)   | settle (新 rustflags でフル再ビルド・参考) |
| ------: | ------------------------: | -----------------------------------------: |
|       1 | **0.471s** (0.467〜0.473) | 27.7s                                      |
|       2 | 0.479s (0.477〜0.482)     | 28.3s                                      |
|       4 | 0.484s (0.480〜0.489)     | 29.0s                                      |
|       8 | 0.483s (0.481〜0.485)     | （warm-up と同 rustflags のためキャッシュ） |
|      12 | 0.494s (0.493〜0.497)     | 37.8s ← 悪化                               |
|      16 | 0.489s (0.486〜0.493)     | **53.1s ← 大幅悪化**                       |

- **incremental は 0.47〜0.49s でほぼフラット**（差 23ms はノイズレベル、むしろ N 大で微増）。
- 理由: lastshot の各クレートは小さい（`app` 159行、`webcore` 204行、他は 50〜70行）。
  rustc 内の並列フロントエンドは **「1クレートを何スレッドで型チェック/codegenするか」** で効くが、
  対象が小さいと並列化する仕事自体が無い。incremental は触ったクレート 1 つしか rustc が走らないので、ここが効かない。
- full rebuild は **cargo のクレート並列 (`-j15`) × `-Z threads=N` のオーバーサブスクライブ**で N=12,16 が悪化。
  15コアに対し「クレート 15並列 × フロントエンド 16」だと 240 スレッド要求、コンテキストスイッチで自滅。

**結論**: `threads=8` は妥当（というか「外しても変わらない」が実測上の正解）。15コアあっても上げる意味はなく、
むしろ上げると full rebuild が遅くなる。`.cargo/config.toml` の `-Z threads=8` は現状維持。

#### なぜ「効かないのに残す」のか — 伸び代の保険

今は効かないが、**1クレートが育ったときに勝手に効き始める保険**として置いておく価値はある:

- rustc の並列フロントエンドは「1クレート内の型チェック / borrow check / MIR / codegen をスレッド分割」する仕組み。
  対象クレートが**数千行クラス**になってきた所から効き始める（今の lastshot は最大でも `webcore` 204行で全く足りない）。
- lastshot は **package by feature** でクレートを細かく割る設計なので暴発しにくいが、feature が機能を抱え込んで
  500 → 2000 行と育つことは普通にある。その時 `threads=8` は何もしなくても勝手に効き始める。
- **N=8 はオーバーサブスクライブの安全圏でもある**: 15コアで `cargo -j15` と掛け合わせても、依存グラフの幅
  ボトルネックにより同時に走る rustc は通常数個。瞬間最大 `8 × 数個 = 数十スレッド` で 15コアで捌けるレンジ。
  N=16 にすると `16 × 数個 = 50+ スレッド` でコンテキストスイッチが効いて自滅する（上の N=16 で 53s に悪化したのがこれ）。

つまり **「今は効かないが害もなく、伸び代を捨てない」設定**。実測値が変わったら（クレートが育ったら）見直す。

#### 再現方法

`RUSTFLAGS="-C link-arg=-fuse-ld=<lld> -Z threads=N" cargo build -p app` で
`crates/app/src/main.rs` にユニークなマーカーを足して毎回 build を強制し、`time` で計測。
N を変えると rustflags が変わって sccache miss するので、settle として 1 回フル再ビルドしてから incremental を 3 試行。

### ⑤ Cranelift codegen-backend（`codegen-backend = "cranelift"`）→ ✗ 現状効かない（残しても害は無い）

nightly 限定機能。**dev プロファイル全体**を Cranelift でコード生成する（自前も依存も）。
Cargo.toml の `[profile.dev]` で配線:

```toml
[profile.dev]
opt-level = 0
codegen-backend = "cranelift"        # 自前クレート

[profile.dev.package."*"]
opt-level = 0
codegen-backend = "cranelift"        # 依存も Cranelift（旧: 依存だけ llvm 強制 → 実測で差ゼロのため撤回）
```

実測（[`fastweb/BENCHMARK.md`](../fastweb/BENCHMARK.md) ②③、小クレート構成 / 巨大1クレート構成の両方）:
**cranelift on/off で差はノイズ（±5% 以内）**。`{cranelift on/off} × {threads on/off}` の 2×2 でフル/増分とも測ったが、
有意差は `-Z threads` の方だけから来る。

#### なぜ効かないのか

Cranelift は **rustc パイプラインの最終段（コード生成: LLVM IR → マシンコード）だけを置き換える**。
今の dev ループはここがボトルネックじゃない:

1. **opt-level=0 では LLVM が既に "fast path"**（最適化パスを全部スキップ）。LLVM at opt-0 と Cranelift の差は元々小さい。
   Cranelift の本領は「opt-2/3 を opt-0 並みに速く」だが、dev でそんな構成にはしない。
2. **支配項はフロント（型チェック・borrow check・マクロ展開）**。codegen の絶対量が小さいので、そこを速くしても全体は動かない。
   `-Z threads` がフロントを並列化するのは効くが、Cranelift はフロントに触らない。
3. **lastshot 固有**: 自前クレートが 50〜200 行と小さく、codegen の絶対量自体が微小。依存は ⑥ で opt-level=0 に
   揃え、`codegen-backend` も `cranelift` に統一した（[`fastweb/BENCHMARK.md`](../fastweb/BENCHMARK.md) ⑤ 末尾の
   検証：依存 cranelift vs llvm で wall は ±0.2s = 誤差、fallback warning 無し）。`opt=0` では LLVM が fast path
   なので backend を揃えても揃えなくても差は出ない、を実測で再確認した上で「カーブアウトを置かない」方を選んでいる。

#### なぜ残すのか

- **害が無い**（±5% のノイズ範囲、増えも減りもしない）。
- **「codegen がボトルネックの世界」に変わった瞬間に勝手に効き始める保険**（自前クレートが数千行に育つ、opt-level を上げたデバッグをやる、など）。
- **本番から確実に剥がす仕組みがある**: `./run release` と Dockerfile が `assets/strip-nightly.sh` で
  Cargo.toml の `cargo-features` / `codegen-backend` 行を一時的に削除して stable で通す。
  → 「効かない nightly 機能が本番に漏れる」リスクは閉じている。

`-Z threads=8` と全く同じ位置づけ — **「今は効かないが、伸び代の保険」**。

### ⑥ dev profile を全クレート opt-level=0 に統一 → 反復速度に全振り（旧: 依存=3 の非対称を撤回）

dev ループの評価軸は**反復速度**だけ・本番動作速度は `[profile.release]`（opt-3）と必要なら `release-max`（opt-3 + LTO + cgu=1）が担保する、と profile の役割を明確に分ける。実測根拠は [`fastweb/BENCHMARK.md` ⑥](../fastweb/BENCHMARK.md)。

**配線**: `Cargo.toml` の `[profile.dev]` を自前も依存も opt-level=0 にする:

```toml
[profile.dev]
opt-level = 0                          # 自前 = 毎回再コンパイル → コンパイル時間最短
codegen-backend = "cranelift"

[profile.dev.package."*"]
opt-level = 0                          # 依存 = フルビルドのコストを最小化
codegen-backend = "cranelift"          # 依存も Cranelift で統一（実測で差なし。詳細は §⑤）
```

#### 経緯: 「dev でも実行を速く」狙いの opt 非対称（自前=0 / 依存=3）を試して撤回した

当初は **dev ループで実行時間も稼ぐ** 設計を入れていた:

|              | 編集ごと            | コンパイル時間の重さ | 実行時の影響           |
| ------------ | ------------------- | -------------------- | ---------------------- |
| 自前コード   | **毎回再コンパイル** | 毎回払う             | 規模小さく支配的でない |
| 依存クレート | 一度ビルドして固定  | 初回1回のみ          | dev ループ中ずっと響く |

→ 自前=opt=0 / 依存=opt=3 で「毎回払う側は軽く、一度きりの側を重く」というコスト構造を狙う。理屈は綺麗だが
実測してコスト/便益を確かめた:

| 設定 | フルビルド (wall) | rustc 合計 CPU | dev binary GET / req/sec | p50 |
|---|---|---|---|---|
| 自前=0 / 依存=3（旧） | 24.02s | 249.7s | 47,874 | 1.03ms |
| **自前=0 / 依存=0（採用）** | **13.18s** | 59.6s | 39,685 | 1.25ms |
| 差分 | **-10.8s (-45%)** | -76% | -17% | +0.22ms |

(lastshot, dev profile, `cargo clean` + `RUSTC_WRAPPER=` で sccache 無効化, oha -c 50 -z 15s。
詳細は [`fastweb/BENCHMARK.md` ⑤](../fastweb/BENCHMARK.md))

**撤回した理由**:
- 「初回のみのコスト」の前提が**半分しか正しくない**。`target/` が残っている間は確かに無料だが、
  `cargo clean` / 新規 checkout / deps が変わるブランチ切替の節目で **+約11秒 (+45%) を毎回払い直す**。
- 動作便益(+17% throughput / +0.2ms p50)は手元の操作頻度では**体感ゼロ**。そもそも **dev で負荷試験はやらない**
  ── 本番速度を見たければ `cargo build --release`（opt-3）か、本番デプロイ相当を測るなら `cargo build --profile release-max`（opt-3 + LTO + cgu=1）で別 profile を焼くのが筋。dev profile で
  動作速度を取りに行こうとしていたこと自体が **profile の役割の混同**だった。

#### 他の §との関係（撤回後）

- **§④ `-Z threads`**: 「自前 opt=0 で codegen は軽い、フロントが支配項」の前提は変わらず（自前は元々 opt=0）。
  むしろ依存も opt=0 になって codegen 比率がさらに下がるので、`-Z threads` の効きは小規模構成では引き続きほぼゼロ。
- **§⑤ Cranelift**: 「opt=0 では LLVM が fast path」は引き続き有効。`[profile.dev.package."*"]` の codegen-backend は
  追加検証で `cranelift` に統一（wall ±0.2s = 誤差・fallback warning 無し、自前と揃える＝カーブアウトを置かない方を選んだ）。
  依存が opt=0 になったぶん codegen の絶対量はさらに小さい。
- **sccache**: 「重い依存をキャッシュから返す」効果は依然あるが、依存ビルド自体が opt=0 で元より軽いので
  **当時の数値ほど劇的ではない**（[`fastweb/BENCHMARK.md` ④](../fastweb/BENCHMARK.md) 末尾「新構成（全 opt-0）での再計測」）。
  実測: 30 クレートのフル再ビルドで `incr ON` -13% / `incr OFF` -52%、90 クレートでは `incr ON` が +14% で **悪化**（依存節約を
  sccache 往復が食い潰す）/ `incr OFF` -39%。日常ループは `incr ON` が baseline と誤差、`incr OFF` は税
  （check +0.1〜0.2s、build +0.2〜0.5s）。**`incr OFF` はフル再ビルドが多い環境（CI / worktree 切替多用 / `cargo clean`
  多用 / 共有キャッシュ）で旨みが大きい**（規模に関わらず -40〜52% で、税はその環境ならフル再ビルドの節約で取り返せる）。
  lastshot 規模で増分ループ中心の場合は局所的・最大の旨味は CI/共有キャッシュ側。

## ホットパッチ（subsecond / dioxus）を採らない理由

プロセス再起動を無くせば cold start も消えるが、

- **関数本体の差し替えにしか効かない**。構造変更（フィールド追加・シグネチャ変更・依存追加・スキーマ変更）は
  フルビルド+再起動に逆戻り。
- 本プロジェクトは設計上、**変更の7〜8割がテンプレ/CSS/HTMX = ビルドゼロ**で、残る Rust 変更も
  スキーマファースト故に**構造変更に寄りがち** → hot-patch が効くのは「service 層の関数本体だけいじる」薄いスライスのみ。
- **AI 高速開発と相性が悪い**: 当てられない時に黙ってフォールバック/部分更新する曖昧さは事故のもと。
  clean に再起動して fail-fast にするほうが安全（lazy DB 接続を避けるのと同じ判断）。
- [`README.md`](./README.md) の除外方針（subsecond は axum 素組に非対応＝Dioxus 移行が要る）とも整合。

## 推奨アクション

- **約1秒の再起動ループを底として受け入れる。**
- 取るなら **codesign のみ**（〜100ms、副作用は署名し直すだけ・任意）。dev ループに入れるなら
  `cargo build -p app && codesign -f -s - target/debug/app && ./target/debug/app` の順。
- **systemfd** は「再起動中のエラー点滅を消したい」時だけ、配置を bacon の**外側**に直して使う。
- **リンカ変更・hot-patch はやらない。**

## 再現方法

- 接続断カウント: 127.0.0.1:3000 に N 秒間 接続し続け OK/REFUSED を数える小スクリプトで、
  再起動サイクルの最中の REFUSED を計測。
- codesign A/B: ソースに一意マーカーを足して**毎回 content を変え**（cold start を確実に発生させる）、
  `touch → build → [codesign] → exec → "listening"` を計測、各5試行。測定後はソースを復元（リポジトリ無改変）。
