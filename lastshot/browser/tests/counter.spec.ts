import { test, expect } from '@playwright/test';

/**
 * lastshot の核（スキーマ＝単一の真実）をブラウザ駆動で検証する。
 *
 * 同じ生成型 CounterView が「画面の数字 / 末尾の <!-- view-data --> 埋め込みJSON /
 * Connect API（GetCount・Increment）」の3経路に流れる。データ取得は1回・出口は複数なので
 * 3つは常に一致するはず ── それを実ブラウザ + 実 HTMX swap + 実 API で突き合わせる。
 *
 * 前提: サーバが起動済み（`./run db-setup && ./run dev`。CI ならワークフローが起動）。
 *       接続先は playwright.config.ts の baseURL（BASE_URL で上書き可）。
 */

// `<!-- view-data\n{json}\n-->` コメントから JSON を取り出す（webcore が埋め込む形式）。
// JSON 中の連続ハイフンは webcore 側で "- -" に退避されるが、value だけの本ビューでは現れない。
function parseViewData(html: string): { value?: number } {
  const m = html.match(/<!-- view-data\s*([\s\S]*?)\s*-->/);
  if (!m) {
    throw new Error(`view-data コメントが見つからない:\n${html.slice(0, 400)}`);
  }
  return JSON.parse(m[1]);
}

// proto3 JSON は 0 値フィールドを省略する（value=0 のとき本体は {}）。欠落は 0 とみなす。
const valueOf = (j: { value?: number }) => j.value ?? 0;

// 表示中の #count テキスト（数字）を読む。
async function shownCount(page: import('@playwright/test').Page): Promise<number> {
  const text = (await page.locator('#count').textContent()) ?? '';
  return Number(text.trim());
}

test('トップ: 画面の数字 = 埋め込み view-data = Connect GetCount', async ({ page, request }) => {
  const resp = await page.goto('/');
  expect(resp?.ok(), 'トップが 2xx で返る').toBeTruthy();

  // カウンター要素と HTMX 属性（部分更新の宣言）が描画されている
  await expect(page.locator('#count')).toBeVisible();
  const button = page.getByRole('button', { name: '+1' });
  await expect(button).toHaveAttribute('hx-post', '/increment');
  await expect(button).toHaveAttribute('hx-target', '#count');

  const shown = await shownCount(page);

  // 末尾に埋め込まれた「この画面が使ったデータ」（同じインスタンス）と一致
  const embedded = valueOf(parseViewData(await page.content()));
  expect(embedded, '画面の数字と埋め込み view-data が一致').toBe(shown);

  // Connect API GetCount も同じ値（同じ service 層・同じ DB を共有）
  const api = await request.post('/counter.v1.CounterService/GetCount', { data: {} });
  expect(api.ok(), 'GetCount が 2xx').toBeTruthy();
  expect(valueOf(await api.json()), '画面の数字と GetCount が一致').toBe(shown);
});

test('+1: 画面・フラグメント view-data・Connect が揃って +1 する', async ({ page, request }) => {
  await page.goto('/');
  const before = await shownCount(page);

  // +1 クリック → HTMX が POST /increment の応答で #count.innerHTML を差し替える。
  // 応答本体（フラグメント）を捕まえて、その先頭の view-data を読む。
  const [resp] = await Promise.all([
    page.waitForResponse(
      (r) => r.url().endsWith('/increment') && r.request().method() === 'POST',
    ),
    page.getByRole('button', { name: '+1' }).click(),
  ]);
  const fragmentValue = valueOf(parseViewData(await resp.text()));

  // DOM が +1 で更新される（HTMX swap 後の表示）
  await expect(page.locator('#count')).toHaveText(String(before + 1));
  expect(fragmentValue, 'フラグメント view-data が +1').toBe(before + 1);

  // 直後の GetCount も同じ +1（DB に永続化されている証拠）
  const api = await request.post('/counter.v1.CounterService/GetCount', { data: {} });
  expect(valueOf(await api.json()), 'クリック後の GetCount が +1').toBe(before + 1);
});
