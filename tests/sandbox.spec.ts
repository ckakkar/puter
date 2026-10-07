import { test, expect } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import type { Page } from '@playwright/test';

async function exportScene(page: Page) {
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export scene' }).click();
  const download = await pending;
  const path = await download.path();
  return JSON.parse(await readFile(path!, 'utf8'));
}
test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Pause simulation' })).toBeEnabled();
});

test('real WASM engine pauses, advances exactly one tick, and resets', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await expect
    .poll(async () => Number((await page.getByTestId('elapsed').textContent())?.replace('s', '')))
    .toBeGreaterThan(0.1);
  await page.getByRole('button', { name: 'Pause simulation' }).click();
  await page.waitForTimeout(120);
  const paused = await page.getByTestId('elapsed').textContent();
  await page.waitForTimeout(200);
  await expect(page.getByTestId('elapsed')).toHaveText(paused!);
  const tick = await page.locator('.tick-label').textContent();
  await page.getByRole('button', { name: 'Advance one tick' }).click();
  await expect(page.locator('.tick-label')).toHaveText(
    `TICK ${String(Number(tick!.replace('TICK ', '')) + 1).padStart(5, '0')}`,
  );
  await page.getByRole('button', { name: 'Reset experiment' }).click();
  await expect(page.getByTestId('elapsed')).toHaveText('0.00s');
  expect(errors).toEqual([]);
});

test('placing and deleting a body travels through the WASM boundary', async ({ page }) => {
  await page.getByRole('button', { name: 'Pause simulation' }).click();
  await page.getByRole('button', { name: 'Add box' }).click();
  const canvas = page.locator('canvas');
  const bounds = await canvas.boundingBox();
  await canvas.click({ position: { x: bounds!.width * 0.7, y: bounds!.height * 0.3 } });
  await expect(page.getByRole('button', { name: 'Inspect box 26', exact: true })).toBeVisible();
  const scene = await exportScene(page);
  expect(scene.bodies).toHaveLength(26);
  expect(scene.bodies.at(-1).kind).toBe(1);
  await page.getByRole('button', { name: 'Delete selected body' }).click();
  await expect(page.getByRole('button', { name: 'Inspect box 26', exact: true })).toHaveCount(0);
  expect((await exportScene(page)).bodies).toHaveLength(25);
});

test('material edits, joints, and snapshot import/export are preserved', async ({ page }) => {
  await page.getByRole('button', { name: /Pendulum garden/ }).click();
  await page.getByRole('button', { name: 'Pause simulation' }).click();
  await page.getByRole('button', { name: 'Inspect circle 3', exact: true }).click();
  for (const [name, value] of [
    ['Mass', '4.2'],
    ['Friction', '0.7'],
    ['Restitution', '0.6'],
  ]) {
    await page.getByRole('slider', { name: new RegExp(`^${name}`) }).evaluate((el, value) => {
      (el as HTMLInputElement).value = value;
      el.dispatchEvent(new Event('input', { bubbles: true }));
    }, value);
  }
  const original = await exportScene(page);
  expect(original.joints).toHaveLength(5);
  const body = original.bodies.find((b: { id: number }) => b.id === 3);
  expect(body.mass).toBeCloseTo(4.2);
  expect(body.friction).toBeCloseTo(0.7);
  expect(body.restitution).toBeCloseTo(0.6);
  await page.getByRole('button', { name: /Tower collapse/ }).click();
  await page
    .getByLabel('Choose scene file')
    .setInputFiles({
      name: 'scene.json',
      mimeType: 'application/json',
      buffer: Buffer.from(JSON.stringify(original)),
    });
  await expect(page.getByRole('heading', { name: 'Custom experiment' })).toBeVisible();
  const restored = await exportScene(page);
  expect(restored.bodies).toEqual(original.bodies);
  expect(restored.joints).toEqual(original.joints.map((j: object) => ({ ...j, impulse: 0 })));
});

test('invalid imports leave the running world untouched', async ({ page }) => {
  await page.getByRole('button', { name: 'Pause simulation' }).click();
  const original = await exportScene(page);
  const bad = {
    ...original,
    bodies: [...original.bodies, { ...original.bodies[0], a: -1, id: 999 }],
  };
  await page
    .getByLabel('Choose scene file')
    .setInputFiles({
      name: 'bad.json',
      mimeType: 'application/json',
      buffer: Buffer.from(JSON.stringify(bad)),
    });
  await expect(page.getByRole('status')).toContainText('invalid body');
  expect((await exportScene(page)).bodies).toEqual(original.bodies);
});

test('impulse, spring dragging, and joint tools change the physical world', async ({ page }) => {
  await page.getByRole('button', { name: /Bounce chamber/ }).click();
  await page.getByRole('button', { name: 'Pause simulation' }).click();
  const original = await exportScene(page);
  const first = original.bodies.find(
    (b: { kind: number; mass: number }) => b.kind === 0 && b.mass > 0,
  );
  const second = original.bodies.find(
    (b: { kind: number; mass: number; id: number }) =>
      b.kind === 0 && b.mass > 0 && b.id !== first.id,
  );
  const canvas = page.locator('canvas');
  async function coords(x: number, y: number) {
    await canvas.scrollIntoViewIfNeeded();
    const b = (await canvas.boundingBox())!;
    const scale = Math.min(b.width / 20, b.height / 9.6);
    return {
      x: b.x + b.width / 2 + x * scale,
      y: b.y + b.height / 2 + 3.8 * scale - y * scale,
      scale,
    };
  }
  await page.getByRole('button', { name: 'Apply impulse', exact: true }).click();
  const start = await coords(first.x, first.y);
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(start.x + start.scale, start.y, { steps: 5 });
  await page.mouse.up();
  const pushed = (await exportScene(page)).bodies.find((b: { id: number }) => b.id === first.id);
  expect(pushed.vx - first.vx).toBeCloseTo(5 / first.mass, 3);
  await page.getByRole('button', { name: 'Connect bodies', exact: true }).click();
  const ca = await coords(first.x, first.y);
  await page.mouse.click(ca.x, ca.y);
  const cb = await coords(second.x, second.y);
  await page.mouse.click(cb.x, cb.y);
  expect((await exportScene(page)).joints).toHaveLength(1);
  await page.getByRole('button', { name: 'Select & drag', exact: true }).click();
  await page.getByRole('button', { name: 'Play simulation' }).click();
  const drag = await coords(first.x, first.y);
  await page.mouse.move(drag.x, drag.y);
  await page.mouse.down();
  await page.mouse.move(drag.x - drag.scale, drag.y - drag.scale, { steps: 5 });
  await page.waitForTimeout(200);
  await page.mouse.up();
  await page.getByRole('button', { name: 'Pause simulation' }).click();
  const moved = (await exportScene(page)).bodies.find((b: { id: number }) => b.id === first.id);
  expect(Math.hypot(moved.x - first.x, moved.y - first.y)).toBeGreaterThan(0.05);
});

test('viewport, overlays, and keyboard controls remain usable', async ({ page }, testInfo) => {
  await page.getByLabel('Contact impulses').check();
  await page.getByLabel('Velocity vectors').check();
  await page.getByRole('button', { name: 'Inspect circle 23', exact: true }).click();
  await page.locator('canvas').click({ position: { x: 5, y: 5 } });
  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: 'Play simulation' })).toBeVisible();
  await page.getByRole('button', { name: 'Help & shortcuts' }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).not.toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  const viewport = await page.locator('.viewport').boundingBox();
  expect(viewport!.height).toBeLessThan(1000);
  expect(viewport!.height).toBeGreaterThan(250);
  await page.screenshot({ path: testInfo.outputPath('sandbox.png'), fullPage: true });
});
