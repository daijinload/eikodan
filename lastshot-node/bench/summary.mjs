// results/*.json (oha) + *.cpu (CPUサンプル) を集計し REPORT.md を生成する。
// 表は (モード × エンドポイント) ごとに、行=同時数、列= Rust/Node の req/s・p99・CPU秒/1k。
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const dir = process.env.RESULTS_DIR || resolve(here, 'results'); // 入力(上書き可)
const reportOut = process.env.REPORT_OUT || resolve(here, 'REPORT.md'); // 出力(上書き可)
const RE = /^(rust|node)_(single|multi)_(ping|light|lightpipe|heavy|sleep)_c(\d+)\.json$/;

const cells = {}; // key: mode|ep|conn -> { rust:{...}, node:{...} }

function avgCpu(name) {
  try {
    const txt = readFileSync(join(dir, `${name}.cpu`), 'utf8').trim();
    if (!txt) return null;
    const nums = txt.split('\n').map(Number).filter((n) => Number.isFinite(n));
    if (!nums.length) return null;
    return nums.reduce((a, b) => a + b, 0) / nums.length; // 平均 %cpu (100=1コア)
  } catch {
    return null;
  }
}

for (const f of readdirSync(dir)) {
  const m = RE.exec(f);
  if (!m) continue;
  const [, lang, mode, ep, connStr] = m;
  const conn = Number(connStr);
  const j = JSON.parse(readFileSync(join(dir, f)));
  const lp = j.latencyPercentiles || {};
  const total = j.summary?.total ?? null; // 実測秒
  const rps = j.summary?.requestsPerSec ?? null;
  const success = j.summary?.successRate ?? null;
  const reqs = j.statusCodeDistribution
    ? Object.values(j.statusCodeDistribution).reduce((a, b) => a + b, 0)
    : rps && total
      ? rps * total
      : null;
  const cpuPct = avgCpu(f.replace(/\.json$/, ''));
  let cpuPer1k = null;
  if (cpuPct != null && total != null && reqs) {
    const cpuSec = (cpuPct / 100) * total; // CPU秒
    cpuPer1k = (cpuSec * 1000) / reqs;
  }
  const key = `${mode}|${ep}|${conn}`;
  (cells[key] ||= {})[lang] = {
    rps,
    success,
    reqs,
    p50: lp.p50 != null ? lp.p50 * 1000 : null,
    p99: lp.p99 != null ? lp.p99 * 1000 : null,
    p999: lp['p99.9'] != null ? lp['p99.9'] * 1000 : null,
    cpuPct,
    cpuPer1k,
  };
}

const fmt = (v, d = 0) =>
  v == null ? '—' : Number(v).toLocaleString('en-US', { maximumFractionDigits: d, minimumFractionDigits: d });
const ratio = (a, b) => (a != null && b != null && b > 0 ? `${(a / b).toFixed(2)}×` : '—');

const MODES = ['single', 'multi'];
const MODE_LABEL = { single: '単一プロセス（1コア対1コア）', multi: '全コア（コア数を揃えた対決）' };
const EPS = ['ping', 'light', 'lightpipe', 'heavy', 'sleep'];
const EP_LABEL = {
  ping: '/ping（DBなし＝言語の素の天井）',
  light: '/db/light（点SELECT＝実APIの大半）',
  lightpipe: '/db/light_pipe（点SELECT・パイプライン版＝tokio-postgres）',
  heavy: '/db/heavy（重い集約＝DB律速領域）',
  sleep: '/db/sleep（pg_sleep＝純待ち）',
};

const connsOf = (mode, ep) =>
  [...new Set(Object.keys(cells).filter((k) => k.startsWith(`${mode}|${ep}|`)).map((k) => Number(k.split('|')[2])))].sort(
    (a, b) => a - b,
  );

const out = [];
out.push('# Rust(lastshot) vs Node(lastshot-node) API ベンチ結果');
out.push('');
out.push(
  '> 「DBが律速だから言語は関係ない」を検証する実測。同一 Postgres・同一クエリ・同一接続(unixソケット)で、' +
    'DBの重さ(ping→light→heavy)を軸に oha で叩いた。**生成は `bench.sh`、集計は `summary.mjs`**。',
);
out.push('');

for (const mode of MODES) {
  if (!Object.keys(cells).some((k) => k.startsWith(`${mode}|`))) continue;
  out.push(`## ${MODE_LABEL[mode]}`);
  out.push('');
  for (const ep of EPS) {
    const conns = connsOf(mode, ep);
    if (!conns.length) continue;
    out.push(`### ${EP_LABEL[ep]}`);
    out.push('');
    out.push('| 同時数 | Rust req/s | Node req/s | Rust優位 | Rust p99(ms) | Node p99(ms) | Rust CPU秒/1k | Node CPU秒/1k |');
    out.push('|---:|---:|---:|---:|---:|---:|---:|---:|');
    for (const c of conns) {
      const cell = cells[`${mode}|${ep}|${c}`] || {};
      const r = cell.rust || {};
      const n = cell.node || {};
      out.push(
        `| ${c} | ${fmt(r.rps)} | ${fmt(n.rps)} | ${ratio(r.rps, n.rps)} | ${fmt(r.p99, 2)} | ${fmt(n.p99, 2)} | ${fmt(
          r.cpuPer1k,
          3,
        )} | ${fmt(n.cpuPer1k, 3)} |`,
      );
    }
    out.push('');
  }
}

// 結論（読み解き）。表の傾向を言葉で固定する。数値は表を参照。
out.push('## 結論（読み解き）');
out.push('');
out.push(
  '**「DBが律速だから言語は関係ない」は、重いDBクエリ1点でしか成り立たない。実APIの大半では Rust が明確に速い。**',
);
out.push('');
out.push(
  '最もクリーンな比較は **単一プロセス（1コア対1コア）**（負荷生成とコアを奪い合わない）。そこでは:',
);
out.push('');
out.push(
  '- **/ping（DBなし）**: Rust が約 2× のスループット・CPU 半分・小さい p99。DBを持ち出せない以上、' +
    'ここで差が出る時点で「言語は関係ない」は崩れる。',
);
out.push(
  '- **/db/light（点SELECT＝実APIの大半）**: Rust が **約 2.4〜5.3× のスループット**で、しかも' +
    '**同時数が上がるほど差が開く**。Node は p99 が桁で悪化（c=256 で Rust 5ms 台 vs Node 30ms 台）、' +
    'CPU も 1req あたり約 5× 食う。**現実のAPIが住むこの領域で主張は完全に崩れる。**',
);
out.push(
  '- **/db/heavy（重い集約＝DB律速）**: ここだけスループットが収束する（飽和時はほぼ同じ req/s）。' +
    '**＝同僚の主張が成り立つ唯一の領域。** ただしそれでも Rust は **CPU を約 3× 節約**し、テール（p99）も安定。' +
    '「同じ速度」でも「同じコスト・同じ安定性」ではない。',
);
out.push('');
out.push('### 全コアモードの注意点（正直な但し書き）');
out.push('');
out.push(
  '全コアの **/db/light で Node が Rust を上回る**（c≥32 で Node ~110〜120k vs Rust ~66〜77k）。' +
    'これは言語の地力ではなく**ドライバ差**: postgres.js は1接続で複数クエリを**パイプライン**するが、' +
    'sqlx はしない（1接続=1クエリ）。',
);
out.push(
  '**追試①（pool=16→64, `REPORT-pool64.md`）**: pool を増やせば Rust が追い抜くと読んだが**外れた**。' +
    'Rust は ~66k→~77k（+17%程度）で頭打ち、Node(~120k)には届かず。Rust で並ぶには pool 増ではなく' +
    '実際のパイプライン化が要る、と判明。',
);
out.push(
  '**追試②＝決着（`REPORT-pipe.md`）**: Rust 側を tokio-postgres でパイプライン化（`/db/light_pipe`）したら、' +
    '同条件(pool=64/multi)で Rust **~135k rps**（sqlx の 1.75×）に到達し、**Node(~121k)を 1.1〜1.5× 上回った**。' +
    'p99 も最良（c256: Rust pipe 2.4ms vs Node 5.3ms）。' +
    '→ 多コア点SELECTの差は「言語」でなく「**ドライバがパイプラインするか**」だったと確定。同じ手札を持たせれば Rust が勝つ。',
);
out.push(
  '※ 全コアモードは oha+PG とコアを奪い合うため「方向性」の数字。**1コア対1コアが最も公平**で、そこは Rust の圧勝。',
);
out.push('');
out.push('### まとめ');
out.push('');
out.push('- DBが**本当に**重い（CPUを使い切る）ときだけ、スループットは収束する。');
out.push('- 実APIの大半（軽い点取得・DBなし）では **Rust が 2〜5× 速く、CPUは数分の1、テールは桁で安定**（1コア比較）。');
out.push('- 収束する領域でも **CPU効率と p99 は別物**で Rust が勝つ → クラウド費用と SLO に直結。');
out.push(
  '- **「例外」も解消済み**: 多コア × 点SELECT で Node が先行したのはドライバのパイプライン差。Rust を tokio-postgres で' +
    'パイプライン化したら ~135k rps で Node(~121k)を抜き返し、p99 も最良（`REPORT-pipe.md`）。= 言語でなくドライバの差と確定。',
);
out.push(
  '- 結局「速度は変わらない」はほぼ全領域で崩れる（＝差は出る）。そして**適切なドライバを使えば Rust が一貫して速い**' +
    '（1コア比較・/ping・heavyのCPU/テール・多コア点SELECTのパイプライン版、すべて Rust 優位）。',
);
out.push('');

// エラー混入の警告(successRate < 1)。
const errs = [];
for (const [key, cell] of Object.entries(cells)) {
  for (const lang of ['rust', 'node']) {
    const s = cell[lang]?.success;
    if (s != null && s < 1) errs.push(`${lang} ${key}: successRate=${(s * 100).toFixed(1)}%`);
  }
}
if (errs.length) {
  out.push('## ⚠️ エラー混入（successRate < 100%）');
  out.push('');
  for (const e of errs) out.push(`- ${e}`);
  out.push('');
}

const text = out.join('\n') + '\n';
writeFileSync(reportOut, text);
console.log(text);
console.log(`→ ${reportOut} を更新しました`);
