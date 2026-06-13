#!/usr/bin/env bash
# RAMディスク vs 実SSD を「durability ON / OFF」の2x2で測り直す。
# 前回 docker-tmpfs vs docker-ssd は両方 fsync=off で、SSD側も同期書き込みしておらず
# 変数を分離できていなかった。今回は native PG で fsync=on(durable)も測る。
#
# ⚠️ macOS 専用(hdiutil / diskutil を使用)。かつ結論は macOS の fsync 挙動に依存する:
#    macOS の fsync は F_FULLFSYNC を出さず物理フラッシュしないため durable でも RAM≒SSD になる。
#    Linux など fsync が実フラッシュする OS では fsync=on 時に RAM ディスクが効くので結果は変わる。
#
# 安全策:
#  - 破壊するのは mktemp -d で作った $TMPDIR 配下の作業ディレクトリのみ(rm -rf 対象を限定)
#  - RAMディスクは hdiutil で新規作成したデバイスのみ erase/detach($RAMDEV を /dev/diskN で検証)
#  - 既存の本番データ(/opt/homebrew/var/postgresql@17)や $HOME には一切触れない
#  - 使うポートは 5455(SSD) / 5456(RAM) のみ。終了時に必ず後始末(trap)
set -euo pipefail

PGBIN=/opt/homebrew/opt/postgresql@17/bin
SCALE=10        # pgbench scale (~150MB)
DUR=8           # 各 pgbench 計測の秒数
SSD_PORT=5455
RAM_PORT=5456
RAM_MOUNT=/Volumes/pgdurram

WORK="$(mktemp -d "${TMPDIR%/}/pgdurbench.XXXXXX")"
SSD_DATA="$WORK/ssd_pgdata"
RAM_DATA="$RAM_MOUNT/pgdata"
RAMDEV=""
OUT="$WORK/out.txt"

log(){ printf '\n=== %s ===\n' "$*"; }

cleanup(){
  set +e
  [ -d "$SSD_DATA" ] && "$PGBIN/pg_ctl" -D "$SSD_DATA" -m immediate stop >/dev/null 2>&1
  [ -d "$RAM_DATA" ] && "$PGBIN/pg_ctl" -D "$RAM_DATA" -m immediate stop >/dev/null 2>&1
  if [ -n "$RAMDEV" ]; then
    hdiutil detach "$RAMDEV" >/dev/null 2>&1 || diskutil eject "$RAMDEV" >/dev/null 2>&1
  fi
  # rm は mktemp で作った作業ディレクトリだけに限定(多重ガード)
  case "$WORK" in
    "${TMPDIR%/}/pgdurbench."*) [ -d "$WORK" ] && rm -rf "$WORK" ;;
    *) echo "SAFETY: refuse to rm '$WORK'" ;;
  esac
}
trap cleanup EXIT

log "create RAM disk (2GB)"
RAMDEV="$(hdiutil attach -nomount ram://4194304 | awk '{print $1}' | tr -d '[:space:]')"
case "$RAMDEV" in
  /dev/disk[0-9]*) echo "RAMDEV=$RAMDEV" ;;
  *) echo "unexpected ram device: '$RAMDEV'"; exit 1 ;;
esac
diskutil erasevolume HFS+ pgdurram "$RAMDEV" >/dev/null

log "initdb (SSD=$SSD_DATA / RAM=$RAM_DATA)"
mkdir -p "$SSD_DATA" "$RAM_DATA"
"$PGBIN/initdb" -D "$SSD_DATA" -U postgres -A trust >/dev/null
"$PGBIN/initdb" -D "$RAM_DATA" -U postgres -A trust >/dev/null

run_bench(){
  # name datadir port fsync(on/off)
  local name="$1" data="$2" port="$3" fs="$4"
  local sc; [ "$fs" = on ] && sc=on || sc=off
  "$PGBIN/pg_ctl" -D "$data" -m immediate stop >/dev/null 2>&1 || true
  "$PGBIN/pg_ctl" -D "$data" -l "$data/server.log" \
    -o "-p $port -k /tmp -c fsync=$fs -c synchronous_commit=$sc -c full_page_writes=$fs -c shared_buffers=512MB -c max_wal_size=4GB" \
    start >/dev/null
  for _ in $(seq 1 40); do "$PGBIN/pg_isready" -h /tmp -p "$port" >/dev/null 2>&1 && break; sleep 0.3; done
  local wsm
  wsm="$("$PGBIN/psql" -h /tmp -p "$port" -U postgres -tAc 'show wal_sync_method' postgres 2>/dev/null)"
  "$PGBIN/pgbench" -i -s "$SCALE" -h /tmp -p "$port" -U postgres postgres >/dev/null 2>&1
  local c1 c8
  c1=$("$PGBIN/pgbench" -h /tmp -p "$port" -U postgres -c 1 -j 1 -T "$DUR" -N postgres 2>/dev/null | awk '/without initial/{print $3}')
  c8=$("$PGBIN/pgbench" -h /tmp -p "$port" -U postgres -c 8 -j 8 -T "$DUR" -N postgres 2>/dev/null | awk '/without initial/{print $3}')
  "$PGBIN/pg_ctl" -D "$data" -m immediate stop >/dev/null 2>&1 || true
  printf '%-26s c1=%-12s c8=%-12s (wal_sync=%s)\n' "$name" "$c1" "$c8" "$wsm"
  echo "$name|$c1|$c8|$wsm" >> "$OUT"
}

log "benchmark (pgbench -N = 書き込み律速, c1=1接続レイテンシ / c8=8接続スループット)"
run_bench "SSD durable(fsync=on)"  "$SSD_DATA" "$SSD_PORT" on
run_bench "SSD throwaway(fsync=off)" "$SSD_DATA" "$SSD_PORT" off
run_bench "RAM durable(fsync=on)"  "$RAM_DATA" "$RAM_PORT" on
run_bench "RAM throwaway(fsync=off)" "$RAM_DATA" "$RAM_PORT" off

log "RESULT (tps)"
column -t -s '|' "$OUT"
