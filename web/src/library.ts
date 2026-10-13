import { parseScene } from './engine';
import type { SceneDoc } from './engine';
const LIBRARY = 'poltergeist:library:v1';
const DRAFT = 'poltergeist:recovery:v1';
export interface Experiment {
  id: string;
  name: string;
  updatedAt: number;
  scene: SceneDoc;
}
export interface Recovery {
  name: string;
  updatedAt: number;
  scene: SceneDoc;
}
const validName = (name: unknown) =>
  typeof name === 'string' && name.trim().length > 0 && name.length <= 80;
export function loadLibrary(): Experiment[] {
  try {
    const data = JSON.parse(localStorage.getItem(LIBRARY) ?? '[]');
    if (!Array.isArray(data)) return [];
    return data
      .slice(0, 24)
      .flatMap((entry) => {
        try {
          if (
            !entry ||
            typeof entry.id !== 'string' ||
            entry.id.length > 80 ||
            !validName(entry.name) ||
            !Number.isFinite(entry.updatedAt) ||
            entry.updatedAt < 0 ||
            entry.updatedAt > 8_640_000_000_000_000
          )
            return [];
          return [
            {
              id: entry.id,
              name: entry.name,
              updatedAt: entry.updatedAt,
              scene: parseScene(entry.scene),
            },
          ];
        } catch {
          return [];
        }
      })
      .sort((a, b) => b.updatedAt - a.updatedAt);
  } catch {
    return [];
  }
}
function write(entries: Experiment[]) {
  const text = JSON.stringify(entries);
  if (text.length > 2_000_000)
    throw new Error('Your library is full. Export an experiment before removing it.');
  try {
    localStorage.setItem(LIBRARY, text);
  } catch {
    throw new Error('Browser storage is unavailable or full. Export your scene to keep it.');
  }
}
export function saveExperiment(name: string, scene: SceneDoc): Experiment[] {
  const clean = name.trim();
  if (!validName(clean)) throw new Error('Give the experiment a name (1–80 characters).');
  const entries = loadLibrary();
  const existing = entries.find((e) => e.name.toLowerCase() === clean.toLowerCase());
  if (!existing && entries.length >= 24)
    throw new Error('You can keep 24 experiments. Export or remove one first.');
  const entry: Experiment = {
    id: existing?.id ?? crypto.randomUUID(),
    name: clean,
    updatedAt: Date.now(),
    scene: parseScene(scene),
  };
  const result = [entry, ...entries.filter((e) => e.id !== entry.id)];
  write(result);
  return result;
}
export function renameExperiment(id: string, name: string): Experiment[] {
  const clean = name.trim();
  if (!validName(clean)) throw new Error('Use a name with 1–80 characters.');
  const entries = loadLibrary();
  if (entries.some((e) => e.id !== id && e.name.toLowerCase() === clean.toLowerCase()))
    throw new Error('Another experiment already has that name.');
  const next = entries.map((e) => (e.id === id ? { ...e, name: clean } : e));
  write(next);
  return next;
}
export function removeExperiment(id: string): Experiment[] {
  const next = loadLibrary().filter((e) => e.id !== id);
  write(next);
  return next;
}
export function loadRecovery(): Recovery | null {
  try {
    const raw = JSON.parse(localStorage.getItem(DRAFT) ?? 'null');
    if (
      !raw ||
      !validName(raw.name) ||
      !Number.isFinite(raw.updatedAt) ||
      raw.updatedAt < 0 ||
      raw.updatedAt > 8_640_000_000_000_000
    )
      return null;
    return { name: raw.name, updatedAt: raw.updatedAt, scene: parseScene(raw.scene) };
  } catch {
    return null;
  }
}
export function saveRecovery(name: string, scene: SceneDoc): boolean {
  try {
    localStorage.setItem(
      DRAFT,
      JSON.stringify({
        name: name.slice(0, 80) || 'Untitled experiment',
        scene,
        updatedAt: Date.now(),
      }),
    );
    return true;
  } catch {
    return false;
  }
}
export function clearRecovery() {
  try {
    localStorage.removeItem(DRAFT);
  } catch {
    /* Storage is optional. */
  }
}
