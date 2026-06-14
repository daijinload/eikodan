import { test, expect } from '@playwright/test';

/**
 * ネットワーク不要のオフライン動作確認用サンプル。
 * setContent で組み立てた DOM に対してアサーションする。
 */

test('見出しがレンダリングされる', async ({ page }) => {
  await page.setContent('<h1>Hello Playwright</h1>');
  await expect(page.locator('h1')).toHaveText('Hello Playwright');
});

test('ボタンクリックでテキストが更新される', async ({ page }) => {
  await page.setContent(`
    <button id="btn">click me</button>
    <p id="out">before</p>
    <script>
      document.getElementById('btn').addEventListener('click', () => {
        document.getElementById('out').textContent = 'after';
      });
    </script>
  `);

  await expect(page.locator('#out')).toHaveText('before');
  await page.click('#btn');
  await expect(page.locator('#out')).toHaveText('after');
});
