import { defineConfig, devices } from '@playwright/test';

/**
 * lastshot ブラウザ駆動E2E の設定。
 *
 * tests-http/ と同じ「待機で解く」方針: サーバは別で起動しておき（`./run dev`、CI なら
 * ワークフローが起動）、ここはそれに対して叩くだけ。アプリ本体をビルド/同梱しないので
 * アプリを変えてもこのパッケージは無関係（= 速い・疎結合）。接続先は BASE_URL で上書き可。
 */
export default defineConfig({
  testDir: './tests',
  // 共有カウンタ（counter テーブル 1 行）を増やすので、ファイル間も直列にして取り違えを防ぐ。
  fullyParallel: false,
  // CI で test.only が残っていたら失敗させる
  forbidOnly: !!process.env.CI,
  // CI のみリトライ（サーバ起動直後のレース等を吸収）
  retries: process.env.CI ? 2 : 0,
  // 共有状態を触るので常にワーカー1で直列実行
  workers: 1,
  // 失敗時の調査用に HTML レポートを出力（CI ではログを汚さず生成のみ）
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'html',
  use: {
    // 既定は dev サーバ。CI/別ポートは BASE_URL で上書き（例: BASE_URL=http://127.0.0.1:3000）。
    baseURL: process.env.BASE_URL ?? 'http://127.0.0.1:3000',
    // リトライ1回目でトレースを取得（失敗調査用）
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
