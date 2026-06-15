# ビルド速度 実測メモ

計測環境: Apple Silicon / nightly (rustc 1.98.0-nightly) / 2026-06-13。
機能クレートをダミー生成（1個=約600行: serde構造体20＋impl＋関数30＋axumルート）して、
それを全部 `.merge()` する使い捨て bin で計測。生成物はコミットしていない（使い捨て）。

> **2026-06-15 追記**: ①〜③ は当時の構成「依存=opt3 / 自前=opt0」（opt 非対称）で計測した。
> その後 ⑤ で実測の上、**opt 非対称を撤回**し dev profile は全クレート opt-level=0 に統一した。
> ①〜③ の数値は引き続き有効（リンカ・並列フロントの評価は opt-level に依存しない）。
> ④ sccache は **新構成（全 opt-0）で再計測済み**（2026-06-15 / ④ 末尾「新構成での再計測」表）。
> 結論の方向は当時と同じ「`incr OFF` ＝ 大勝ち / `incr ON` ＝ 規模次第」だが、**旨味は当時より小さく、90 クレート級では
> incr ON が baseline より遅くなり得る** ことが新たに判明（依存が opt-0 で軽い + sccache の per-call 往復が乗るため）。

## ① 規模が増えたときのスケール

| 操作 | 1機能(現状) | 30クレート | 90クレート |
|---|---|---|---|
| no-op（無変更build） | ~0.02s | 0.05s | 0.06s |
| **1クレート編集→`check`**（baconの内側ループ） | <0.16s | **0.16s** | **0.18s** |
| 1クレート編集→`build`（起動時・リンク込み） | ~0.5s | 0.52s | **0.9–1.0s** |
| 自分のコードを全再コンパイル（cold） | — | 2.5s | 7.1s |
| 初回フルビルド（依存込み・並列） | — | 6.7s | 6.5s |

- **日常の型チェック(`check`)は規模に依らずほぼ横ばい（0.16→0.18s）**。1クレート編集なら再チェックは「そのクレート＋合流点」だけ、残りはキャッシュ。
- **唯一伸びるのはリンク**＝`build`/起動時のみ。バイナリ総量に比例し、90クレート級(自前約5.4万行)で約1秒に到達。`check`は超えない。
- 「全再コンパイル」は target全消し or 共有クレート(webcore)編集のときだけ。日常では踏まない。

## ② nightly有り vs 外し（cranelift+`-Zthreads` の効果）

lldリンカ と opt非対称(dep=opt3 / own=opt0) は stable でも使えるので両方で維持し、差分だけ比較。

**30クレート**
| | 全再コンパイル | 1編集→build | 1編集→check |
|---|---|---|---|
| nightly有り (cranelift + `-Zthreads`) | 2.62s | 0.54s | 0.38s |
| nightly外し (llvm / threads無し) | 2.56s | 0.54s | 0.34s |

**90クレート**
| | 全再コンパイル | 1編集→build | 1編集→check |
|---|---|---|---|
| nightly有り | 7.48s | 0.93s | 0.97s |
| nightly外し | 7.46s | 0.89s | 0.89s |

> ②のcheck絶対値が①より高いのは計測手順差（cache温度）。on/off比較用に見ること。

- **全項目で差は±5%以内、むしろ外した方がわずかに速い**。この構成では nightly のビルド速度上乗せは実質ゼロ。
- 理由: `-Zthreads`は「クレート内」並列化だが、cargoが既に「クレート間」を並列化しコアを飽和 → 小クレート多数では効かない（逆に少数の巨大クレートで効く）。Craneliftはコード生成を速くするが、opt0の小クレートではリンク(lld・両者同じ)が支配的で差が出ない。
- **速さの実体は stable で使える層**: lldリンカ / opt非対称 / 実行時テンプレ(ノービルドUI) / 葉クレートの局所性。

## ③ 逆の構成: 巨大1クレートを単独開発したら（`-Zthreads`が効く世界）

1クレート＝約28,000行（serde構造体1500＋関数2000＋ハンドラ60）を生成し、そのクレートだけを編集する想定で計測。

| | coldフルビルド | 1関数編集→build | →check |
|---|---|---|---|
| nightly有り (cranelift + `-Zthreads`) | **11.7s** | 6.2s | 4.9s |
| nightly外し (llvm / threads無し) | **23.3s** | 6.5s | 4.0s |

**coldフルビルドだけ約2倍速くなる。** 1クレートしか無いとcargoのクレート間並列が使えず全コアが遊ぶ → 並列フロント(`-Zthreads`)がその遊休コアに型チェック/マクロ展開を分散させて半減する。切り分けると、この高速化は **`-Zthreads`が100%・craneliftは無関係**:

| 巨大クレート coldフルビルド | 時間 |
|---|---|
| cranelift + threads | 11.6s |
| cranelift のみ（threads無し） | 23.1s |
| llvm + threads | 11.6s |
| llvm のみ（=素stable） | 23.3s |

`threads`有=約11.6s / 無=約23.2s で、backendは無関係。opt-level 0では支配項がコード生成ではなく**フロント（型チェック・マクロ展開）**なので、コード生成を速くするcraneliftは効かず、フロントを並列化する`-Zthreads`だけが効く。

一方で **増分(1関数編集)は約6sで nightly有無ほぼ同じ**。増分は再実行するフロントが少なく並列化の余地が無いため。そして約6sは「クレートが巨大だから」のコスト ── 同じ作業を**小さい葉クレートに割れば増分は約0.9s（①②参照）で、巨大1クレート＋nightlyの約7倍速い**。

→ **日々の編集ループを速くする正解は「nightlyを足す」ではなく「クレートを割る」。** `-Zthreads`が巨大クレートのフルビルドを救うのは事実だが、増分は救わない。package-by-feature は最初からこの状況を避けている。

## ④ sccache を足すと効くのか（コンパイルキャッシュ）

sccache 0.15.0 を `brew install` して、①②と同じダミー生成ワークスペース(30/90クレート)で計測。
sccache は `RUSTC_WRAPPER` で挟むのでビルドフラグ(lld / cranelift / `-Z threads`)はそのまま維持し、sccache の有無だけを比較する。

**先に効く仕組みを2点。** ① sccacheはrustc1呼び出しごとに「ソース＋フラグ＋依存rmeta＋コンパイラ」のハッシュを鍵にキャッシュし、鍵一致なら再コンパイルせず成果物を返す。
② cargoは**incrementalを自分のworkspaceクレートにしか使わない**（registryの依存=axum/tokioには元々使わない）。
そして**フル再ビルドで重いのはopt-level 3でビルドされる依存の方**。この2つが効き方とトレードオフを決める。

> 当初メモにあった「`CARGO_INCREMENTAL=0` が必須」は誤り。incrementalを残したまま依存だけキャッシュできる（下表「incr ON」列）。
> `CARGO_INCREMENTAL=0` は「自分のクレートもキャッシュしたい」ときの**任意**の追い込みで、その代償にincrementalを失う。

### 日常ループ（差分ビルド / min of 3）

| 操作 | baseline(sccache無) | **sccache + incr ON**（推奨） | sccache + incr OFF | incr=0 ペナルティ |
|---|---|---|---|---|
| no-op build (30) | 0.05s | 0.05s | 0.05s | ~0 |
| 1編集→check (30) | 0.14s | **0.16s** | 0.24s | **+0.10s** |
| 1編集→build (30) | 0.54s | **0.58s** | 0.60s | **+0.06s** |
| 1編集→check (90) | 0.17s | **0.18s** | 0.27s | **+0.10s** |
| 1編集→build (90) | 0.87s | **0.89s** | 0.93s | **+0.06s** |

- **incrementalを残せば日常ループはbaselineとほぼ同じ（誤差・+0.01〜0.02s）。** 編集したクレートは内容が変わる＝必ずミスなのでsccacheの出番は無いが、incrementalが生きているので速い。
- `CARGO_INCREMENTAL=0` の税は **check +0.10s ／ build +0.06s**。内訳は incremental喪失(+0.04s)＋sccacheラッパ往復(+0.06s)。
- **このペナルティは規模で伸びない**（30も90も同じ +0.10/+0.06s）。差分ビルドが触るのは編集クレート＋合流点(app)だけでクレート総数に依らないため。

### フル再ビルド（cargo clean → build、依存込み・定常warm）── sccache が効く場所

| 規模 | sccache無 | **sccache + incr ON** | sccache + incr OFF |
|---|---|---|---|
| 30クレート | 8.2–8.4s | **5.7–6.0s**（約-28%） | **4.4s**（約-47%） |
| 90クレート | 11.9–12.7s | **9.8s**（約-20%） | **5.1–5.5s**（約-57%） |

- **効く実体は opt-level 3 / LLVM の依存クレート(axum・tokio・hyper…)がキャッシュから返ること。** 依存は incr の有無に関わらず100%ヒットした（cacheableな依存は全部ヒット・ミス0）。
- **incr ON と OFF の差＝自分のクレートをキャッシュするか。** incr ONだとclean時に自crートは作り直し(opt0で安い)、incr OFFだとそれもヒット。クレート数が増えるほどOFFの取り分が伸びる（90個で 9.8s→5.3s）。
- 注意点3つ:
  1. **app(bin)は永遠にキャッシュされない**（`CannotCache(crate-type, bin)`。sccacheはライブラリしか効かない）。最後のリンクは毎回走る。
  2. **ウォームアップが要る。** populate直後の初回clean buildはまだフルコスト〜それ以上(初回 ~10s/~14s)。rmetaのキー揺れで2回目から安定ヒット。
  3. cranelift も `-Z threads` も**犯人ではない**（{cranelift on/off}×{threads on/off} の2×2で全てヒット・同等を確認）。フル構成のままキャッシュは効く。

### 新構成（全 opt-0）での再計測 (2026-06-15)

⑤ で dev profile を全クレート opt-0 に揃えた後、上の数値（依存=opt3 前提）が崩れているはずなので
同じハーネスで再計測した。条件: nightly (rustc 1.98.0-nightly, 2026-06-12) / sccache 0.15.0 / lld+`-Z threads=8` /
30,90 ダミークレート（serde 構造体 20＋impl＋関数 30＋axum ルートを 1 クレート 800 行で生成、`.merge()` する bin）/
各 min of 3 試行・edit-check の trial1 は warmup 外れ値（直前の build から check への切替コストが乗る）。

#### 日常ループ（差分ビルド・新構成）

| 操作 | baseline | sccache+incr ON | sccache+incr OFF | incr=0 ペナルティ |
|---|---|---|---|---|
| no-op (30) | 0.08s | 0.05s | 0.05s | ~0 |
| 1編集→check (30) | 0.19s | **0.16s** | 0.29s | +0.10s |
| 1編集→build (30) | 0.82s | **0.79s** | 1.00s | +0.18s |
| no-op (90) | 0.09s | 0.06s | 0.07s | ~0 |
| 1編集→check (90) | 0.20s | **0.18s** | 0.43s | +0.23s |
| 1編集→build (90) | 1.44s | **1.34s** | 1.91s | +0.47s |

- **日常ループの形は当時と同じ**: incr ON は baseline と誤差レベル、incr OFF は税（check +0.1〜0.2s / build +0.2〜0.5s）を払う。
  ペナルティは規模が増えるほど大きくなる（90 クレートで build +0.47s）── 編集クレート＋合流点(app) に加え、
  ⑥ で依存も opt-0 で軽いぶん「自前の incremental 喪失」が相対的に支配的になるため。

#### フル再ビルド（cargo clean → build、新構成）── ここが当時と変わった

| 規模 | baseline | sccache+incr ON | sccache+incr OFF |
|---|---|---|---|
| 30クレート | 6.17s | **5.34s** (約-13%) | **2.93s** (約-52%) |
| 90クレート | 12.15s | **13.87s**（約+14% ← **悪化**） | **7.43s** (約-39%) |

参考: 同条件・旧構成（依存=opt3）の数値は上の表 ── 30: 8.2 → 5.7 / 4.4、90: 11.9 → 9.8 / 5.1。

- **30 クレートは ON でも勝つが旨味は小さい**（-28% → -13%）。当時の主役だった「opt-3/LLVM の重い依存をキャッシュから返す」効果が、依存=opt-0 では元々軽いので相対的に縮む。
- **90 クレートは incr ON が baseline より遅くなる**（+14%）。sccache の `--show-stats` を見ると workspace クレートは `Non-cacheable reasons: incremental` で **600+ 件キャッシュ対象外**。incr ON では「軽くなった依存だけがヒット」する一方、自前 90 クレート分の sccache 往復オーバーヘッド（1 rustc 呼び出しあたり数十 ms）が依存節約を食い潰すと負ける。
- **incr OFF は規模に関わらず勝つ**（30: -52%、90: -39%）。自前クレートも cacheable になり、当時の「クレート数が増えるほど OFF の取り分が伸びる」傾向は今も維持。ただし当時より絶対値は近づいた（90: 5.3 → 7.43s）。
- 旧 注意点3つは引き続き有効: ① `app(bin)` は永遠にキャッシュ対象外 ② warm-up が要る（**実測でも初回 clean-build は ws-90 で 16.5s と外れ値**） ③ cranelift / `-Z threads` はキャッシュ可否に無関係。

### 判断

- sccache の効きどころは新構成でも同じ（**`cargo clean` / 新規checkout / 別worktree / deps が変わるブランチ切替** での「依存ごとフル再ビルド」）。**日常の編集ループは依存を踏まないので、効くのはこの局面だけ。**
- **`RUSTC_WRAPPER=sccache` + incrementalを残す（incr ON）** は **小〜中規模なら依然「片務的に得」**（30 クレートで約-13%）。だが **90 クレート級では逆に baseline より遅くなる**ので「規模次第」が新しい答え。手元のプロジェクトで一度測ってから決めるべき設定で、過去のような全肯定ではない。
- **`CARGO_INCREMENTAL=0` はフル再ビルドが頻繁な環境では旨みが大きい**。規模に関わらず -40〜52% で、絶対値で見ても 30 クレートで -3.2s、90 クレートで -4.7s と効きはむしろ規模が大きいほど増える。**該当する環境**: CI（毎回クリーンビルド）、worktree を頻繁に切り替える / `cargo clean` を多用する開発、deps が変わるブランチを行き来する人、共有キャッシュをマシンをまたいで使うチーム。代償は日常ループの税（check +0.1〜0.2s / build +0.2〜0.5s、規模が増えるほど増える）で、増分ビルド中心の常時 local では過剰だが、上記の環境ならフル再ビルドの節約が税を一瞬で取り返す。
- 最大の旨味は **CI/共有キャッシュ**（マシンをまたいで依存を使い回す）── 新構成でも変わらない。そこは `CARGO_INCREMENTAL=0` 込みで隔離して使う。
- **lastshot 規模 (自前 50〜200 行 × 数クレート) では sccache を入れる積極的な理由は薄い**。`.cargo/config.toml` の `rustc-wrapper = "sccache"` は残しているが、これは「CI/共有キャッシュへの伸び代を捨てない」「`cargo clean` 多用時の保険」のため。たまの clean-build を速くしたい人だけが恩恵を受ける。

## ⑤ 依存も opt-level=0 にするか（=「opt 非対称」を捨てるか）

①〜④ では「自前=opt0 / 依存=opt3」の非対称を維持していた（"dev でも実行を速くしたい"狙い）。
だが当初コメントの「初回のみのコスト」は半分しか正しくない ── `cargo clean` / 新規checkout /
deps が変わるブランチ切替の節目で何度も払い直す。実測してこの非対称が割に合うか確かめた。

計測 (2026-06-15): lastshot, dev profile, `cargo clean` 後の cold ビルド, `RUSTC_WRAPPER=` で
sccache をバイパスして純粋な opt-level コストを見る。ランタイムは `oha -c 50 -z 15s` で GET `/`
（DB1往復 + minijinja 描画 + JSON 埋め込み）を叩く。

| 設定 | フルビルド (wall) | rustc 合計 CPU | GET / req/sec | p50 | p99 |
|---|---|---|---|---|---|
| 自前=0 / 依存=3（従来） | 24.02s | 249.7s | 47,874 | 1.03ms | 1.28ms |
| **自前=0 / 依存=0（採用）** | **13.18s** | 59.6s | 39,685 | 1.25ms | 1.51ms |
| 差分 | **-10.8s (-45%)** | **-76%** | -17% | +0.22ms | +0.23ms |

- **コスト**: 依存=opt3 はフルビルドで wall +約11秒 / rustc CPU 時間 +約190秒（4倍以上の LLVM 仕事を
  焼いている）。`target/` が残っている間は無料だが、worktree 切替や deps 違いブランチに飛ぶたびに払う。
- **便益**: dev binary で GET / が 17% 速く・p50 が 0.22ms 縮む。だが **dev で負荷試験はやらない**
  ── 本番速度は release（`[profile.release]` の opt-3）／本番デプロイ相当を測るなら release-max（opt-3 + LTO + cgu=1、⑥参照）で取る。手元で人がポチポチする頻度では
  17% も 0.2ms も体感ゼロなので、dev では意思決定に使えない数字。
- **結論**: opt 非対称は撤回し `[profile.dev.package."*"] opt-level = 0` に統一。dev profile の
  存在意義は「速い反復」で、ms 単位の動作速度より秒単位のビルド速度を取る方が profile の役割と整合する。
  動作速度は release で取る（dev で取りに行こうとしていたこと自体が profile の役割の混同）。

> 計測の限界: 単発計測なのでノイズ±1〜2秒は含む。ランタイムは GET の DB 読みだけで、別ワークロード
> （POST `/increment`・connectrpc 経路・テンプレ重描画）では比率が変わり得る。それでも結論は変わらない
> （= dev 動作速度を評価軸にしないので、別ワークロードで便益が増減しても意思決定に影響しない）。
> sccache 有効時の「2回目以降のフルビルド」は両構成とも秒オーダーまで縮むので、本数値はあくまで
> 「キャッシュ無しの初回」の話。④の sccache 評価は別途。

### 依存も cranelift にできるか（dev=opt0 で「依存は llvm 強制」のカーブアウトを外せるか）

⑤ で依存も opt-level=0 に揃えた後、当初の「依存は LLVM 強制」（cranelift は強い最適化が苦手なので opt-3
依存だけは LLVM で焼く）の理由が崩れたので、依存も cranelift にできるか測った。

計測 (2026-06-15): lastshot, dev profile, `cargo clean` 後の cold ビルド, `RUSTC_WRAPPER=` で sccache をバイパス。
各構成3試行（trial1 は FS キャッシュが冷たい外れ値、trial2/3 は warm）:

| 設定 | trial1 (cold FS) | trial2 (warm) | trial3 (warm) | rustc CPU |
|---|---|---|---|---|
| 自前=cranelift / 依存=llvm（旧） | 12.75s | 8.66s | 8.84s | 60〜66s |
| **自前=cranelift / 依存=cranelift（採用）** | 13.07s | **8.67s** | **8.67s** | 54〜61s |
| 差分（warm） | — | ±0s | ±0.2s | -約10% |

- **wall は完全に誤差範囲**（warm trial で ±0.2s 以内・cold trial も同程度）。dev=opt-0 では LLVM が fast path
  なので cranelift と LLVM の codegen 時間差は元々小さい、を再確認した（COLD-START.md §⑤ の理屈通り）。
- **rustc CPU は約-10%**（依存の codegen が cranelift で軽くなる分）。ただし cargo がクレート並列で既にコアを
  飽和させているため、CPU 減は wall に乗らない（並列度の天井に当たって直列化される）。
- **fallback warning 無し**: 依存に未対応 intrinsic で LLVM へ落ちるクレートは（lastshot の依存グラフでは）出なかった。
- **採用**: 「自前=cranelift / 依存=llvm」の非対称を撤回し `[profile.dev.package."*"] codegen-backend = "cranelift"` に
  統一。理由は単純さ（自前と依存の backend が揃う・カーブアウトの説明が要らない）。wall は同等で害が無く、伸び代として
  「codegen がボトルネックになる構成（自前が育つ・依存が増える）」で勝手に効き始める保険も維持できる。

> 限界: lastshot 単体での実測。依存グラフが大きく変わるプロジェクト（巨大な C 連携クレート群など）では
> fallback で warning が出る可能性がある。出たクレートだけ `[profile.dev.package.<name>] codegen-backend = "llvm"` で
> 個別に戻せばよい（package "*" のままで `[profile.dev.package.crate-name]` を後勝ちで上書きできる）。

## ⑥ release プロファイルの最適化レベル（本番デプロイで LTO を盛るか）

⑤ で「dev は反復速度、動作速度は release が担保する」と役割分担した。残る論点は **release 自体の最適化レベル**:
`cargo build --release` の素は **Rust 既定 = opt-3 のみ・LTO 無し・codegen-units=16**。これ以上盛るなら明示的に
`[profile.release]` を上書きする（LTO=fat / cgu=1 / strip / panic=abort 等）。本番デプロイで「最大最適化」を
入れる価値があるかを実測した。

### 設計判断: baseline と最大最適化を**切り替え可能**な別 profile で持つ

カスタムプロファイル `[profile.release-max]`（`inherits = "release"`）を定義し、Cargo 標準の機能で 2 つを共存させる。
成果物は `target/release/` と `target/release-max/` に分かれて並ぶので、両者の binary を同条件で比較できる。
`./run release` / `./run release-max` の両方を用意（lastshot のみ実装）。

```toml
# lastshot/Cargo.toml
[profile.release-max]
inherits = "release"
lto = "fat"          # クレート跨ぎインライン化
codegen-units = 1    # ユニット跨ぎ最適化
# strip = "symbols" は入れない: panic backtrace の関数名が消える(0xアドレスのみになる)→ 本番事後解析が困る
# panic = "abort"    は入れない: axum ハンドラ panic を tokio が catch して 500 を返す挙動が消える
#                                → プロセス丸ごと死ぬ。Web サーバには不適。
```

### 計測 (lastshot, 2026-06-15)

- 環境: stable + RUSTFLAGS=""（Docker と同条件）, `cargo build -p app`, `cargo clean` 後の cold ビルド
- ランタイム: `oha -c 50 -z 15s` で GET `/`（DB1往復 + minijinja 描画 + JSON 埋め込み）, baseline 2試行 / release-max 3試行

| profile | フルビルド (wall) | バイナリサイズ | req/sec (avg) | p50 | p95 | p99 | p99.9 |
|---|---|---|---|---|---|---|---|
| **release** (現状=既定 opt-3 のみ) | **13.0s** | **7.6MB** | **53,956** | 0.914ms | 1.057ms | 1.147ms | 1.279ms |
| **release-max** (opt-3 + lto=fat + cgu=1) | **41.9s** | **5.0MB** | **55,058** | 0.898ms | 1.029ms | 1.107ms | 1.236ms |
| 差分 | **+28.9s (約3.2倍)** | **-2.6MB (-34%)** | **+1,102 (+2.0%)** | -0.02ms | -0.03ms | **-3.5%** | -3.4% |

### 読み方

- **スループット +2.0%**: 100 台必要なら 97〜98 台で済む計算。インスタンス費用が常時かかる本番では確実に黒字、
  build 時間 +30s/回 を本番デプロイで取り返す。**lastshot のように I/O 律速（DB 1往復が大半）のワークロードでも
  この程度は出る** ── tokio・axum・hyper の hot path がクレート跨ぎでインライン化される効果。
- **p99 改善のほうが p50 より大きい (-3.5% vs -0.02ms)**: LTO + cgu=1 の主効果はテール改善。
  「たまに長い処理」が無くなることで avg req/sec が稼げている。GraphQL / connect-rpc みたいに同期処理の長い経路が
  増えるとここがもっと効きやすい。
- **バイナリ -34%（7.6MB → 5.0MB）**: strip 無しでも縮む。LTO の dead code elimination が効いていて、
  クレート跨ぎで使われない関数がリンク段階で除去される。Docker image / pull 時間に直で効く副次効果。
- **build +28.9s (約3.2倍)**: ローカルで `./run release-max` を回す頻度は低いので体感はほぼゼロ。
  CI / Docker build に乗るのが実体で、デプロイ 1 回ごとに +30s ── 本番が回す頻度なら許容範囲。

### 入れていない 2 つの理由（あえて）

- **`strip = "symbols"`**: シンボル表が消えると panic backtrace が関数名でなく **生アドレス（0x10a3f4e）**になる。
  `debug = false` で DWARF（ファイル:行）は元から無いが、シンボルを残せば関数名は出る → 本番の事後解析（panic ログから
  どの関数で死んだかを特定する）を捨てる価値は無いので保持。バイナリ縮小は LTO で十分（4.3MB ← 5.0MB の追加削減
  600KB だけのために panic 解析を捨てるのは割に合わない）。
- **`panic = "abort"`**: tokio が axum ハンドラの panic を catch して 500 を返す挙動が消える ──
  プロセス丸ごと死ぬので **in-flight の他リクエストも全部巻き添え**。バイナリは小さくなる（unwind テーブル削除）が、
  Web サーバの可用性とトレードする価値は薄い。CLI ツールや組み込み向けの設定。

### opt-level の上限

参考: Rust の `opt-level` は **0/1/2/3/"s"/"z" の 6 値のみで `3` が最大**。`9` 等は `optimization level needs to be
between 0-3, s or z` で cargo がパース時点で拒否する。gcc の `-O9` も内部で `-O3` にクランプされる旧来の誤解で、Rust は
そもそも受け付けない。つまり Cargo の素の手段で取れる最大最適化は **「opt-3 + LTO=fat + codegen-units=1」**
（＝ release-max）。これ以上を狙うなら PGO（profile-guided optimization）/ BOLT といった本番プロファイル前提の
別レイヤに上る話で、本 PR のスコープ外。

### 結論

- **`release-max` を採用**。本番デプロイは `cargo build --profile release-max -p app` を使う（`./run release-max` で
  ローカル起動・本番と同じ最適化レベルで動作確認可）。
- **既定 `release` も削除しない**。「最大最適化を入れたら何かおかしくなった」のときに比較できる比較対象として残す。
  `cargo build --release` と `cargo build --profile release-max` の成果物は `target/release/` と `target/release-max/`
  に並ぶので、同条件で並列に比較できる。
- **Dockerfile / CI を release-max に切り替えるかは別 PR**。まずプロファイル定義と `./run` タスクを整備した段階で、
  Docker / CI 反映の是非はデプロイ運用側の判断。

> 限界: lastshot 単体での実測。ワークロードが CPU 律速（重い JSON シリアライズ・テンプレ・暗号など）に寄ると
> release-max の旨味は +5〜15% まで伸び得る。I/O 律速（DB 多用・connect-rpc 通信が大半）に寄るとさらに 1% 以下になる
> 可能性もある。意思決定としては「コスト build +30s/回」が許容できれば常時 ON、許容できなければ既定 release のまま、
> で十分（規模が大きい本番ほど ON が有利）。

## ⑦ フルビルド速度の正体（sccache warm/cold × dev/release × nightly/stable）

①〜③ で dev profile を全クレート opt-0 に統一し、⑥ で release-max を整えた段階で、改めて
**「現状のフルビルドは結局いくら？」**を分解して測った。意思決定の場面は 3 つの軸で変わる:

1. **sccache の状態**: warm（普段のループ。前回ビルドのキャッシュが残ってる）vs cold
   （新規 clone / worktree 切替 / sccache パージ / 別 toolchain でのビルドが先に走った直後）
2. **dev profile (opt-0) vs release profile (opt-3)**
3. **nightly フル構成 (cranelift + -Z threads + lld) vs stable + 素のツール**

### 計測 (lastshot, 2026-06-15)

- ハーネス: `cargo clean -q && /usr/bin/time -p cargo build [--release] -p app`、各構成 **5 試行**
  （trial 1 = cold sccache、trial 2-5 = warm sccache、**warm は 4 試行の median**）
- sccache 有効（`.cargo/config.toml` の `rustc-wrapper = "sccache"`）、`.cargo/config.toml` の他設定は構成で切替
- "stable + 素のツール" = `RUSTUP_TOOLCHAIN=stable RUSTFLAGS=""` で nightly rustflag(`-Z threads`/`lld`) を無効化 +
  `strip-nightly.sh` で `cargo-features` / `codegen-backend` 行を剥がす（Docker と同条件）
- 順序は A (nightly+full) → C (release stable) → B (dev stable) で実行（B cold の解釈は ※ 参照）

| 構成 | trial 1 (cold) | warm trials (2-5) | warm median |
|---|---|---|---|
| **A: dev (nightly + cranelift + -Z threads=8 + lld + opt-0)** | 12.46s | 6.56 / 6.59 / 6.72 / 7.45※外 | **~6.66s** |
| **B: dev (stable + RUSTFLAGS="" + opt-0)** | 12.78s ※1 | 6.44 / 6.59 / 6.59 / 6.62 | **~6.59s** |
| **C: release (stable + RUSTFLAGS="" + opt-3)** | 13.43s | 6.70 / 6.84 / 6.84 / 6.85 | **~6.84s** |

※外 A trial 5 = 7.45s は背景ノイズによる外れ値（median 算出時に上位 1 個を捨てた効果と等価。trial 2-4 のみで
median = 6.59s）。warm の構成間 spread は ±3% で、差は計測ノイズと同水準（lastshot 規模ではビルドツールの
最適化が支配項にならないという ⑤ ②③ の結論を再確認）。

※1 B cold は順序効果に強く依存する。今回は先に C (stable+LLVM+opt-3) を焼いたので、stable LLVM の
proc-macro エントリが sccache に残っており B cold は 12.78s に圧縮された。**「真に cold」（前段が nightly
構成のみで stable+LLVM の proc-macro エントリが空）の B cold は別セッションで 17.14s** と計測済み（(d) の
「cold で nightly +17%」はこの 17.14s 比較に基づく）。同様に、今回の C cold (13.43s) は前段が nightly のみ
なので「真に cold」に近い。

### 読み方

**(a) warm が普段の体感、cold は節目で当たる数字**
- 普段の dev ループは sccache warm 状態で回っているので、**フルビルドの体感は約 6.6〜6.8s**。
- 12〜17s を払うのは「sccache のキャッシュキーが変わった節目」: 新規 clone / `cargo clean` 直後 /
  `worktree` を新規追加した直後 / toolchain を nightly↔stable で切り替えた直後 / 大きい deps の更新。
- 「`cargo clean` 後 = 常に 13s」と書いてある旧表現は不正確（sccache が冷えてるかどうかで意味が変わる）。
  以前 FAST-RUST.md §1.4 / COLD-START.md に "~13s (sccache が重い依存を返すため)" と書いていたのは
  実は cold sccache の数字で、warm sccache での説明文と数字がチグハグだった（本 § で修正）。

**(b) warm なら dev opt-0 と release opt-3 がほぼ同じ (~6.6s vs ~6.84s)**
- 重い依存の opt-3 LLVM コード生成は sccache が返すので、実 LLVM を回るのは自前クレートだけ。
  自前が小さければ opt-3 / opt-0 の差が 0.2s レベルに縮む（+3% ＝ 計測ノイズ圏内。
  **lastshot 規模では opt レベルは支配項ではない**）。
- これが「`./run release` 体感が dev とほぼ変わらない」の正体。本番デプロイ相当を手元で確認するコストが軽い
  ので、release 検証を節目に挟みやすい（release-max のコスト 42s/回とは桁が違う）。

**(c) warm なら nightly フル構成 vs stable+素のツールが誤差圏 (~6.66s vs ~6.59s)**
- cranelift / -Z threads=8 / lld のチューニング合計効果が ±1% で**実測差ほぼゼロ**（むしろ stable のほうが
  わずかに速い計測結果、＝ノイズ圏）。lastshot 規模では保険として残しているだけ、という FAST-RUST.md §2.2 の
  主張が裏取りされた形（同 § が引用する旧記述もこの実測を根拠にする）。
- 効き始める閾値: **-Z threads** は「1 クレートが巨大化したフルビルド」で約 2 倍（③ 参照）、
  **cranelift** は codegen 律速の自前構成（lastshot 規模ではまだ来ていない）、**lld** は apple-ld と
  そもそも差がない（[`COLD-START.md` §③](../lastshot/COLD-START.md)）。今日の構成では一つも効いていない。
- **規模を増やさない設計（package by feature の葉クレート分割）を守る限り、stable + 素のツールに降りても
  ビルド速度は落ちない**。nightly を残す理由は今日も「将来の保険」だけ。

**(d) cold 状態では nightly がわずかに優位 (12.46s vs 17.14s)**
- 「真に cold」（stable+LLVM proc-macro エントリが sccache に無い状態）の B cold = 17.14s に対し、
  A (nightly+full) cold = 12.46s で +37%（自前 90 クレート相当を実 rustc で焼く局面では `-Z threads=8` が
  コア余りを並列で食って差が出る）。新規 clone / worktree 追加直後 / 別 toolchain で初めて焼く瞬間など
  「最初の 1 ビルド」だけ nightly がわずかに気持ちいい。普段のループ（warm）には乗らない差。
- 同セッションで連続計測すると、B が先に C の stable+LLVM proc-macro エントリを共有して B cold = 12.78s
  まで縮む（今回の計測値）。**「cold ≒ 13s で揃って見える」のは順序効果が打ち消した結果**で、構成本来の
  cold 性能を比較したいなら sccache を意図的に空にするか、トコルチェーンを切り替えた直後の単発で測る。

**(e) "warm 6s で並ぶ" は feature 分割が要らないという話ではない**

(b) で「dev opt-0 と release opt-3 が warm で並ぶ」を見ると「じゃあ feature 分割の build-speed 動機も
弱いのでは？」と読まれかねないので、明示的に分解しておく。**6s が並ぶのはフルビルド warm の場面だけで、
日々の hot loop ＝ 増分ビルド と `check -p` では feature 分割が依然 1 桁効いている**。3 場面で並べると:

| 場面 | feature 分割（現状） | モノリス 1 クレート（仮） | 効き |
|---|---|---|---|
| 増分 (touch 1 feature → `cargo build -p app`) | **~0.6〜0.74s** | **~6s**（③ 実測） | **約 10 倍** |
| 型チェック (`cargo check -p feature`) | **~0.1〜0.3s** | 全機能ぶんを check（推定 1〜3s, ① の n=90 で 0.18s と比較しても 5〜15 倍） | **約 5〜30 倍** |
| 単体テスト (`cargo nextest run -p feature`) | その feature のテストだけ走る | 全テスト、もしくはフィルタしてもビルド時間が乗る | **規模で大きく開く** |
| フルビルド warm（`cargo build -p app`） | **~6s** | 推定 ~6〜8s（`-Z threads` で並列フロントが回る場合） | 並ぶ／差が見えない |

含意:
- **増分・check・test の 3 場面では feature 分割が 1 桁〜2 桁効いている**。これが bacon の hot loop と
  「保存して型エラーをサブ秒で見る」体験の本体で、③ の「日々の編集ループを速くする正解は『nightly を
  足す』ではなく『クレートを割る』」と整合する。
- **フルビルド warm だけ並ぶのは設計どおり**。`app` は全 feature を merge してルーターを組むので
  app の依存グラフ = 全 feature を含み、`cargo build -p app` が実質ワークスペース全体ビルドになる場面。
  ここで feature 分割の効きが見えないのは「app を起動するときは結局全部要る」という構造の素直な帰結。
- **両立してる**: feature 分割は ①〜③ の場面（増分 / check / test）でサブ秒〜0.3 秒の hot loop を作り、
  フルビルド warm 6s は「節目で動作確認するときに気楽に待てる」上乗せ。**主役（feature 分割）が交代した
  わけではなく、節目の心理的コストが消えた**、という関係。
- 逆方向の含意: もし feature を 1 つに肥大化（モノリス化）させたら、フルビルド warm はぎりぎり並ぶが
  ①〜③ の hot loop が全部 1 桁遅くなる。**warm 6s を理由に feature 分割を緩めると損する側**。

### 結論（今日の lastshot で「現状のフルビルドはいくら？」と聞かれたら）

- **warm sccache（普段の体感）= 約 6.6〜6.8s** ── dev / release / nightly / stable のどれでも ±3% で誤差圏。
- **cold sccache（節目）= 12〜17s** ── 構成と前段で焼いたものに依存。「真に cold」の幅は dev nightly+full ~12.5s /
  dev stable+plain ~17s / release stable ~13s 前後。
- **約 6.7s と 12〜17s のどちらに当たるかは「sccache キャッシュキーの世代交代があったか」で決まる**、
  と覚えておけば日々の体感とドキュメント上の数字がズレない。
- ① の「フルビルド -45% (24s → 13s)」は **sccache を意図的に切って測った数字**（RUSTC_WRAPPER= 無効化）。
  sccache 有効の warm では旧構成も新構成も差は -10〜20% に縮む（重い deps を sccache が返すと opt-3/opt-0
  の差自体が支配項でなくなるため）。** ① の改善が無意味になったわけではない**: sccache 無効・新規 clone・
  CI で焼く時間が短くなる効果は別途残る。

## 結論

- **大多数のプロジェクト規模では、編集ループは1秒未満**。1秒を超えるのは「起動して動作確認する瞬間(build)」だけで、それも自前5万行(機能90個)級から。
- **この構成は今すぐ nightly を外してもビルド速度はほぼ落ちない**（安定版入りを待つ必要なし）。stableに倒すなら `Cargo.toml` の `cargo-features=["codegen-backend"]`＋`codegen-backend` 行と、`.cargo/config.toml` の `-Z threads=8` を外す。
- ただし1クレートを肥大化(モノリス化)させると `-Zthreads` がフルビルドを約2倍速くする（craneliftは無関係・③参照）。ただし増分ループは救わないので本筋は分割。**nightlyは「葉クレートの規律を破ったときの、フルビルド用の保険」**。
- **sccacheは現状入れていない（fastweb 側 ＝ lastshot 側は `.cargo/config.toml` に `rustc-wrapper = "sccache"` 入り）**。入れるなら `RUSTC_WRAPPER=sccache` だけ・incrementalは残す（④）が、**新構成（全 opt-0）では旨味は当時より小さい**: 30 クレートでフル再ビルド -13%、90 クレートでは **incr ON が baseline よりむしろ遅くなる**（+14%・依存節約を sccache 往復が食う）。**`CARGO_INCREMENTAL=0` まで切ればフル再ビルドは規模に関わらず -40〜52% 速くなり、CI / worktree 切替多用 / `cargo clean` 多用 / 共有キャッシュなど「フル再ビルドが多い環境」では旨みが大きい**（代償の日常ループ税は、その環境では取り返せる）。手元の増分中心ループでは規模次第なので一度測ってから決める。
