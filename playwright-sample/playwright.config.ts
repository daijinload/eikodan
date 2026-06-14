import { defineConfig, devices } from '@playwright/test';

/**
 * Microsoft Playwright の設定。
 * 詳細: https://playwright.dev/docs/test-configuration
 */
export default defineConfig({
  testDir: './tests',
  // 各テストファイルを並列実行する
  fullyParallel: true,
  // CI で test.only が残っていたら失敗させる
  forbidOnly: !!process.env.CI,
  // CI のみリトライする
  retries: process.env.CI ? 2 : 0,
  // ローカルは並列、CI はワーカー1で安定実行
  workers: process.env.CI ? 1 : undefined,
  // 失敗時の調査用に HTML レポートを出力
  reporter: 'html',
  use: {
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
