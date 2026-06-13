# in-memory Postgres ベンチマーク (Mac / Apple Silicon)

「最速 Postgres、かつ SSD ではなくメモリに書く」を Mac 上で実現する方式を、
**アプリ(Node)から接続して同一ワークロードを叩いた時の実効速度**で横並び比較した。

- 機種: Apple Silicon (arm64) / macOS 26.5.1
- Node: v25 / クライアント: `pg` (サーバ型), `@electric-sql/pglite`, `pg-mem`
- 接続: **単一コネクション・単一スレッド**(PGlite/pg-mem が単一接続なので公平のため揃えた)
- 計測: 各操作を数千回、ウォームアップ後に hrtime で計測。ops/s と p50/p99 レイテンシ
- チューニング(サーバ型に共通適用):
  `fsync=off synchronous_commit=off full_page_writes=off shared_buffers=512MB max_wal_size=4GB wal_level=minimal`

## 結果

| metric | native RAM socket | native RAM TCP | apple-container | docker tmpfs tuned | docker ssd tuned | docker default(ssd) | PGlite | pg-mem |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| init (ms) | 3.7 | 4.2 | 6.4 | 9.7 | 4.6 | 7.2 | **389** | 2.7 |
| schema (ms) | 4.4 | 3.8 | 8.2 | 4.9 | 5.0 | 9.0 | 3.5 | 14.6 |
| bulk seed (rows/s) | **348,716** | 341,880 | 184,740 | 259,023 | 235,775 | 199,203 | 182,171 | 9,786 |
| single INSERT (ops/s) | **33,672** | 22,634 | 9,207 | 4,393 | 4,389 | 3,148 | 7,908 | 3,942 |
| point SELECT (ops/s) | **34,233** | 25,374 | 9,037 | 4,337 | 4,397 | 4,307 | 7,648 | 3,066 |
| indexed SELECT (ops/s) | **34,136** | 24,826 | 9,388 | 4,554 | 4,745 | 4,558 | 6,881 | 5,211 |
| UPDATE (ops/s) | **37,013** | 25,436 | 9,784 | 4,468 | 4,372 | 3,391 | 6,742 | 5,567 |
| JOIN+agg (ops/s) | 613 | **619** | 577 | 519 | 517 | 516 | 95 | 21 |

(太字は各行の最速)

> ⚠️ この表は **単一接続で1クエリずつ叩いたレイテンシ(1往復の速さ)**であって、並列スループットではない。
> スループットは別途 pgbench で測り直した(下記)。結論(native が最速)は変わらず、並列ではむしろ差が拡大する。

## 設定の再点検(チューニング・環境の検証)

「Docker が遅いのは設定ミスでは?」を潰すための確認:

- **チューニングは全サーバで同一に効いている。** `pg_settings` で確認: tmpfs / ssd / apple / native の4つは
  `fsync=off, synchronous_commit=off, full_page_writes=off, shared_buffers=512MB(65536×8kB), wal_level=minimal, max_wal_size=4GB`
  が全て **`source=command line`**。default だけ stock(`fsync=on` 等)。→ tmpfs↔native の差は設定ではない。
- **tmpfs は本当にメモリ。** `/var/lib/postgresql/data` は `tmpfs size=2097152k`(2GB RAM)で mount 確認済み。
- **エンジン素の性能は全部ほぼ同じ。** ホスト網を除外して VM 内で pgbench を回すと docker/apple も native と同オーダー(下「スループット」表の〔参考〕行)。
  中身は同じ Postgres 17 で、native socket が速いのは「VMを挟まない+unixソケット」だけが理由。エンジン差ではない。

## スループット(並列・pgbench で測り直し)

`pgbench`(C製クライアント、scale=10)で「アプリ=Macホストから接続」した時の実スループット:

| 経路 | c1 SELECT (tps) | c8 SELECT (tps) | c8 書き込みTPC-B (tps) |
|---|---:|---:|---:|
| **native RAM / unix socket** | **53,168** | **218,701** | **31,782** |
| native RAM / TCP loopback | 38,479 | 139,310 | — |
| Apple container(ホストから) | 11,640 | 39,814 | — |
| docker tmpfs(ホストから) | 4,389 | 18,241 | — |
| 〔参考〕docker tmpfs(VM内) | 26,561 | 109,995 | 15,316 |
| 〔参考〕Apple container(VM内) | 28,400 | 222,547 | — |

要点:
- **ホストから繋ぐ限り native socket が圧勝**(c8 218k tps = Docker の約12倍 / Apple の約5.5倍)。差の正体はすべてホスト→VM の往復で、VM 内に入れれば全部同等(〔参考〕行)。
- **コンテナで行くなら Apple 一択**(Docker の約2倍)。
- **PGlite / pg-mem は単一スレッドで並列に伸びない**(単一接続値が上限)。native PG はプール+並列で 20万 tps 級まで伸び、負荷時ほど差が開く。

## RAMディスク vs SSD を分離して再検証(durability 2x2)

最初は docker-tmpfs(4,393)vs docker-ssd(4,389)で「差≒0」としたが、これは **両方 `fsync=off`** で測っており、
SSD 側も同期書き込みしていない(VM のページキャッシュに溜まる)ので **変数を分離できていなかった**。
そこで native PG で「RAMディスク / 実SSD」×「`fsync=on` / `off`」の 2x2 を測り直した(`durability_bench.sh`、pgbench -N=書き込み律速):

| 構成 | c1 (1接続=レイテンシ) tps | c8 (8接続=スループット) tps |
|---|---:|---:|
| SSD durable (`fsync=on`) | 13,271 | 36,862 |
| **RAM durable (`fsync=on`)** | **14,219** | **37,527** |
| SSD throwaway (`fsync=off`) | 12,979 | 44,856 |
| RAM throwaway (`fsync=off`) | 13,155 | 44,570 |

(全条件 `wal_sync_method=open_datasync`)

- **RAM vs SSD は durable でも誤差(c1 +7% / c8 +2%)。** VM 非介在・fsync 両モードで裏取りした、信頼できる結果。
- **`fsync=on` と `fsync=off` が c1 でほぼ同速(13,271 vs 12,979)。** c1=13,000 tps は 1コミット ≈75µs。
  本物の物理フラッシュ(NVMe の `F_FULLFSYNC`)なら 0.5〜1ms=1,000〜2,000 tps に落ちるはずが、落ちていない。
- **理由:macOS の `open_datasync`/`fsync` は `F_FULLFSYNC` を発行しない。** つまり Mac では Postgres を `fsync=on` にしても
  書き込みは OS/ドライブのキャッシュ(RAM)に渡るだけで物理 NAND への強制フラッシュをしない。SSD パスでも同期点で物理ディスクを待っていない
  → **RAM ディスクが消せる待ちがそもそも無い**ので上乗せが誤差になる。(裏を返すと macOS の Postgres はデフォルトで電源断に対して非クラッシュセーフ。)
- **速度目的では RAM ディスクは不要。ただし**物理 SSD への書き込み自体は(遅延書き戻しで)発生するので、
  **SSD の書き込み寿命(摩耗)を避けたい / ディスク痕跡を残したくない用途では RAM ディスクは有効**(かつ僅かに速い)。

## 結論(プレーンな一文)

**Mac で最速かつ高互換なのは「ネイティブ Postgres を unix ソケットで繋ぐ」構成。
速度を生むのは①VM を挟まないこと ②`fsync=off`(c8 書き込みスループット +20%)で、RAM ディスクの速度上乗せは誤差(durable でも +2〜7%)。
ただし RAM ディスクには SSD の書き込み摩耗を肩代わりする価値があるので、使い捨て DB を何度も init し直す用途では有効。**

## わかったこと

1. **軽いクエリの速度は『転送経路』でほぼ決まる。DBエンジンの差ではない。**
   point SELECT は native socket 34k > native TCP 25k > Apple container 9k > Docker 4.4k。
   中身は全部同じ Postgres 17。違いは「unix socket / loopback TCP / 軽量VM / Docker の VM+ポート転送」という
   1往復あたりのコスト(p99: 0.04ms → 0.06ms → 0.19ms → 0.28ms)。

2. **RAM ディスクと SSD の速度差は、durable(`fsync=on`)で測っても誤差(c1 +7% / c8 +2%)。**
   native PG の 2x2(上記「RAMディスク vs SSD を分離して再検証」)より。理由は macOS の `fsync` が
   `F_FULLFSYNC` を出さず物理フラッシュしないため、SSD パスでも同期点で物理ディスクを待っていないこと。
   → **速度目的なら RAM ディスクは不要。ただし SSD の書き込み摩耗を避ける用途では有効**(物理書き戻しは発生するため)。
   なお速さの源泉は別にあり、`fsync=off` は c8 書き込みスループットを +20% する。

3. **チューニングは書き込みに効く(読み取りには無関係)。**
   docker default 3,148 → tuned 4,393(single INSERT, +40%)。SELECT/JOIN は不変。

4. **重いクエリ(JOIN+集約)は CPU 律速で、本物の PG が圧勝。**
   real PG ~520〜620 ops/s に対し、PGlite 95、pg-mem 21。
   pg-mem の JS 再実装と PGlite の WASM はプランナ/実行器が弱く、実クエリで 6〜25 倍遅い。

5. **互換性 × 速度:**
   - native / Docker / Apple container / PGlite = **本物の Postgres**(高互換)
   - pg-mem = PG の **部分 JS 再実装**(低互換)で、かつ実クエリ最遅。テスト専用。
   - PGlite は本物PGをWASM化。組み込みで高互換だが、起動 389ms・単一接続・JOIN は native の 1/6。

6. **Docker/コンテナが遅いのは「ホスト→VM 境界」だけ。同じ Docker の内側なら native 並み**(スループット表の〔参考〕行: docker VM内 26,561 ↔ ホストから 4,389 tps)。
   → **アプリも同じ Docker 網に入れれば速度は戻る**(別コンテナならコンテナ間TCP、最速は同一コンテナ/ソケット共有)。
   ただし Mac 開発では Rust ビルドを Docker 化する不利がある(README「Docker は内側なら速い」参照)。

## 用途別のおすすめ

- **開発用ローカルDB / 使い捨て運用** → **ネイティブ Postgres(Postgres.app か `brew install postgresql@17`)を
  自分で起動し、unix ソケット接続、`fsync=off` 等でチューニング。** 最速かつ 100% 互換。
  RAM ディスクは任意(速度差≒0、SSD 摩耗を避けたい時だけ)。Postgres.app と brew 版はエンジン同一で、利点はメニューバー常駐 GUI。
- **十分速ければコンテナでもよい、を優先** → **Apple container(~9k ops/s)が Docker(~4.4k)の倍速**。
  Docker Desktop の VM+ポート転送がボトルネック。コンテナで行くなら Apple container 推奨。
- **CIや自動テストで使い捨て・インストール不要** → **PGlite**(本物PG・高互換・ゼロ常駐)。pg-mem は最速でもないうえ低互換なので、
  よほど軽い単体テスト以外では避ける。

## 補足・限界

- 単一コネクション計測。実アプリでコネクションプール+並列を使うとサーバ型(native/container)はコア数分スケールするが、
  PGlite/pg-mem は単一スレッドなので頭打ち。差はさらに広がる方向。
- Docker は Docker Desktop の VM 経由。Colima(vz/virtiofs)等に替えると多少縮むが、native には届かない。
- 1台・各1回の測定。相対順位とオーダーは安定。

## 再現手順

```sh
cd pg-bench && npm install

# 1) 埋め込み系(即実行)
node bench.mjs pglite pglite
node bench.mjs pgmem  pgmem

# 2) Docker(tmpfs=メモリ / SSD / 無チューニング)— docker_setup 参照
node bench.mjs pg docker-tmpfs-tuned "postgres://postgres@127.0.0.1:5440/postgres"

# 3) ネイティブ PG on RAM ディスク(下記)
node bench.mjs pg native-ram-socket "postgresql://postgres@/postgres?host=/tmp&port=5443"

# 4) Apple container
node bench.mjs pg apple-container-pub "postgres://postgres@127.0.0.1:5444/postgres"

# 集計
node summary.mjs
```

RAM ディスク + ネイティブ PG の立ち上げ:

```sh
PGBIN=/opt/homebrew/opt/postgresql@17/bin
DISK=$(hdiutil attach -nomount ram://2097152 | awk '{print $1}')   # 1GB
diskutil erasevolume HFS+ 'pgram' "$DISK"
$PGBIN/initdb -D /Volumes/pgram/pgdata -U postgres -A trust
$PGBIN/pg_ctl -D /Volumes/pgram/pgdata -l /tmp/pgram.log \
  -o "-p 5443 -k /tmp -c fsync=off -c synchronous_commit=off -c full_page_writes=off -c shared_buffers=512MB" start
```

RAMディスク vs 実SSD の durability 2x2(上表)を測り直す:

```sh
bash durability_bench.sh   # 作業ディレクトリと RAM ディスクは終了時に自動で後始末(rm 対象は $TMPDIR 配下のみ)
```
