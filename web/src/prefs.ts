import { jointKinds, MAX_SIDES, MIN_SIDES } from './engine';
import type { JointKind } from './engine';
import type { Overlays } from './renderer';

/** Per-viewer conveniences remembered between visits. The world itself is not stored. */
export interface Prefs {
  overlays: Overlays;
  speed: number;
  placeStatic: boolean;
  sides: number;
  jointKind: JointKind;
}
const KEY = 'poltergeist:prefs:v1';
export const speeds = [0.1, 0.25, 0.5, 1, 2];

export function loadPrefs(defaults: Prefs): Prefs {
  let raw: Record<string, unknown> = {};
  try {
    raw = JSON.parse(localStorage.getItem(KEY) ?? '{}') ?? {};
  } catch {
    return defaults;
  }
  const overlays = { ...defaults.overlays };
  if (raw.overlays && typeof raw.overlays === 'object')
    for (const key of Object.keys(overlays) as (keyof Overlays)[]) {
      const value = (raw.overlays as Record<string, unknown>)[key];
      if (typeof value === 'boolean') overlays[key] = value;
    }
  return {
    overlays,
    speed: speeds.includes(raw.speed as number) ? (raw.speed as number) : defaults.speed,
    placeStatic: typeof raw.placeStatic === 'boolean' ? raw.placeStatic : defaults.placeStatic,
    sides:
      Number.isInteger(raw.sides) &&
      (raw.sides as number) >= MIN_SIDES &&
      (raw.sides as number) <= MAX_SIDES
        ? (raw.sides as number)
        : defaults.sides,
    jointKind: jointKinds.includes(raw.jointKind as JointKind)
      ? (raw.jointKind as JointKind)
      : defaults.jointKind,
  };
}
export function savePrefs(prefs: Prefs) {
  try {
    localStorage.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // Storage can be unavailable (private windows, blocked site data); preferences are optional.
  }
}
