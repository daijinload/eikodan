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

> ⚠️ **上の表は「単一接続を1クエリずつ同期で叩いた時のレイテンシ」**を測っている。
> これは「1往復の速さ」であって「最速スループット(並列でどこまで出るか)」ではない。
> 設定ミスを疑って再点検した結果(下記)、**数値自体は正しいが“レイテンシ計測”であり、
> スループットは別途 pgbench で測り直した**。結論(native が最速)は変わらず、むしろ並列では差が拡大する。

## 設定の再点検(チューニング・環境の検証)

「Docker が遅いのは設定ミスでは?」を潰すための確認:

- **チューニングは全サーバで同一に効いている。** `pg_settings` で確認: tmpfs / ssd / apple / native の4つは
  `fsync=off, synchronous_commit=off, full_page_writes=off, shared_buffers=512MB(65536×8kB), wal_level=minimal, max_wal_size=4GB`
  が全て **`source=command line`**。default だけ stock(`fsync=on` 等)。→ tmpfs↔native の差は設定ではない。
- **tmpfs は本当にメモリ。** `/var/lib/postgresql/data` は `tmpfs size=2097152k`(2GB RAM)で mount 確認済み。
- **エンジン素の性能は全部ほぼ同じ。** ホスト網を除外して **VM内 / native で pgbench(-S, 単一クライアント)** を回すと:
  docker-tmpfs **26,561 tps**(0.038ms) / apple **28,400** / native TCP **38,479** / native socket **53,168**。
  → 中身は同じ Postgres 17 なので素の力は同等。native socket が高いのは「VMを挟まない+unixソケット」だけが理由。

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
- **Macホストから繋ぐ限り、native socket が圧勝**(c8 で 218k tps、Docker の約12倍、Apple container の約5.5倍)。
- **VM内なら全部同等**(26k〜28k / 110k〜222k)。差はすべて **ホスト→VM の往復(+0.05〜0.19ms)**。
- **Apple container は Docker の約2倍**(c1: 11,640 vs 4,389 / c8: 39,814 vs 18,241)。コンテナで行くなら Apple 一択。
- 私の最初の Node 単一接続計測(point SELECT)は、ホスト pgbench c1 と一致(docker 4,389≒4,337, native socket は Node 34k < pgbench 53k=Nodeクライアントのオーバーヘッド差)。→ **計測は正しい。ただしレイテンシであってスループットではない。**
- **PGlite / pg-mem は単一スレッドなので並列で伸びない**:単一接続の数値(PGlite point ~7.6k)がほぼ上限。
  対して native PG はプール+並列で 20万 tps 級まで伸びる → 負荷時の差は単一接続時よりさらに開く。

## 結論(プレーンな一文)

**Mac で最速かつ高互換なのは「ネイティブ Postgres を常駐させ、unix ソケットで繋ぐ」構成。
RAM ディスクは fsync を切れば速度的にはほぼ不要(差≒0)、効くのはチューニングと“VM を挟まないこと”。**

## わかったこと

1. **軽いクエリの速度は『転送経路』でほぼ決まる。DBエンジンの差ではない。**
   point SELECT は native socket 34k > native TCP 25k > Apple container 9k > Docker 4.4k。
   中身は全部同じ Postgres 17。違いは「unix socket / loopback TCP / 軽量VM / Docker の VM+ポート転送」という
   1往復あたりのコスト(p99: 0.04ms → 0.06ms → 0.19ms → 0.28ms)。

2. **tmpfs(メモリ)と SSD の差は、チューニング済みならほぼゼロ。**
   docker tmpfs 4,393 vs docker ssd 4,389(single INSERT)。
   `fsync=off`/`synchronous_commit=off` でコミット毎のディスク同期が消え、ホットデータはページキャッシュに乗るため、
   RAM ディスク化の上乗せは誤差。**「メモリに書く」効果の大半はチューニングで得られている。**

3. **チューニングは書き込みに効く(読み取りには無関係)。**
   docker default 3,148 → tuned 4,393(single INSERT, +40%)。SELECT/JOIN は不変。

4. **重いクエリ(JOIN+集約)は CPU 律速で、本物の PG が圧勝。**
   real PG ~520〜620 ops/s に対し、PGlite 95、pg-mem 21。
   pg-mem の JS 再実装と PGlite の WASM はプランナ/実行器が弱く、実クエリで 6〜25 倍遅い。

5. **互換性 × 速度:**
   - native / Docker / Apple container / PGlite = **本物の Postgres**(高互換)
   - pg-mem = PG の **部分 JS 再実装**(低互換)で、かつ実クエリ最遅。テスト専用。
   - PGlite は本物PGをWASM化。組み込みで高互換だが、起動 389ms・単一接続・JOIN は native の 1/6。

6. **Docker/コンテナが遅いのは「ホスト→VM 境界」だけ。同じ Docker の内側なら native 並みに速い。**
   同一コンテナ内で pgbench を回すと docker-tmpfs **26,561 tps**(0.038ms)/ Apple **28,400** で、native(38k〜53k)と同オーダー。
   同じものをホストから繋ぐと docker **4,389** / Apple **11,640** に落ちる。差は全部ホスト→VM の往復コスト。
   → **アプリも同じ Docker 網に入れれば速度は戻る**(別コンテナならコンテナ間TCP、最速は同一コンテナ/ソケット共有)。
   ただし Mac 開発では Rust ビルドを Docker 化する不利がある(README「Docker は内側なら速い」参照)。

## 用途別のおすすめ

- **開発用ローカルDB / PC常駐させて使い捨て運用** → **ネイティブ Postgres(Postgres.app か `brew install postgresql@17`)を
  ログイン常駐させ、unix ソケット接続、`fsync=off` 等でチューニング。** 最速かつ 100% 互換。RAM ディスクは任意(速度差≒0、
  “ディスク痕跡ゼロ”が欲しい時だけ)。Postgres.app と brew 版はエンジン同一で速度も同じ。Postgres.app の利点はメニューバー常駐 GUI。
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
