import { test, expect } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import type { Page } from '@playwright/test';

interface SceneBody {
  id: number;
  kind: number;
  x: number;
  y: number;
  angle: number;
  a: number;
  b: number;
  mass: number;
  vx: number;
  vy: number;
  omega: number;
}
interface SceneJoint {
  id: number;
  kind: string;
  a: number;
  b: number;
  anchorA: [number, number];
  anchorB: [number, number];
  length: number;
}

async function exportScene(page: Page) {
  const pending = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export scene' }).click();
  const download = await pending;
  const path = await download.path();
  return JSON.parse(await readFile(path!, 'utf8'));
}
const chamber = (page: Page) => page.getByLabel(/^Interactive physics chamber/);
/** Page coordinates of a world point under the default camera. */
async function toScreen(page: Page, x: number, y: number) {
  const canvas = chamber(page);
  await canvas.scrollIntoViewIfNeeded();
  const b = (await canvas.boundingBox())!;
  const scale = Math.min(b.width / 20, b.height / 9.6);
  return {
    x: b.x + b.width / 2 + x * scale,
    y: b.y + b.height / 2 + 3.8 * scale - y * scale,
    scale,
  };
}
const pause = (page: Page) => page.getByRole('button', { name: 'Pause simulation' }).click();
const loadScene = (page: Page, name: string) =>
  page.getByRole('button', { name: new RegExp(name) }).click();
const bodyById = (scene: { bodies: SceneBody[] }, id: number) =>
  scene.bodies.find((b) => b.id === id)!;
async function importJson(page: Page, doc: unknown) {
  await page.getByLabel('Choose scene file').setInputFiles({
    name: 'scene.json',
    mimeType: 'application/json',
    buffer: Buffer.from(JSON.stringify(doc)),
  });
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
  await pause(page);
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
  await pause(page);
  await page.getByRole('button', { name: 'Add box' }).click();
  const canvas = chamber(page);
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
  await loadScene(page, 'Pendulum garden');
  await pause(page);
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
  await loadScene(page, 'Tower collapse');
  await importJson(page, original);
  await expect(page.getByRole('heading', { name: 'Custom experiment' })).toBeVisible();
  const restored = await exportScene(page);
  expect(restored.bodies).toEqual(original.bodies);
  expect(restored.joints).toEqual(original.joints.map((j: object) => ({ ...j, impulse: 0 })));
});

test('invalid imports leave the running world untouched', async ({ page }) => {
  await pause(page);
  const original = await exportScene(page);
  const bad = {
    ...original,
    bodies: [...original.bodies, { ...original.bodies[0], a: -1, id: 999 }],
  };
  await importJson(page, bad);
  await expect(page.getByRole('status')).toContainText('invalid body');
  expect((await exportScene(page)).bodies).toEqual(original.bodies);
});

test('impulse, spring dragging, and joint tools change the physical world', async ({ page }) => {
  await loadScene(page, 'Bounce chamber');
  await pause(page);
  const original = await exportScene(page);
  const first = original.bodies.find((b: SceneBody) => b.kind === 0 && b.mass > 0);
  const second = original.bodies.find(
    (b: SceneBody) => b.kind === 0 && b.mass > 0 && b.id !== first.id,
  );
  await page.getByRole('button', { name: 'Apply impulse', exact: true }).click();
  const start = await toScreen(page, first.x, first.y);
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(start.x + start.scale, start.y, { steps: 5 });
  await page.mouse.up();
  const pushed = bodyById(await exportScene(page), first.id);
  expect(pushed.vx - first.vx).toBeCloseTo(5 / first.mass, 3);
  await page.getByRole('button', { name: 'Connect bodies', exact: true }).click();
  const ca = await toScreen(page, first.x, first.y);
  await page.mouse.click(ca.x, ca.y);
  const cb = await toScreen(page, second.x, second.y);
  await page.mouse.click(cb.x, cb.y);
  expect((await exportScene(page)).joints).toHaveLength(1);
  await page.getByRole('button', { name: 'Select & drag', exact: true }).click();
  await page.getByRole('button', { name: 'Play simulation' }).click();
  const drag = await toScreen(page, first.x, first.y);
  await page.mouse.move(drag.x, drag.y);
  await page.mouse.down();
  await page.mouse.move(drag.x - drag.scale, drag.y - drag.scale, { steps: 5 });
  await page.waitForTimeout(200);
  await page.mouse.up();
  await pause(page);
  const moved = bodyById(await exportScene(page), first.id);
  expect(Math.hypot(moved.x - first.x, moved.y - first.y)).toBeGreaterThan(0.05);
});

test('viewport, overlays, and keyboard controls remain usable', async ({ page }, testInfo) => {
  await page.getByLabel('Contact impulses').check();
  await page.getByLabel('Velocity vectors').check();
  await page.getByRole('button', { name: 'Inspect circle 23', exact: true }).click();
  await chamber(page).click({ position: { x: 5, y: 5 } });
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

test('keyboard shortcuts keep working after clicking a button', async ({ page }) => {
  await page.getByRole('button', { name: 'Add box', exact: true }).click();
  await page.keyboard.press('v');
  await expect(page.getByRole('button', { name: 'Select & drag', exact: true })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  // Space must pause rather than re-click the focused Reset button.
  await page.getByRole('button', { name: 'Reset experiment' }).click();
  await expect
    .poll(async () => Number((await page.getByTestId('elapsed').textContent())?.replace('s', '')))
    .toBeGreaterThan(0.2);
  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: 'Play simulation' })).toBeVisible();
  const elapsed = Number((await page.getByTestId('elapsed').textContent())?.replace('s', ''));
  expect(elapsed).toBeGreaterThan(0.2);
  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: 'Pause simulation' })).toBeVisible();
});

test('impulses push from the point on the body, even after it moves', async ({ page }) => {
  await loadScene(page, 'Empty chamber');
  await pause(page);
  await page.getByRole('button', { name: 'Add circle', exact: true }).click();
  const spot = await toScreen(page, 0, 6);
  await page.mouse.click(spot.x, spot.y);
  const placed = (await exportScene(page)).bodies.at(-1) as SceneBody;
  await page.getByRole('button', { name: 'Apply impulse', exact: true }).click();
  const press = await toScreen(page, 0, 6);
  await page.mouse.move(press.x, press.y);
  await page.mouse.down();
  // Let the body fall while the impulse is being aimed. Pressed at its center, the push
  // must stay at its center: no spin, whatever direction the arrow points.
  await page.keyboard.press('Space');
  await page.waitForTimeout(400);
  await page.keyboard.press('Space');
  const release = await toScreen(page, 0.4, 6);
  await page.mouse.move(release.x, release.y, { steps: 3 });
  await page.mouse.up();
  const pushed = bodyById(await exportScene(page), placed.id);
  expect(pushed.y).toBeLessThan(5.7);
  expect(Math.abs(pushed.omega)).toBeLessThan(0.2);
  expect(pushed.vx).toBeCloseTo((0.4 * 5) / placed.mass, 1);
});

test('joints anchor at the clicked points', async ({ page }) => {
  await loadScene(page, 'Bounce chamber');
  await pause(page);
  const scene = await exportScene(page);
  const [first, second] = scene.bodies.filter((b: SceneBody) => b.kind === 0 && b.mass > 0);
  await page.getByRole('button', { name: 'Connect bodies', exact: true }).click();
  await page.getByRole('button', { name: 'Spring', exact: true }).click();
  const a = await toScreen(page, first.x + 0.15, first.y);
  await page.mouse.click(a.x, a.y);
  const b = await toScreen(page, second.x, second.y);
  await page.mouse.click(b.x, b.y);
  const [joint] = (await exportScene(page)).joints as SceneJoint[];
  expect(joint.kind).toBe('spring');
  expect(Math.hypot(...joint.anchorA)).toBeCloseTo(0.15, 1);
  expect(Math.hypot(...joint.anchorB)).toBeLessThan(0.02);
});

test('undo and redo restore edits, including deletions', async ({ page }) => {
  await pause(page);
  await expect(page.getByRole('button', { name: 'Undo' })).toBeDisabled();
  await page.getByRole('button', { name: 'Add box', exact: true }).click();
  const spot = await toScreen(page, 4, 5);
  await page.mouse.click(spot.x, spot.y);
  await expect(page.getByRole('button', { name: 'Inspect box 26', exact: true })).toBeVisible();
  await page.keyboard.press('ControlOrMeta+z');
  await expect(page.getByRole('button', { name: 'Inspect box 26', exact: true })).toHaveCount(0);
  await page.keyboard.press('ControlOrMeta+Shift+z');
  await expect(page.getByRole('button', { name: 'Inspect box 26', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Inspect box 26', exact: true }).click();
  await page.keyboard.press('Delete');
  await expect(page.getByRole('button', { name: 'Inspect box 26', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Undo' }).click();
  await expect(page.getByRole('button', { name: 'Inspect box 26', exact: true })).toBeVisible();
  // Undoing a scene change brings the previous experiment back.
  await loadScene(page, 'Empty chamber');
  expect((await exportScene(page)).bodies).toHaveLength(3);
  await page.getByRole('button', { name: 'Undo' }).click();
  await expect(page.getByRole('heading', { name: 'Tower collapse' })).toBeVisible();
  expect((await exportScene(page)).bodies).toHaveLength(26);
});

test('the timeline rewinds and replays recorded motion', async ({ page }) => {
  await expect
    .poll(async () => Number((await page.getByTestId('elapsed').textContent())?.replace('s', '')))
    .toBeGreaterThan(1);
  await pause(page);
  const tick = async () => Number((await page.locator('.tick-label').textContent())!.slice(5));
  const now = await tick();
  for (let i = 0; i < 30; i++) await page.keyboard.press('ArrowLeft');
  await expect.poll(tick).toBe(now - 30);
  await expect(page.locator('.status')).toContainText('REPLAY');
  for (let i = 0; i < 30; i++) await page.keyboard.press('ArrowRight');
  await expect.poll(tick).toBe(now);
  await expect(page.locator('.status')).toContainText('PAUSED');
  await page.getByRole('slider', { name: 'Timeline' }).evaluate((el) => {
    (el as HTMLInputElement).value = '0';
    el.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await expect.poll(tick).toBeLessThan(now - 30);
  // Playing from the past discards the old future.
  await page.keyboard.press('Space');
  await page.waitForTimeout(300);
  await pause(page);
  await expect(page.locator('.timeline-label')).toHaveText('LIVE');
});

test('polygons, drag-to-size, and static placement', async ({ page }) => {
  await loadScene(page, 'Empty chamber');
  await pause(page);
  await page.getByRole('button', { name: 'Add polygon', exact: true }).click();
  await page.getByRole('button', { name: 'More sides' }).click();
  const spot = await toScreen(page, 0, 4);
  await page.mouse.click(spot.x, spot.y);
  let bodies = (await exportScene(page)).bodies as SceneBody[];
  expect(bodies.at(-1)).toMatchObject({ kind: 2, b: 6 });
  await page.getByRole('button', { name: 'Add box', exact: true }).click();
  const from = await toScreen(page, -4, 6);
  const to = await toScreen(page, -2, 5);
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move(to.x, to.y, { steps: 5 });
  await page.mouse.up();
  bodies = (await exportScene(page)).bodies;
  const box = bodies.at(-1)!;
  expect(box.kind).toBe(1);
  expect(box.a).toBeCloseTo(1, 1);
  expect(box.b).toBeCloseTo(0.5, 1);
  expect(box.x).toBeCloseTo(-3, 1);
  expect(box.y).toBeCloseTo(5.5, 1);
  await page.getByRole('button', { name: 'Static', exact: true }).click();
  const ledge = await toScreen(page, 3, 2);
  await page.mouse.click(ledge.x, ledge.y);
  expect((await exportScene(page)).bodies.at(-1).mass).toBe(0);
});

test('pins hold bodies to the world while rotation stays free', async ({ page }) => {
  await loadScene(page, 'Empty chamber');
  await pause(page);
  await page.getByRole('button', { name: 'Add box', exact: true }).click();
  const spot = await toScreen(page, 0, 5);
  await page.mouse.click(spot.x, spot.y);
  await page.getByRole('button', { name: 'Connect bodies', exact: true }).click();
  await page.getByRole('button', { name: 'Pin', exact: true }).click();
  const pin = await toScreen(page, 0.4, 5);
  await page.mouse.click(pin.x, pin.y);
  await page.getByRole('button', { name: 'Play simulation' }).click();
  await page.waitForTimeout(800);
  await pause(page);
  const scene = await exportScene(page);
  const [joint] = scene.joints as SceneJoint[];
  expect(joint).toMatchObject({ kind: 'pin', b: 0 });
  const body = bodyById(scene, joint.a);
  const [lx, ly] = joint.anchorA;
  const ax = body.x + lx * Math.cos(body.angle) - ly * Math.sin(body.angle);
  const ay = body.y + lx * Math.sin(body.angle) + ly * Math.cos(body.angle);
  expect(Math.hypot(ax - joint.anchorB[0], ay - joint.anchorB[1])).toBeLessThan(0.05);
  expect(Math.abs(body.angle)).toBeGreaterThan(0.2);
});

test('paused bodies can be moved directly, and the move can be undone', async ({ page }) => {
  await pause(page);
  const before = bodyById(await exportScene(page), 23);
  const from = await toScreen(page, before.x, before.y);
  const to = await toScreen(page, before.x + 1, before.y + 2);
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move(to.x, to.y, { steps: 6 });
  await page.mouse.up();
  const after = bodyById(await exportScene(page), 23);
  expect(after.x - before.x).toBeCloseTo(1, 1);
  expect(after.y - before.y).toBeCloseTo(2, 1);
  await page.keyboard.press('ControlOrMeta+z');
  const restored = bodyById(await exportScene(page), 23);
  expect(restored.x).toBeCloseTo(before.x, 4);
  expect(restored.y).toBeCloseTo(before.y, 4);
});

test('zooming keeps the point under the cursor fixed', async ({ page }) => {
  await loadScene(page, 'Empty chamber');
  await pause(page);
  const zoom = page.getByTestId('zoom');
  await expect(zoom).toHaveText('100%');
  await page.getByRole('button', { name: 'Add circle', exact: true }).click();
  const target = await toScreen(page, 2, 4);
  await page.mouse.move(target.x, target.y);
  await page.mouse.wheel(0, -400);
  await expect(zoom).not.toHaveText('100%');
  // A body placed at the same screen point lands at the same world point.
  await page.mouse.click(target.x, target.y);
  const placed = (await exportScene(page)).bodies.at(-1);
  expect(placed.x).toBeCloseTo(2, 1);
  expect(placed.y).toBeCloseTo(4, 1);
  await page.getByRole('button', { name: 'Reset view' }).click();
  await expect(zoom).toHaveText('100%');
  await page.keyboard.press('Escape');
  await page.keyboard.press('+');
  await expect(zoom).toHaveText('125%');
});

test('version 1 scenes still import', async ({ page }) => {
  const body = (
    id: number,
    kind: number,
    x: number,
    y: number,
    a: number,
    b: number,
    mass: number,
  ) => ({
    id,
    kind,
    x,
    y,
    angle: 0,
    a,
    b,
    mass,
    friction: 0.5,
    restitution: 0.1,
    vx: 0,
    vy: 0,
    omega: 0,
  });
  await importJson(page, {
    format: 'poltergeist-scene',
    version: 1,
    gravity: 9.81,
    iterations: 12,
    bodies: [body(1, 1, 0, -0.3, 9, 0.3, 0), body(7, 0, 1, 3, 0.4, 0.4, 2)],
    joints: [{ ax: 1, ay: 3, bx: 1, by: 6, length: 3, impulse: 0, a: 7, b: 1 }],
  });
  await expect(page.getByRole('heading', { name: 'Custom experiment' })).toBeVisible();
  const scene = await exportScene(page);
  expect(scene.version).toBe(2);
  expect(scene.bodies.map((b: SceneBody) => b.id)).toEqual([1, 7]);
  expect(scene.joints).toHaveLength(1);
  expect(scene.joints[0]).toMatchObject({ kind: 'rod', a: 7, b: 1, length: 3 });
});

test('overlay preferences survive a reload', async ({ page }) => {
  await page.getByLabel('Motion trails').check();
  await page.reload();
  await expect(page.getByRole('button', { name: 'Pause simulation' })).toBeEnabled();
  await expect(page.getByLabel('Motion trails')).toBeChecked();
});
