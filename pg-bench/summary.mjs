// results/*.json を読み込み、横並びの Markdown 表を出力する
import { readdirSync, readFileSync } from 'node:fs';

const dir = new URL('./results/', import.meta.url);
const files = readdirSync(dir).filter((f) => f.endsWith('.json'));
const rows = files.map((f) => JSON.parse(readFileSync(new URL(f, dir))));
rows.sort((a, b) => a.label.localeCompare(b.label));

const metrics = [
  ['init (ms)', (r) => r.initMs, 'lower'],
  ['schema (ms)', (r) => r.phases.schemaMs, 'lower'],
  ['bulk seed (rows/s)', (r) => r.phases.bulkSeedRowsPerSec, 'higher'],
  ['single INSERT (ops/s)', (r) => r.phases.singleInsert.opsPerSec, 'higher'],
  ['  └ p99 (ms)', (r) => r.phases.singleInsert.p99ms, 'lower'],
  ['point SELECT (ops/s)', (r) => r.phases.pointSelect.opsPerSec, 'higher'],
  ['  └ p99 (ms)', (r) => r.phases.pointSelect.p99ms, 'lower'],
  ['indexed SELECT (ops/s)', (r) => r.phases.indexedSelect.opsPerSec, 'higher'],
  ['UPDATE (ops/s)', (r) => r.phases.update.opsPerSec, 'higher'],
  ['JOIN+agg (ops/s)', (r) => r.phases.joinAgg.opsPerSec, 'higher'],
];

const labels = rows.map((r) => r.label);
const header = `| metric | ${labels.join(' | ')} |`;
const sep = `|---|${labels.map(() => '---:').join('|')}|`;
const lines = [header, sep];
for (const [name, get] of metrics) {
  const cells = rows.map((r) => {
    const v = get(r);
    return typeof v === 'number' ? v.toLocaleString() : String(v);
  });
  lines.push(`| ${name} | ${cells.join(' | ')} |`);
}

console.log('\n' + lines.join('\n') + '\n');
for (const r of rows) console.log(`- ${r.label}: ${r.name}`);
