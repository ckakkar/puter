import { readFileSync } from 'node:fs';
import { cpus } from 'node:os';
const { instance } = await WebAssembly.instantiate(readFileSync('web/public/poltergeist.wasm'));
const api = instance.exports;
const scenes = [];
for (const [scene, name] of [
  [0, 'Tower'],
  [6, 'Mechanisms'],
  [10, '160 mixed bodies'],
]) {
  api.reset(scene);
  const samples = [];
  for (let i = 0; i < 600; i++) {
    const start = performance.now();
    api.step(1 / 60);
    api.frame_ptr();
    samples.push(performance.now() - start);
  }
  samples.sort((a, b) => a - b);
  scenes.push({
    name,
    medianMs: Number(samples[300].toFixed(3)),
    p95Ms: Number(samples[570].toFixed(3)),
  });
}
console.log(
  JSON.stringify(
    {
      runtime: process.version,
      platform: process.platform,
      arch: process.arch,
      cpu: cpus()[0]?.model ?? 'unavailable',
      ticks: 600,
      includes:
        'release WASM stepping, history recording, and frame packing; excludes UI rendering',
      scenes,
    },
    null,
    2,
  ),
);
