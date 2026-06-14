// agent-browser ベンチ: MCP 版と同じ「1ステップ = snapshot(状態確認) → click(+1)」を
// CLI 呼び出しで回し、各 op の wall-clock とstdoutバイト数を計測する。
// CLI のプロセス起動コストも込み（エージェントが毎回 `agent-browser ...` を叩く実コスト）。
//
// 使い方: node ab-bench.mjs <url> <steps>
import { spawnSync } from "node:child_process";

const URL = process.argv[2] ?? "http://localhost:8080";
const STEPS = Number(process.argv[3] ?? 30);
const now = () => Number(process.hrtime.bigint() / 1000n) / 1000;

function ab(args) {
  const t0 = now();
  const r = spawnSync("agent-browser", args, { encoding: "utf8" });
  const ms = now() - t0;
  const bytes = Buffer.byteLength((r.stdout ?? "") + (r.stderr ?? ""), "utf8");
  return { ms, bytes, out: r.stdout ?? "" };
}

// cold: ブラウザ起動 + open（初回外れ値として別計測）
const open = ab(["open", URL]);
console.log(JSON.stringify({ phase: "cold", op: "open", ms: open.ms, bytes: open.bytes }));

for (let i = 0; i < STEPS; i++) {
  // MCP の browser_snapshot に対応する「状態確認」op。-i = インタラクティブ要素のみ（既定の使い方）
  const snap = ab(["snapshot", "-i"]);
  const ref = (snap.out.match(/button "\+1"\s*\[[^\]]*ref=(e\d+)\]/) || [])[1] ?? "e2";
  const click = ab(["click", "@" + ref]);
  console.log(JSON.stringify({ phase: "warm", op: "snapshot", i, ms: snap.ms, bytes: snap.bytes }));
  console.log(JSON.stringify({ phase: "warm", op: "click", i, ms: click.ms, bytes: click.bytes }));
}

const fin = ab(["eval", "[...document.querySelectorAll('p')].find(p=>p.textContent.startsWith('count')).textContent"]);
process.stderr.write(`[ab-bench] final ${fin.out.trim()} (expected count: ${STEPS})\n`);

ab(["close"]);
