import { test, expect } from '@playwright/test';

/**
 * カウンター機能の**仕様テスト**（ブラウザ駆動）。契約の正本は隣の spec_counter.md。
 *
 * 各テストは md の契約 ID（B-1 / B-2）に 1:1 で対応する。**期待値を変えるときは先に md を変える**
 * ── md が動かずここだけ動く diff は「バグ修正でテストを黙らせた」典型的な事故パターンなので、
 * 規約上の異常として扱う（../../docs/testing.md）。
 *
 * 何を見ているか: 同じ生成型 CounterView の1インスタンスが「画面の数字 / <!-- view-data -->
 * 埋め込みJSON / Connect API」の3経路に流れる。データ取得は1回・出口は複数なので3つは常に一致する
 * はず ── それを実ブラウザ + 実 htmx swap + 実 API で突き合わせる。HTML 文字列としての不変条件は
 * HTTP 層（tests-http）の担当で、ここでは見ない（担当範囲の表は md）。
 *
 * 前提: サーバが起動済み（`./run db-setup && ./run dev`。CI ならワークフローが起動）。
 *       接続先は playwright.config.ts の baseURL（BASE_URL で上書き可）。
 *       workers:1 の直列実行を前提に**厳密な +1** を要求する（md「前提条件」）。
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

/** **B-1**: トップページは3経路（画面 / 埋め込み view-data / Connect GetCount）が同じ値を指す。 */
test('B-1 トップ: 画面の数字 = 埋め込み view-data = Connect GetCount', async ({ page, request }) => {
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

/**
 * **B-2**: `+1` クリックで画面・フラグメント view-data・Connect が揃って +1 する。
 *
 * 厳密な +1 を要求できるのは workers:1 の直列実行が前提だから（HTTP 層 C-2 は単調増加）。
 * 並列化するなら**先に md の B-2 を単調増加へ緩めること**。
 */
test('B-2 +1: 画面・フラグメント view-data・Connect が揃って +1 する', async ({ page, request }) => {
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
