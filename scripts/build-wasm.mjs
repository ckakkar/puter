import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
const result = spawnSync(
  'cargo',
  [
    'build',
    '--offline',
    '--release',
    '--target',
    'wasm32-unknown-unknown',
    '-p',
    'poltergeist-wasm',
  ],
  { stdio: 'inherit' },
);
if (result.status !== 0) {
  console.error(
    'WASM build failed. Install Rust and run: rustup target add wasm32-unknown-unknown',
  );
  process.exit(result.status ?? 1);
}
mkdirSync('web/public', { recursive: true });
copyFileSync(
  'target/wasm32-unknown-unknown/release/poltergeist_wasm.wasm',
  'web/public/poltergeist.wasm',
);
