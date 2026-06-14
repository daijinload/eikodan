// 計測ログ（1行1JSON）を読み、外れ値を除いて統計を出す。
// 外れ値処理: phase=="cold" は除外（初回起動コスト）。残りも上下10%トリム。
// 使い方: node ab-bench.mjs ... > ab.log ; node analyze.mjs <label> < ab.log
import { createInterface } from "node:readline";

const label = process.argv[2] ?? "tool";
const rows = [];
const rl = createInterface({ input: process.stdin });
for await (const line of rl) {
  const s = line.trim();
  if (!s) continue;
  try { rows.push(JSON.parse(s)); } catch {}
}

const cold = rows.filter((r) => r.phase === "cold");
const warm = rows.filter((r) => r.phase === "warm");

function stats(arr) {
  if (!arr.length) return null;
  const s = [...arr].sort((a, b) => a - b);
  const k = Math.floor(s.length * 0.1);                 // 上下10%トリム
  const t = s.slice(k, s.length - k);
  const sum = t.reduce((a, b) => a + b, 0);
  const mean = sum / t.length;
  const median = s[Math.floor(s.length / 2)];
  return { n: arr.length, kept: t.length, median, trimmedMean: mean, min: s[0], max: s[s.length - 1] };
}

function group(op) {
  const r = warm.filter((x) => x.op === op);
  return { ms: stats(r.map((x) => x.ms)), bytes: stats(r.map((x) => x.bytes)) };
}

const ops = [...new Set(warm.map((r) => r.op))];
const fmt = (s, d = 1) => (s == null ? "-" : s.toFixed(d));
const out = { label, cold: {}, warm: {} };
for (const c of cold) out.cold[c.op] = { ms: c.ms, bytes: c.bytes };
let stepMsTrim = 0, stepBytesTrim = 0;
for (const op of ops) {
  const g = group(op);
  out.warm[op] = g;
  stepMsTrim += g.ms?.trimmedMean ?? 0;
  stepBytesTrim += g.bytes?.trimmedMean ?? 0;
}

console.log(`\n=== ${label} ===`);
for (const op of Object.keys(out.cold)) {
  console.log(`cold ${op}: ${fmt(out.cold[op].ms)} ms, ${out.cold[op].bytes} B`);
}
console.log(`warm (cold除外 + 上下10%トリム):`);
for (const op of ops) {
  const g = out.warm[op];
  console.log(
    `  ${op.padEnd(9)} ms: median=${fmt(g.ms.median)} trimMean=${fmt(g.ms.trimmedMean)} ` +
    `[min ${fmt(g.ms.min)} / max ${fmt(g.ms.max)}, n=${g.ms.n}→${g.ms.kept}]   ` +
    `bytes: median=${fmt(g.bytes.median, 0)} trimMean=${fmt(g.bytes.trimmedMean, 0)}`
  );
}
console.log(`  ── 1ステップ計(trimMean): ${fmt(stepMsTrim)} ms, ${fmt(stepBytesTrim, 0)} B`);
