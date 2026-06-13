#!/usr/bin/env bash
# Docker / Apple container の Postgres を起動する。tmpfs=メモリ, SSD, 無チューニングの3種 + Apple container。
set -euo pipefail

TUNE="-c fsync=off -c synchronous_commit=off -c full_page_writes=off -c shared_buffers=512MB -c max_wal_size=4GB -c wal_level=minimal -c max_wal_senders=0"

echo "== docker: tmpfs(メモリ) 最大チューニング :5440 =="
docker rm -f pgbench-tmpfs 2>/dev/null || true
docker run -d --name pgbench-tmpfs -e POSTGRES_HOST_AUTH_METHOD=trust \
  --tmpfs /var/lib/postgresql/data:rw,size=2g -p 5440:5432 postgres:17 $TUNE

echo "== docker: SSD 同チューニング :5441 =="
docker rm -f pgbench-ssd 2>/dev/null || true
docker run -d --name pgbench-ssd -e POSTGRES_HOST_AUTH_METHOD=trust \
  -p 5441:5432 postgres:17 $TUNE

echo "== docker: デフォルト(無チューニング, SSD) :5442 =="
docker rm -f pgbench-default 2>/dev/null || true
docker run -d --name pgbench-default -e POSTGRES_HOST_AUTH_METHOD=trust \
  -p 5442:5432 postgres:17

echo "== apple container: tmpfs(メモリ) 同チューニング :5444 =="
if command -v container >/dev/null; then
  container system start || true
  container system kernel set --recommended || true
  container rm -f pgac 2>/dev/null || true
  container run -d --name pgac -e POSTGRES_HOST_AUTH_METHOD=trust \
    --tmpfs /var/lib/postgresql/data -p 5444:5432 postgres:17 $TUNE
fi

echo "done. 数秒待ってから bench.mjs を実行してください。"
echo "停止: docker rm -f pgbench-tmpfs pgbench-ssd pgbench-default ; container rm -f pgac"
