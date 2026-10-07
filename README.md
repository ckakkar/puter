# Poltergeist

An inspectable 2D physics sandbox: create bodies, pull them through a spring, apply impulses, connect distance joints, and reveal contacts and solver impulses. The same custom Rust engine runs in native tests and in the browser as WebAssembly.

## Run locally

Requires Node.js 22.12+ (or 20.19+) and a current stable Rust toolchain.

```sh
rustup target add wasm32-unknown-unknown
npm install
npm run dev
```

Open **http://127.0.0.1:5173**. The dev command builds the WASM binary before starting Vite. After changing Rust, run `npm run wasm` and refresh the browser; web changes reload automatically. The Rust crates have no external dependencies, so subsequent Rust builds work offline.

```sh
npm run build       # WASM, Svelte/TypeScript checks, production bundle
npm run preview     # serve the production build
npm run test:core   # native physics regressions
npm run test:e2e    # desktop + mobile browser workflows
cargo clippy --workspace --offline -- -D warnings
```

For browser tests, install Chromium once with `npx playwright install chromium`.

## What is implemented

- Static and dynamic circles and oriented boxes, including angular motion and mass-dependent inertia.
- AABB candidate filtering; circle collision routines; separating-axis box collisions with clipped contact manifolds.
- Sequential accumulated normal/friction impulses, contact warm starting, restitution threshold, and separate positional correction.
- Fixed 60 Hz ticks with two physics substeps. Browser frame rate is independent of simulation time; excess catch-up work is capped.
- Distance joints and a capped spring-based pointer constraint.
- Tower, pendulum, friction, and bounce experiments.
- Selectable bodies, editable mass/friction/restitution, pause/reset/single-step, simulation speed, gravity, and solver iterations.
- Contact normals, impulse arrows, velocity arrows, bounding boxes, and a reference grid.
- Versioned JSON scene import/export, validated before replacing the world. Exports preserve bodies, motion, settings, and joints; importing pauses the simulation.
- Responsive layout, keyboard shortcuts, an object list for selection, and modal help.

## Structure

```text
crates/poltergeist-core/   Headless physics, collision geometry, regression tests
crates/poltergeist-wasm/   Thin scalar WebAssembly API and packed frame output
web/src/engine.ts         WASM adapter and scene serialization/validation
web/src/renderer.ts       Canvas rendering and inspection overlays
web/src/App.svelte        Sandbox controls and inspectors
tests/                   Browser workflows on desktop and mobile
scripts/build-wasm.mjs    Reproducible Rust-to-WASM build
```

The browser instantiates a local WASM asset directly: no backend, account, API keys, external fonts, or network services. The UI sends commands to the engine and copies a packed render frame out of WASM memory. The headless core does not depend on browser APIs or the UI framework.

## Engine conventions

Meters, kilograms, seconds, and radians are canonical. Positive Y points upward; only the renderer converts to pixels. Contact normals point from body A to B. Displayed contact impulses are **N·s**, sampled from the latest solver substep, not forces or totals over an entire render frame. Restitution combines with `min`; friction combines with the geometric mean.

The engine has stable body/contact iteration order and is single-threaded. This is not a claim of bit-identical native/browser results or cross-platform determinism. Rendering and input state live outside the engine. Presets reset the world; an imported scene's reset restores its original imported snapshot.

## First-version boundaries

This is an extensible prototype, not a replacement for a production physics engine. It is bounded to 256 bodies and 256 joints. Broad-phase pair enumeration is O(n²) with AABB rejection. Collisions are discrete: fast bodies can tunnel, and small substeps do not guarantee prevention. There is no sleeping, continuous collision detection, general polygon support, islands, multithreaded solver, or reverse replay. The pointer spring is intentionally simple. Exporting a scene does not preserve contact caches or execution history.

Useful next steps: sweep-and-prune broad phase, contact-block solving for stacks, sleeping/islands, continuous collision detection, and recorded inputs with full-state checkpoints. Keep changes measurable with scene regressions and performance measurements on stated hardware.

## References

- [Erin Catto: Sequential Impulses](https://box2d.org/files/ErinCatto_SequentialImpulses_GDC2006.pdf)
- [Box2D collision documentation](https://box2d.org/documentation/md_collision.html)
- [Box2D simulation documentation](https://box2d.org/documentation/md_simulation.html)

No Git repository is initialized. `.gitignore` excludes dependencies, build outputs, generated WASM, and test reports. Lockfiles are retained for reproducible installs.
