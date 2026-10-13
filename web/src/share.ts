import { parseScene } from './engine';
import type { SceneDoc } from './engine';
const MAX_LINK = 24_000;
/** Bounded streaming reads also reject a small compressed decompression bomb. */
async function bounded(stream: ReadableStream<Uint8Array>, max: number): Promise<Uint8Array> {
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let length = 0;
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      length += value.length;
      if (length > max) {
        await reader.cancel();
        throw new Error('The shared experiment is too large. Use a scene file.');
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }
  const result = new Uint8Array(length);
  let offset = 0;
  for (const c of chunks) {
    result.set(c, offset);
    offset += c.length;
  }
  return result;
}
export async function createSceneLink(name: string, scene: SceneDoc): Promise<string> {
  if (!('CompressionStream' in window))
    throw new Error('This browser cannot create compressed links. Export a scene file instead.');
  const data = new TextEncoder().encode(
    JSON.stringify({ name: name.slice(0, 80), scene: parseScene(scene) }),
  );
  const compressed = await bounded(
    new Blob([data]).stream().pipeThrough(new CompressionStream('gzip')),
    18_000,
  );
  const encoded = btoa(String.fromCharCode(...compressed))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
  const url = `${location.origin}${location.pathname}#scene=${encoded}`;
  if (url.length > MAX_LINK)
    throw new Error('This scene is too large for a link. Export a scene file instead.');
  return url;
}
export async function readSceneLink(
  hash: string,
): Promise<{ name: string; scene: SceneDoc } | null> {
  if (!hash.startsWith('#scene=')) return null;
  if (hash.length > MAX_LINK || !/^#scene=[A-Za-z0-9_-]+$/.test(hash))
    throw new Error('The shared scene link is invalid or too large.');
  try {
    const binary = atob(hash.slice(7).replace(/-/g, '+').replace(/_/g, '/'));
    const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
    const decoded = await bounded(
      new Blob([bytes]).stream().pipeThrough(new DecompressionStream('gzip')),
      1_000_000,
    );
    const raw = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(decoded));
    return {
      name:
        typeof raw.name === 'string'
          ? raw.name.slice(0, 80) || 'Shared experiment'
          : 'Shared experiment',
      scene: parseScene(raw.scene),
    };
  } catch {
    throw new Error(
      'This shared scene could not be read. Ask for a new link or import its JSON file.',
    );
  }
}
