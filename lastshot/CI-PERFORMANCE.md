# CI の高速化 ── 調査・実測・採否の記録

lastshot の CI（`.github/workflows/lastshot-ci.yml`）について、「どこを速くできるか」を
**推測せず ARM 実機ベンチで実測して**取捨選択した記録。要約は README「CI / コンテナ ＞
CI の高速化方針」にもあるが、本書は**測り方・実数・なぜその判断にしたか**まで残す（`COLD-START.md` と同じ体裁）。

> 方針の一言: **「少数の太いキャッシュ」だけ採る。** 細かいキャッシュは restore/保存コストで
> 相殺〜逆効果になる。足す前に必ず実機で正味（restore + 保存 Post を引いた値）を測る。

---

## 結論サマリ（採否一覧）

| 対象 | 措置 | 実測 | 判断 |
|---|---|---|---|
| runner アーキ | `ubuntu-latest`(x86) → **`ubuntu-24.04-arm`** | build 92s→79s(~14%速)・総時間はほぼ互角 | **採用**（狙いは速度でなくローカルとのアーキ一致。public repo で無料） |
| semgrep 導入 | `pipx` → **`uv tool install`** | 導入 ~16s → ~2.5s | **採用** |
| `apt-get update` | **省略**（失敗時のみ update→retry） | ~12-17s 削減 | **採用** |
| postgres image | `postgres:17` → **`17-alpine`** | コンテナ初期化 ~14s → ~10s | **採用** |
| cargo+target | **`Swatinem/rust-cache`** | build ~76s → ~15s | **採用（本命）** |
| cargo-nextest | `taiki-e/install-action@nextest` | 導入済みキャッシュ ~0s | **採用（既定で入っていた）** |
| Playwright Chromium + npm | **`actions/cache`**（`~/.cache/ms-playwright` + `browser/node_modules`） | browser-setup 21s→6s + restore 5s = 正味 ~10s。保存 Post はキー命中時 0s | **採用**（当初却下→再計測で変更。下記） |
| semgrep の pip cache | 入れない | cold 17s → warm 16s（~1s）。重さは venv 展開で wheel DL でない | **却下**（uv で解決済み） |
| pipx venv cache | 入れない | warm でも当たらない | **却下** |
| rustup toolchain cache | 入れない | 復元 ~4s 得 ＜ **保存 Post 48s**。巨大で rust-cache の 10GB 枠を圧迫 | **却下（逆効果）** |
| apt パッケージ cache | 入れない | payload 小で restore 相殺 | **却下** |

---

## 計測方法（共通）

- **使い捨てブランチ** に matrix ベンチ用ワークフローを置き、`runs-on: ubuntu-24.04-arm` で実測。終わったら削除。
- **`max-parallel: 1` で順次実行**し、`Swatinem/rust-cache` を全ジョブで `shared-key` 共有して
  **build 区間を温める**＝重い build を一定化して「測りたい区間」のノイズを排除。
- **各条件3回**＋先頭に warmup（捨て）を1回。中央値で比較。
- ステップ単体を見たいときは **計測対象ステップを分離**（例: `browser-setup` と `browser` を別ステップに割る）。
- run の所要は GitHub API の `steps[].started_at/completed_at` 差分（秒）で取得。

> なぜここまでやるか: GitHub Actions の総時間は run ごとのばらつきが大きく（apt のロック待ち、
> ネットワーク変動など）、1回計測で「速くなった/ならない」を語ると誤判定する。**中央値**と
> **ステップ分解**で「決定論的に効く区間」と「ノイズ区間」を分けるのが要点。

---

## 各候補の調査ログ

### 1. runner: x86 → ARM
- **背景**: ローカルは Apple Silicon(aarch64)。CI も合わせればアーキ差由来の取りこぼしが減る。ARM runner は public repo なら無料。
- **実測**（コールド・各3回 matrix）: `Build app(release)` 中央値 x86 **92s** / ARM **79s** ＝ **計算は ARM ~14% 速い**。
  だがジョブ総時間は x86 **184s** / ARM **191s** ＝ **ほぼ互角**（むしろ僅差で x86）。
- **理由**: 総時間の大半は apt/コンテナ初期化/DL/test など**アーキ非依存**。さらに本番は rust-cache で
  build が既に短いので、ARM が勝つ build 区間自体が小さい。
- **判断**: **採用**。速度目的ではなく**ローカル・arm64 Docker とのアーキ一致**を総時間ノーコストで得るため。

### 2. semgrep: pipx → uv
- **背景**: install ステップが setup 区間で一番重く、semgrep 導入が支配的だった。
- **実測**: `pipx install semgrep==1.166.0` は ~16s（cold 17s → warm 16s ＝ **pip cache を足してもほぼ縮まない**）。
  正体は **wheel DL でなく venv 展開**。`uv tool install`（Rust製・並列・hardlink）は素で **~2.5s**。
- **判断**: **uv 採用・pip/pipx cache は却下**。uv はキャッシュ無しで速いのでキャッシュ自体不要。
- **補足**: semgrep の scan は `--config assets/semgrep` の**ローカルルール**なので、実行時のレジストリ DL も無い
  （`--config=auto`/`p/...` のような遠隔ルール取得はしていない）。

### 3. apt-get update の省略
- **実測**: `apt-get update` は ~12-17s。ランナー同梱 index で直接 `install` すれば ~8s で済む。
- **対策**: update を省き、**index 不整合で install が落ちたときだけ** `update → retry` で自己回復。ARM 実機で fast-path を確認。
- **判断**: **採用**（稀な失敗時のみ従来コストを払うだけ）。

### 4. postgres: 17 → 17-alpine
- **実測**: `Initialize containers` が `postgres:17` で ~14s、`17-alpine`(musl) で ~10s（pull が軽い）。
- **用途**は素の SQL のみなので alpine で機能差なし。**採用**。

### 5. rust-cache（本命）/ nextest
- `Swatinem/rust-cache` で cargo deps + target をキャッシュ ＝ build **~76s → ~15s**。**最大の効き**。これだけは太い。
- `taiki-e/install-action@nextest` は導入済みキャッシュを内蔵（~0s）。既に入っていたので追加作業なし。

### 6. rustup toolchain cache（却下＝逆効果の実例）
- **仮説**: nightly toolchain 導入 ~7s を `~/.rustup` キャッシュで消せる？
- **実測**: 復元は ~4s 得するが、**保存（Post ステップ）が 48s** かかった。さらに toolchain は巨大で、
  リポジトリの **10GB キャッシュ枠**を食い、本命の rust-cache を LRU で押し出すリスク。
- **判断**: **却下**。「キャッシュ＝速い」ではなく**正味（保存込み）で測る**ことの教訓。

### 7. 「旧構成 vs 新構成」を ARM 同条件で突き合わせ（総時間の正直な話）
- 上記 2〜4 をまとめて、**旧（pipx + apt-update + pg:17）vs 新（uv + apt-skip + alpine）**をARMで各3回:

  | 構成 | 3回 | 中央値 |
  |---|---|---|
  | 旧 | 112 / 126 / 126 | **126s** |
  | 新 | 89 / 120 / 131 | **120s** |

- ステップ分解すると、**触った区間は決定論的に速い**（install 39s→16s、container 14→10s ＝ 計 ~25s）。
  だが**総時間の中央値は ~6s しか縮まない**。
- 原因は **Browser E2E が 15〜41s と激しくブレて差を飲み込む**こと（下記 8 で正体を特定）。
- 教訓: 個々の run（例 1m34s）は**当たり回**で、中央値ではない。速さは中央値＋ステップ分解で語る。

### 8. Browser E2E（Playwright）キャッシュ（当初却下 → 再計測で採用）
- **当初**（ARM 化 PR の引き継ぎメモ）: 「payload 小で restore 相殺」と**却下**していた。
- **再計測**: `browser-setup`（`npm install` + `playwright install chromium`）を独立ステップに分離し、
  キャッシュ有/無 各3回:

  | | setup(DL) | restore | 正味 browser準備 |
  |---|---|---|---|
  | キャッシュ無 | 21s（20–23, 安定） | — | **~21s** |
  | キャッシュ有 | 6s | 5s | **~11s** |

- **正味 ~10s 短縮**。保存 Post は**キー命中時 0s**（playwright 更新時だけ 1 回 ~4s）。payload は
  Chromium **~120MB** と小さく rust-cache の 10GB 枠を圧迫しない。
- **本命の収穫**: 7 で総時間を暴れさせていた「Browser E2E 15〜41s」の正体は **Chromium DL の
  ネットワーク変動**だと特定（テスト実行 `npm test` 自体は cache 有無に関わらず**安定 ~4s**）。
  キャッシュで 30〜40s に跳ねる外れ回が消える ＝ **CI 総時間のブレも縮む**。
- **判断**: **採用**。キー `playwright-<arch>-hash(package-lock.json)` は playwright 更新で自動失効。
- 設定: `.github/workflows/lastshot-ci.yml` の Browser E2E 直前に `actions/cache@v4`
  （`~/.cache/ms-playwright` + `lastshot/browser/node_modules`）。

---

## 原則（このプロジェクトの CI チューニング作法）

1. **推測せず実機で測る。** GitHub Actions は run ごとのばらつきが大きい。中央値＋ステップ分解で判断する。
2. **キャッシュは正味で評価する。** 復元の得だけでなく**保存（Post）と 10GB 枠の圧迫**まで引いて黒字か見る。
3. **「少数の太いキャッシュ」だけ。** 細かいキャッシュは相殺〜逆効果。採用は rust-cache（本命）と
   Playwright（DL変動の除去が主目的）の 2 つに絞る。
4. **判断は更新されうる。** Playwright は一度却下したが再計測で採用に転じた。古い結論を鵜呑みにしない。

## 再現のしかた（また測りたくなったら）
1. main 起点で使い捨てブランチを切る（例 `bench/xxx`）。
2. matrix（`max-parallel: 1`・warmup + 各条件3回）の使い捨てワークフローを `.github/workflows/` に置き、
   `runs-on: ubuntu-24.04-arm`／rust-cache を `shared-key` 共有で温める。測りたいステップは分離する。
3. push → `gh run view <id> --json jobs` と GitHub API の `steps[].started_at/completed_at` で秒を集計、中央値で比較。
4. 終わったら**ブランチとワークフローを削除**（計測 run の記録は GitHub Actions 側に残る）。
