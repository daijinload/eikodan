// PM2 cluster mode 設定。`next start` を 15 ワーカで起動して同一ポートを共有する
// (Node の cluster モジュール経由)。
//
// 用途は lastshot-bench の「Next.js を多重化したら lastshot との RPS 差は縮むか?」
// 実験。`./run prod-cluster` から pm2-runtime で読み込まれ、フォアグラウンドで動く
// (lastshot/lastshot-laravel と同じ「ターミナルに張り付く本番起動」運用)。
//
// 環境変数(./run prod-cluster が export する):
//   PORT / HOST       — listen 先
//   INSTANCES         — ワーカ数(既定 15 = 論理コア数)
//   PG_POOL_MAX       — 1 ワーカあたりの PG コネクション上限(既定 4)
//                       15 × 4 = 60 で PostgreSQL の max_connections=100 内に収める
// __dirname を cwd に固定しないと Next.js が .env / .next/BUILD_ID を
// "ecosystem.config.cjs/.env" のように解決して落ちる(PM2 既定の挙動)。
module.exports = {
  apps: [
    {
      name: "lastshot-next",
      cwd: __dirname,
      script: "./cluster-start.cjs",
      instances: parseInt(process.env.INSTANCES || "15", 10),
      exec_mode: "cluster",
      autorestart: false,
      env: {
        NODE_ENV: "production",
        PORT: process.env.PORT,
        HOST: process.env.HOST,
        PG_POOL_MAX: process.env.PG_POOL_MAX || "4",
        PGHOST: process.env.PGHOST || "/tmp",
        PGDATABASE: process.env.PGDATABASE || "lastshot",
      },
    },
  ],
};
