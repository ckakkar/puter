import { expect, test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { readFile } from 'node:fs/promises';
import type { Page } from '@playwright/test';
const ready = async (page: Page) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Pause simulation' })).toBeEnabled();
};
const pause = async (page: Page) => {
  await page.getByRole('button', { name: 'Pause simulation' }).click();
};
test('video capture produces a real media file when supported', async ({ page }) => {
  await ready(page);
  const supported = await page.evaluate(
    () =>
      typeof MediaRecorder !== 'undefined' &&
      typeof HTMLCanvasElement.prototype.captureStream === 'function',
  );
  test.skip(!supported, 'This browser does not support canvas recording.');
  await page.getByRole('button', { name: 'Record video', exact: true }).click();
  await page.waitForTimeout(350);
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Stop recording', exact: true }).click();
  const download = await pending;
  const bytes = await readFile((await download.path())!);
  expect(bytes.length).toBeGreaterThan(200);
  expect(download.suggestedFilename()).toMatch(/\.(webm|mp4)$/);
});
test('new experiments exercise swept impacts and mixed-body stress', async ({ page }) => {
  await ready(page);
  await pause(page);
  await page.getByRole('button', { name: /High-speed lab/ }).click();
  const launch = await snapshot(page);
  expect(launch.bodies.filter((b: { vx: number }) => b.vx > 30)).toHaveLength(3);
  for (let i = 0; i < 8; i++)
    await page.getByRole('button', { name: 'Advance one tick', exact: true }).click();
  const hit = await snapshot(page);
  expect(hit.bodies.some((b: { mass: number; vx: number }) => b.mass > 0 && b.vx < 0)).toBe(true);
  await page.getByRole('button', { name: /Stress chamber/ }).click();
  const stress = await snapshot(page);
  expect(stress.bodies.filter((b: { mass: number }) => b.mass > 0)).toHaveLength(160);
});
async function snapshot(page: Page) {
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export scene', exact: true }).click();
  return JSON.parse(await readFile((await (await pending).path())!, 'utf8'));
}
test('saved experiments survive reload and can be renamed and opened', async ({ page }) => {
  await ready(page);
  await pause(page);
  await page.getByRole('button', { name: 'Saved experiments', exact: true }).click();
  await page.getByLabel('Experiment name').fill('A quiet tower');
  await page.getByRole('button', { name: 'Save current scene' }).click();
  await expect(page.getByRole('heading', { name: 'A quiet tower' })).toBeVisible();
  await page.getByRole('button', { name: 'Rename A quiet tower', exact: true }).click();
  await page.getByLabel('New experiment name').fill('Tower study');
  await page.getByRole('button', { name: 'Rename', exact: true }).click();
  await page.getByRole('button', { name: 'Close library' }).click();
  await page.reload();
  await expect(page.getByRole('button', { name: 'Pause simulation' })).toBeEnabled();
  await page.getByRole('button', { name: 'Saved experiments', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Tower study' })).toBeVisible();
  await page.getByRole('button', { name: 'Open experiment' }).click();
  await expect(page.getByRole('heading', { name: 'Tower study' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Play simulation' })).toBeVisible();
});
test('a compressed shared scene loads an independent copy', async ({ page, context }) => {
  await ready(page);
  await pause(page);
  const original = await snapshot(page);
  await page.getByRole('button', { name: 'Share experiment', exact: true }).click();
  await expect(page.getByLabel('Scene link')).not.toHaveValue('');
  const link = await page.getByLabel('Scene link').inputValue();
  const recipient = await context.newPage();
  await recipient.goto(link);
  await expect(recipient.getByRole('button', { name: 'Play simulation' })).toBeEnabled();
  const received = await snapshot(recipient);
  expect(received.bodies).toEqual(original.bodies);
  expect(received.joints).toEqual(original.joints);
  await recipient.close();
});
test('recovery preserves work across reload without replacing the current scene silently', async ({
  page,
}) => {
  await ready(page);
  await pause(page);
  await page.getByRole('button', { name: 'Inspect circle 23', exact: true }).click();
  await page.getByRole('button', { name: 'Delete selected body' }).click();
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem('poltergeist:recovery:v1')))
    .not.toBeNull();
  await page.reload();
  await expect(page.getByRole('button', { name: 'Restore session' })).toBeVisible();
  await page.getByRole('button', { name: 'Restore session' }).click();
  expect((await snapshot(page)).bodies).toHaveLength(24);
});
test('material presets, keyboard placement, precise motion, search, and world undo', async ({
  page,
}) => {
  await ready(page);
  await pause(page);
  await page.getByRole('button', { name: /Empty chamber/ }).click();
  await page.getByRole('button', { name: 'Add circle', exact: true }).click();
  await page.getByLabel('Placement material').selectOption('rubber');
  const canvas = page.getByLabel(/^Interactive physics chamber/);
  await canvas.focus();
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('Enter');
  const placed = await snapshot(page);
  expect(placed.bodies).toHaveLength(4);
  expect(placed.bodies.at(-1).restitution).toBeCloseTo(0.8);
  await page.getByLabel('Body material preset').selectOption('ice');
  expect((await snapshot(page)).bodies.at(-1).friction).toBeCloseTo(0.02);
  await page.getByText('Transform & motion', { exact: true }).click();
  await page.getByLabel('Velocity X value', { exact: true }).fill('7');
  await page.getByLabel('Velocity X value', { exact: true }).press('Enter');
  expect((await snapshot(page)).bodies.at(-1).vx).toBeCloseTo(7);
  await page.getByLabel('Search objects').fill('circle');
  await expect(page.locator('.object-button')).toHaveCount(1);
  await page.getByLabel('Search objects').fill('');
  await page.getByLabel('Gravity value', { exact: true }).fill('0');
  await page.getByLabel('Gravity value', { exact: true }).press('Enter');
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  expect((await snapshot(page)).gravity).toBeCloseTo(9.81);
});
test('measurement, commands, image, and energy exports work', async ({ page }, info) => {
  await ready(page);
  await page.waitForTimeout(250);
  await pause(page);
  await page.getByRole('button', { name: 'Measure distance', exact: true }).click();
  const canvas = page.getByLabel(/^Interactive physics chamber/);
  await canvas.scrollIntoViewIfNeeded();
  const rect = (await canvas.boundingBox())!;
  await page.mouse.move(rect.x + rect.width * 0.6, rect.y + rect.height * 0.3);
  await page.mouse.down();
  await page.mouse.move(rect.x + rect.width * 0.8, rect.y + rect.height * 0.4, { steps: 5 });
  await page.mouse.up();
  await expect(page.getByText('1 measurement', { exact: true })).toBeVisible();
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export energy CSV' }).click();
  const download = await pending;
  const csv = await readFile((await download.path())!, 'utf8');
  expect(csv).toContain('tick,time_s,kinetic_J,potential_J,total_J');
  expect(csv.split('\n').length).toBeGreaterThan(2);
  await page.getByRole('button', { name: 'Search commands', exact: true }).click();
  await page
    .getByRole('combobox', { name: 'Search commands', exact: true })
    .fill('Clear measurements');
  await page.getByRole('combobox', { name: 'Search commands', exact: true }).press('Enter');
  await expect(page.getByText('0 measurements', { exact: true })).toBeVisible();
  const image = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Save image' }).click();
  expect((await image).suggestedFilename()).toMatch(/\.png$/);
  await page.screenshot({ path: info.outputPath('workspace.png'), fullPage: true });
});
test('the app and library have no serious accessibility violations', async ({ page }) => {
  await ready(page);
  await pause(page);
  const critical = async () =>
    (await new AxeBuilder({ page }).analyze()).violations.filter((v) =>
      ['critical', 'serious'].includes(v.impact ?? ''),
    );
  expect(await critical()).toEqual([]);
  await page.getByRole('button', { name: 'Saved experiments', exact: true }).click();
  expect(await critical()).toEqual([]);
  await page.getByRole('button', { name: 'Close library' }).click();
  await page.getByRole('button', { name: 'Search commands', exact: true }).click();
  expect(await critical()).toEqual([]);
});
