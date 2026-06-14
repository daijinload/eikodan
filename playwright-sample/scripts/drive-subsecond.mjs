// subsecond-demo（http://localhost:8080）を Playwright で操作し、動画(webm)に録画するスクリプト。
// 事前に別ターミナルで `cd ../subsecond-demo && dx serve --port 8080` を起動しておくこと。
import { chromium, expect } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import fs from 'node:fs';

const URL = process.env.TARGET_URL ?? 'http://localhost:8080';
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.resolve(__dirname, '..', 'videos');
const rawDir = path.join(outDir, '.raw');
const outFile = path.join(outDir, 'subsecond-demo.webm');
const shotFile = path.join(outDir, 'subsecond-final.png');

fs.mkdirSync(rawDir, { recursive: true });

const size = { width: 1000, height: 700 };
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({
  viewport: size,
  recordVideo: { dir: rawDir, size },
});
const page = await context.newPage();
const video = page.video();
const pause = (ms = 600) => page.waitForTimeout(ms); // 録画を見やすくする間

await page.goto(URL, { waitUntil: 'networkidle' });

// WASM が DOM を描画し終えるまで待つ
await page.getByRole('heading', { name: 'subsecond hot-patch demo' }).waitFor();
const countP = page.locator('p', { hasText: /count:/ }); // "count: N" の段落
const plus = page.getByRole('button', { name: '+1' });
const minus = page.getByRole('button', { name: '-1' });

await expect(countP).toHaveText('count: 0');
await pause(900);

// +1 を5回
for (let i = 1; i <= 5; i++) {
  await plus.click();
  await expect(countP).toHaveText(`count: ${i}`);
  await pause(500);
}

// -1 を2回（5 → 3）
for (let i = 4; i >= 3; i--) {
  await minus.click();
  await expect(countP).toHaveText(`count: ${i}`);
  await pause(500);
}

await expect(countP).toHaveText('count: 3');
await pause(1000);

await page.screenshot({ path: shotFile });
console.log('final:', await countP.innerText());

await context.close(); // ここで動画の書き出しが確定する
await video.saveAs(outFile);
await browser.close();

fs.rmSync(rawDir, { recursive: true, force: true });
console.log('video :', outFile);
console.log('shot  :', shotFile);
