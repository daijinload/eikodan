# pg-bench

「最速 Postgres、かつ SSD ではなくメモリに書く」を Mac 上でどう実現するかを、
**アプリから接続した時の実効速度**で横並び比較するベンチ。

対象: PGlite / pg-mem / Docker(tmpfs・SSD) / Apple container / ネイティブPG on RAMディスク。

- 結果と考察 → [POSTGRES_BENCHMARK.md](./POSTGRES_BENCHMARK.md)
- 計測コード → [bench.mjs](./bench.mjs)(バックエンドごとに別プロセスで起動)
- 集計 → `node summary.mjs`(`results/*.json` を Markdown 表に)

```sh
npm install
node bench.mjs pglite pglite      # 埋め込み系は即実行
node bench.mjs pg <label> <conn>  # サーバ型は接続文字列を渡す
node summary.mjs
```

サーバ型の立ち上げ・チューニング・RAMディスク作成は [POSTGRES_BENCHMARK.md](./POSTGRES_BENCHMARK.md) の「再現手順」を参照。

## 一行結論

Mac で最速かつ高互換なのは **ネイティブPostgresを立てて unix ソケット接続**。
速度を生むのは "VMを挟まないこと" と `fsync=off` で、**RAMディスクの速度上乗せは誤差**(durable でも +2〜7%)。
ただし RAMディスクは **SSD の書き込み摩耗を肩代わり**できるので、使い捨て DB を何度も init し直す用途では有効。

> ⚠️ 計測はすべて **macOS (Apple Silicon)**。「durable でも RAM≒SSD」は macOS の fsync が物理フラッシュしないため成り立つ話で、Linux では `fsync=on` 時に RAM ディスクが効く。OS が変われば結論も変わりうる。

詳細は [POSTGRES_BENCHMARK.md](./POSTGRES_BENCHMARK.md) の「RAMディスク vs SSD を分離して再検証」。

## 採用構成と起動方法

ベンチの勝者(ネイティブ Postgres)を **`brew install postgresql@17`** で導入し、
ベンチと同じチューニングを `ALTER SYSTEM`(`postgresql.auto.conf`)で適用済み。
データは SSD の `/opt/homebrew/var/postgresql@17` に永続。**ログイン自動起動はオフ。自分で起動する運用。**

適用済みチューニング:
`fsync=off / synchronous_commit=off / full_page_writes=off / shared_buffers=512MB / wal_level=minimal / max_wal_size=4GB`
(`fsync=off` 運用なので、クラッシュ時は潰して `initdb` し直す使い捨て前提)

> SSD の書き込み摩耗を避けたい場合は、データディレクトリを RAM ディスクに置く手もある(速度はほぼ同じだが SSD への物理書き込みが消える)。
> 手順は [POSTGRES_BENCHMARK.md](./POSTGRES_BENCHMARK.md) の「再現手順」。

### 起動 / 停止

postgresql@17 は keg-only(PATH 非追加)なので `brew services run` が手軽。

```sh
brew services run  postgresql@17    # 起動(ログインには登録されない=手動運用)
brew services stop postgresql@17    # 停止
```

brew を介さず直接やる場合:

```sh
PGBIN=/opt/homebrew/opt/postgresql@17/bin
$PGBIN/pg_ctl -D /opt/homebrew/var/postgresql@17 -l /opt/homebrew/var/log/postgresql@17.log start
$PGBIN/pg_ctl -D /opt/homebrew/var/postgresql@17 stop
```

> ⚠️ `brew services start`(`run` ではなく `start`)を使うと**ログイン常駐に戻る**。手動運用の間は `run` を使う。

### 接続先(起動後)

```
unix socket (最速) : postgresql://daijinload@/postgres?host=/tmp
TCP                : postgresql://daijinload@127.0.0.1:5432/postgres
```

### その他

```sh
# psql / pg_ctl をコマンドで直接使いたい場合だけ PATH 追加(任意)
echo 'export PATH="/opt/homebrew/opt/postgresql@17/bin:$PATH"' >> ~/.zprofile

# チューニングを元に戻す
/opt/homebrew/opt/postgresql@17/bin/psql -p 5432 -d postgres -c 'ALTER SYSTEM RESET ALL;'
# → その後 stop/start で再読込
```

## Docker は内側なら速い(アプリの置き場所の話)

一言: **Docker が遅いのは「ホスト→VM の往復」だけで、アプリも同じ Docker 網に入れれば速度は戻る。
ただし Mac 開発では Rust を Docker 化する不利の方が大きいので、開発はネイティブ・本番/結合は compose、と分けるのが良い。**

### 仕組み

- 遅さの正体は macOS Docker Desktop の `ホスト→Linux VM` 往復(+0.05〜0.19ms/クエリ)。
- **同一 Docker 内なら native 並み**(VM内 26,561 tps ↔ ホストから 4,389 tps)。→ [POSTGRES_BENCHMARK.md](./POSTGRES_BENCHMARK.md)「わかったこと #6」。
- ただし「アプリを別コンテナ」に置くと `コンテナ間TCP(ブリッジ網)`になり、ホスト経由よりは速いが unix ソケットよりは落ちる。
  最速を狙うなら **同一コンテナ or ソケットを共有ボリュームで渡す**。

### Mac 開発で「Rust も Docker 内」が微妙な理由

- このリポジトリの最優先要件は「開発が高速」(lld 切替 / subsecond ホットパッチ / fastweb のビルド回避)。
  **Rust のビルド/実行を Docker 内に入れると、この高速イテレーションが弱る**(VMファイル共有・非ネイティブ)。
- ネイティブビルドしたバイナリは Mach-O で **Linux では動かない** → `aarch64-unknown-linux-*` へクロスビルド or
  ビルダコンテナが必要。「バイナリをポンおき」は実は無料ではない。

### 本番 / 結合テストなら compose で同一網に

- 本番は app も PG も Linux で、**今回の VM 税は存在しない**(= macOS ローカル開発特有のアーティファクト)。
- なので docker-compose で `app` + `postgres` を同一ネットワーク(できればソケット共有 or コンテナ間TCP)に置けば速い。
  結合・E2E もこれでよい。Mac 開発の都合を、本番に存在しない問題のために持ち込まない。

### 使い分け

| 場面 | 構成 |
|---|---|
| ローカル開発(内ループ) | **ネイティブ Rust + ネイティブ PG(unix ソケット)** = 最速・摩擦ゼロ |
| 本番 / 結合・E2E | **compose で app+pg を同一網**(VM 税なし) |
