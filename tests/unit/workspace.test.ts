import { beforeEach, describe, expect, it, vi } from 'vitest';
import { parseScene, emptyFrame } from '../../web/src/engine';
import type { SceneDoc } from '../../web/src/engine';
import {
  loadLibrary,
  saveExperiment,
  renameExperiment,
  removeExperiment,
  loadRecovery,
  saveRecovery,
} from '../../web/src/library';
import { createSceneLink, readSceneLink } from '../../web/src/share';
import { interpolate } from '../../web/src/presentation';
import { UndoStack } from '../../web/src/undo';
import { homeCamera, viewFor, screenToWorld, worldToScreen, zoomAt } from '../../web/src/view';

const doc = (): SceneDoc => ({
  format: 'poltergeist-scene',
  version: 2,
  gravity: 9.81,
  iterations: 12,
  sleeping: true,
  bodies: [
    {
      id: 1,
      kind: 0,
      x: 0,
      y: 3,
      angle: 0,
      a: 0.4,
      b: 0.4,
      mass: 1,
      friction: 0.5,
      restitution: 0.2,
      vx: 0,
      vy: 0,
      omega: 0,
    },
  ],
  joints: [],
});
let data: Map<string, string>;
beforeEach(() => {
  data = new Map();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => data.set(k, v),
    removeItem: (k: string) => data.delete(k),
  });
  vi.stubGlobal('window', globalThis);
  vi.stubGlobal('location', { origin: 'https://lab.example', pathname: '/' });
});
describe('scene boundary', () => {
  it.each([null, [], {}, { format: 'poltergeist-scene', version: 999 }])(
    'rejects an invalid envelope',
    (raw) => expect(() => parseScene(raw)).toThrow(),
  );
  it('rejects IDs that lose precision in the WASM render ABI', () => {
    const s = doc();
    s.bodies[0].id = 16_777_217;
    expect(() => parseScene(s)).toThrow('invalid body');
  });
  it('rejects non-representable angles and duplicate IDs', () => {
    const s = doc();
    s.bodies[0].angle = 1e100;
    expect(() => parseScene(s)).toThrow();
    s.bodies[0].angle = 0;
    s.bodies.push({ ...s.bodies[0] });
    expect(() => parseScene(s)).toThrow();
  });
  it('normalizes legacy rods to body-local anchors', () => {
    const s = {
      ...doc(),
      version: 1,
      bodies: [...doc().bodies, { ...doc().bodies[0], id: 2, x: 2 }],
      joints: [{ a: 1, b: 2, ax: 0, ay: 3, bx: 2, by: 3, length: 2 }],
    };
    expect(parseScene(s).joints[0]).toMatchObject({
      kind: 'rod',
      anchorA: [0, 0],
      anchorB: [0, 0],
    });
  });
});
describe('experiment storage', () => {
  it('saves, updates by name, and keeps unique IDs', () => {
    const first = saveExperiment('Pendulum', doc());
    const next = saveExperiment('pendulum', { ...doc(), gravity: 2 });
    expect(next).toHaveLength(1);
    expect(next[0].id).toBe(first[0].id);
    expect(loadLibrary()[0].scene.gravity).toBe(2);
  });
  it('renames without colliding with another experiment', () => {
    const a = saveExperiment('A', doc())[0];
    saveExperiment('B', doc());
    expect(() => renameExperiment(a.id, 'b')).toThrow();
    expect(renameExperiment(a.id, 'C').some((e) => e.name === 'C')).toBe(true);
    expect(removeExperiment(a.id)).toHaveLength(1);
  });
  it('does not replace a saved library when the write fails', () => {
    saveExperiment('A', doc());
    const previous = [...data.values()];
    vi.stubGlobal('localStorage', {
      getItem: (k: string) => data.get(k) ?? null,
      setItem: () => {
        throw new Error('quota');
      },
    });
    expect(() => saveExperiment('B', doc())).toThrow('storage');
    expect([...data.values()]).toEqual(previous);
  });
  it('ignores corrupt data without throwing into the application', () => {
    data.set('poltergeist:library:v1', 'bad');
    data.set('poltergeist:recovery:v1', 'null');
    expect(loadLibrary()).toEqual([]);
    expect(loadRecovery()).toBeNull();
  });
  it('keeps a validated recovery snapshot', () => {
    expect(saveRecovery('Saved motion', doc())).toBe(true);
    expect(loadRecovery()).toMatchObject({ name: 'Saved motion', scene: doc() });
  });
  it('refuses storage exhaustion and excessively long names', () => {
    for (let i = 0; i < 24; i++) saveExperiment(`Scene ${i}`, doc());
    expect(() => saveExperiment('Another', doc())).toThrow('24');
    expect(() => saveExperiment('x'.repeat(81), doc())).toThrow();
  });
});
describe('share links', () => {
  it('round-trips a compressed scene with a unicode title', async () => {
    const link = await createSceneLink('振り子 / pendulum', doc());
    expect(link).toMatch(/^https:\/\/lab.example\/#scene=/);
    expect(await readSceneLink(new URL(link).hash)).toEqual({
      name: '振り子 / pendulum',
      scene: doc(),
    });
  });
  it('rejects invalid encodings and broken gzip data', async () => {
    await expect(readSceneLink('#scene=@@@@')).rejects.toThrow();
    await expect(readSceneLink('#scene=AAAA')).rejects.toThrow();
    expect(await readSceneLink('#other')).toBeNull();
  });
  it('bounds decompressed data', async () => {
    const compressed = await new Response(
      new Blob(['x'.repeat(1_000_001)]).stream().pipeThrough(new CompressionStream('gzip')),
    ).arrayBuffer();
    const encoded = btoa(String.fromCharCode(...new Uint8Array(compressed)))
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=+$/, '');
    await expect(readSceneLink(`#scene=${encoded}`)).rejects.toThrow();
  });
});
describe('presentation and editing', () => {
  it('interpolates only consecutive ticks without mutating authoritative state', () => {
    const previous = { ...emptyFrame(), tick: 1, bodies: [{ ...doc().bodies[0], awake: true }] };
    const current = { ...previous, tick: 2, bodies: [{ ...previous.bodies[0], x: 2 }] };
    expect(interpolate(previous, current, 0.5).bodies[0].x).toBe(1);
    expect(current.bodies[0].x).toBe(2);
    expect(interpolate(previous, { ...current, tick: 8 }, 0.5).bodies[0].x).toBe(2);
  });
  it('undo preserves the pre-drag state, redo, and branch truncation', () => {
    const stack = new UndoStack();
    const state = (scene: string) => ({ scene, sceneIndex: 0, custom: false, initial: '' });
    stack.push(state('before'), 'Move', 'drag', 1);
    stack.push(state('during'), 'Move', 'drag', 2);
    expect(stack.undo(state('after'))?.scene).toBe('before');
    expect(stack.redo(state('before'))?.scene).toBe('after');
    stack.undo(state('after'));
    stack.push(state('before'), 'New edit');
    expect(stack.canRedo).toBe(false);
  });
  it('camera transforms invert and cursor zoom preserves world coordinates', () => {
    const camera = homeCamera(),
      view = viewFor(1000, 600, camera);
    const screen = worldToScreen(view, 3, 4);
    expect(screenToWorld(view, screen.x, screen.y)).toEqual({ x: 3, y: 4 });
    const next = viewFor(1000, 600, zoomAt(camera, view, screen.x, screen.y, 2));
    const p = screenToWorld(next, screen.x, screen.y);
    expect(p.x).toBeCloseTo(3);
    expect(p.y).toBeCloseTo(4);
  });
});
