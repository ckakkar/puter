import { test, expect } from '@playwright/test';
test('production shell caches the app and real WASM engine for offline reload', async ({
  page,
  context,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Pause simulation' })).toBeEnabled();
  await page.waitForFunction(() => Boolean(navigator.serviceWorker.controller));
  await expect(page.locator('.status-version')).toContainText('OFFLINE READY');
  await context.setOffline(true);
  await page.reload();
  await expect(page.getByRole('button', { name: 'Pause simulation' })).toBeEnabled();
  await page.getByRole('button', { name: 'Pause simulation' }).click();
  const before = await page.getByTestId('elapsed').textContent();
  await page.getByRole('button', { name: 'Advance one tick' }).click();
  await expect(page.getByTestId('elapsed')).not.toHaveText(before!);
  expect(errors).toEqual([]);
  await context.setOffline(false);
});
