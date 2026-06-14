#!/usr/bin/env bash
# Rust(lastshot) vs Node(lastshot-node) の API 速度比較ベンチ。
#
# 同一 Postgres・同一クエリ・同一接続(unixソケット)で、3 エンドポイント × 同時数スイープ ×
# 並列モード(単一/全コア) を oha で叩き、結果(JSON)と CPU サンプルを results/ に貯める。
# 集計は `node summary.mjs` で REPORT.md に出す。
#
# 前提:
#   - ネイティブ Postgres 起動済み + bench migration 適用済み:
#       (cd ../../lastshot && ./run db-start && ./run db-setup && ./run db-migrate)
#   - oha / node / cargo が PATH にある。
#
# 主な env(全て上書き可):
#   DURATION=10  WARMUP=3  POOL_MAX=16  MULTI_WORKERS=8
#   CONNS="1 8 32 64 128 256"  MODES="single multi"  ENDPOINTS="ping light heavy"
#   RUST_PORT=4001  NODE_PORT=4002  PGDATABASE=<自動>
#   例(短縮): DURATION=5 CONNS="1 64 256" ENDPOINTS="ping light heavy" ./bench.sh
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"
BENCH_DIR="$PWD"
NODE_DIR="$(cd .. && pwd)"
LASTSHOT_DIR="$(cd ../../lastshot && pwd)"
REPO_ROOT="$(cd ../.. && pwd)"

# --- PGDATABASE を ../lastshot/run と同じ規則で決める(worktree スロット) ---------------
_wt="$(basename "$REPO_ROOT")"                                   # 例: dan1 / eikodan
_slot="$(printf '%s' "$_wt" | grep -oE '[0-9]+$' || true)"      # 末尾数字
PGDATABASE="${PGDATABASE:-lastshot${_slot:+_dan$_slot}}"
export PGDATABASE
unset DATABASE_URL || true                                       # 未設定 = unix ソケット経路

# --- 設定 ---------------------------------------------------------------------------
DURATION="${DURATION:-10}"
WARMUP="${WARMUP:-3}"
POOL_MAX="${POOL_MAX:-16}"
MULTI_WORKERS="${MULTI_WORKERS:-8}"
CONNS="${CONNS:-1 8 32 64 128 256}"
MODES="${MODES:-single multi}"
ENDPOINTS="${ENDPOINTS:-ping light heavy}"
LANGS="${LANGS:-rust node}"          # 計測対象の言語(rust専用比較などで絞れる)
RUST_PORT="${RUST_PORT:-4001}"
NODE_PORT="${NODE_PORT:-4002}"

RESULTS="${RESULTS:-$BENCH_DIR/results}"          # 出力先(上書き可。pool違い等を別dirへ隔離するため)
REPORT_OUT="${REPORT_OUT:-$BENCH_DIR/REPORT.md}"  # 集計の出力先(同上)
mkdir -p "$RESULTS"

ep_path() { case "$1" in ping) echo /ping ;; light) echo /db/light ;; lightpipe) echo /db/light_pipe ;; heavy) echo /db/heavy ;; sleep) echo /db/sleep ;; esac; }

wait_up() { # <base_url> : /ping が 200 を返すまで待つ
  for _ in $(seq 1 100); do
    if curl -fsS "$1/ping" >/dev/null 2>&1; then return 0; fi
    sleep 0.1
  done
  echo "!! server did not come up: $1" >&2; return 1
}

stop_all() { pkill -f 'target/release/app' 2>/dev/null || true; pkill -f 'lastshot-node/server.mjs' 2>/dev/null || true; sleep 0.5; }
trap stop_all EXIT

# --- Rust release ビルド ------------------------------------------------------------
echo "== building rust (release) =="
( cd "$LASTSHOT_DIR" && cargo build --release -p app )
RUST_BIN="$LASTSHOT_DIR/target/release/app"

start_rust() { # <threads>
  TOKIO_WORKER_THREADS="$1" POOL_MAX="$POOL_MAX" PGDATABASE="$PGDATABASE" \
    HOST=127.0.0.1 PORT="$RUST_PORT" "$RUST_BIN" >"$RESULTS/_rust.log" 2>&1 &
  wait_up "http://127.0.0.1:$RUST_PORT"
}
start_node() { # <workers>
  WORKERS="$1" POOL_MAX="$POOL_MAX" PGDATABASE="$PGDATABASE" \
    PORT="$NODE_PORT" NODE_ENV=production node "$NODE_DIR/server.mjs" >"$RESULTS/_node.log" 2>&1 &
  wait_up "http://127.0.0.1:$NODE_PORT"
}

# 1 言語ぶんの計測(エンドポイント × 同時数)。
run_lang() { # <lang> <mode> <base_url> <cpu_pattern>
  local lang="$1" mode="$2" base="$3" pat="$4" ep path c name url
  for ep in $ENDPOINTS; do
    path="$(ep_path "$ep")"; url="$base$path"
    echo "  -- $lang/$mode $path (warmup ${WARMUP}s)"
    oha --no-tui --output-format quiet -z "${WARMUP}s" -c 32 "$url" >/dev/null 2>&1 || true
    for c in $CONNS; do
      name="${lang}_${mode}_${ep}_c${c}"
      bash "$BENCH_DIR/cpusample.sh" "$pat" "$RESULTS/${name}.cpu" 0.5 &
      local cpid=$!
      oha --no-tui --output-format json -o "$RESULTS/${name}.json" -z "${DURATION}s" -c "$c" "$url"
      kill "$cpid" 2>/dev/null || true
      printf '     c=%-4s done\n' "$c"
    done
  done
}

echo "== config: dur=${DURATION}s warmup=${WARMUP}s pool=${POOL_MAX} workers(multi)=${MULTI_WORKERS} db=${PGDATABASE} =="
echo "== conns: ${CONNS} | modes: ${MODES} | endpoints: ${ENDPOINTS} =="

for mode in $MODES; do
  if [ "$mode" = "single" ]; then th=1; wk=1; else th="$MULTI_WORKERS"; wk="$MULTI_WORKERS"; fi
  echo "== mode=$mode (rust threads=$th / node workers=$wk) =="

  case " $LANGS " in *" rust "*)
    echo "= rust ="
    start_rust "$th"
    run_lang rust "$mode" "http://127.0.0.1:$RUST_PORT" 'target/release/app'
    stop_all
  ;; esac

  case " $LANGS " in *" node "*)
    echo "= node ="
    start_node "$wk"
    run_lang node "$mode" "http://127.0.0.1:$NODE_PORT" 'lastshot-node/server.mjs'
    stop_all
  ;; esac
done

echo "== done. 集計: node summary.mjs =="
RESULTS_DIR="$RESULTS" REPORT_OUT="$REPORT_OUT" node "$BENCH_DIR/summary.mjs"
