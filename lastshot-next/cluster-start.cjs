// PM2 cluster mode 用の Next.js 起動ラッパー。
//
// `pm2-runtime start node_modules/next/dist/bin/next ...` だと PM2 が argv を介して
// "ecosystem.config.cjs" のような文字列を CLI の dir 引数に混ぜてしまい、Next.js が
// .env / .next/BUILD_ID を `<file>/.env` として解決して落ちる。
// ここでは next の programmatic API を直接叩いて、ディレクトリ・ポート・ホストを
// 明示することでその混入を断つ。
//
// PM2 cluster mode は同一スクリプトを fork し、子は親が listen した同じソケットを
// 共有する。worker 1 つあたり 1 つの Next.js HTTP サーバを立てれば OK。
const next = require("next");
const { createServer } = require("http");

const port = parseInt(process.env.PORT || "3000", 10);
const hostname = process.env.HOST || "127.0.0.1";
const app = next({ dev: false, dir: __dirname, hostname, port });
const handle = app.getRequestHandler();

app.prepare().then(() => {
  createServer((req, res) => handle(req, res)).listen(port, hostname);
});
