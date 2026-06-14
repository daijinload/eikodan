// Playwright MCP ベンチ: @playwright/mcp サーバーを stdio JSON-RPC で直接駆動し、
// 1ステップ = browser_click(+1) → browser_snapshot(状態確認) の往復レイテンシと
// レスポンスのバイト数（= エージェントが読むトークン量の代理）を計測する。
//
// 使い方: node mcp-bench.mjs <url> <steps>  (出力は1行1JSON: {phase,op,ms,bytes})
import { spawn } from "node:child_process";

const URL = process.argv[2] ?? "http://localhost:8080";
const STEPS = Number(process.argv[3] ?? 30);

const srv = spawn("npx", ["@playwright/mcp@latest", "--browser", "chromium"], {
  stdio: ["pipe", "pipe", "inherit"],
});

let buf = "";
const pending = new Map();
let nextId = 1;
srv.stdout.on("data", (d) => {
  buf += d.toString();
  let nl;
  while ((nl = buf.indexOf("\n")) >= 0) {
    const line = buf.slice(0, nl);
    buf = buf.slice(nl + 1);
    if (!line.trim()) continue;
    let msg;
    try { msg = JSON.parse(line); } catch { continue; }
    if (msg.id != null && pending.has(msg.id)) {
      const { resolve } = pending.get(msg.id);
      pending.delete(msg.id);
      resolve(msg);
    }
  }
});

function rpc(method, params) {
  const id = nextId++;
  const payload = JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n";
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    srv.stdin.write(payload);
  });
}
function notify(method, params) {
  srv.stdin.write(JSON.stringify({ jsonrpc: "2.0", method, params }) + "\n");
}

const now = () => Number(process.hrtime.bigint() / 1000n) / 1000; // ms, float
function bytesOf(resp) {
  // tool 結果テキスト部の実バイト数（エージェントが受け取る本文）
  const c = resp?.result?.content ?? [];
  return c.reduce((n, p) => n + Buffer.byteLength(p.text ?? "", "utf8"), 0);
}
async function call(name, args) {
  const t0 = now();
  const resp = await rpc("tools/call", { name, arguments: args ?? {} });
  const ms = now() - t0;
  return { ms, bytes: bytesOf(resp), resp };
}

async function main() {
  await rpc("initialize", {
    protocolVersion: "2024-11-05",
    capabilities: {},
    clientInfo: { name: "mcp-bench", version: "0" },
  });
  notify("notifications/initialized", {});

  const refOfPlus = (resp) => {
    const txt = (resp?.result?.content ?? []).map((p) => p.text ?? "").join("\n");
    const m = txt.match(/button "\+1"[^\n]*\[ref=(e\d+)\]/);
    return m ? m[1] : null;
  };

  // cold: ブラウザ起動 + ナビゲート（初回外れ値として別計測）
  const nav = await call("browser_navigate", { url: URL });
  console.log(JSON.stringify({ phase: "cold", op: "navigate", ms: nav.ms, bytes: nav.bytes }));
  // Dioxus(WASM) の描画完了を待つ（直後だとサーバ描画のトーストしか出ない）
  await call("browser_wait_for", { text: "+1" });

  // ウォームの本計測: 各ステップ = snapshot(状態確認 + ref取得) → click(+1)
  // ref は同一スナップショット由来なので常に有効。
  for (let i = 0; i < STEPS; i++) {
    const snap = await call("browser_snapshot", {});
    const ref = refOfPlus(snap.resp);
    if (!ref) throw new Error("could not find +1 button ref in snapshot");
    const click = await call("browser_click", { element: "+1 button", target: ref });
    console.log(JSON.stringify({ phase: "warm", op: "snapshot", i, ms: snap.ms, bytes: snap.bytes }));
    console.log(JSON.stringify({ phase: "warm", op: "click", i, ms: click.ms, bytes: click.bytes }));
  }

  const fin = await call("browser_snapshot", {});
  const finTxt = (fin.resp?.result?.content ?? []).map((p) => p.text ?? "").join("\n");
  const finCount = (finTxt.match(/count: (\d+)/) || [])[1];
  console.error(`[mcp-bench] final count=${finCount} (expected ${STEPS})`);

  await call("browser_close", {});
  srv.stdin.end();
  srv.kill();
  process.exit(0);
}
main().catch((e) => { console.error(e); srv.kill(); process.exit(1); });
