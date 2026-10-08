# Poltergeist

An inspectable 2D physics sandbox: build with circles, boxes, and polygons; connect them with rods, ropes, springs, and motorized pins; pull, push, and rewind them; and reveal contacts, impulses, energy, and sleep states. The same custom Rust engine runs in native tests and in the browser as WebAssembly.

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
cargo clippy --workspace --offline --all-targets -- -D warnings
```

For browser tests, install Chromium once with `npx playwright install chromium`.

## What is implemented

**Engine**

- Static and dynamic circles, oriented boxes, and regular polygons (3–8 sides), with angular motion and mass-dependent inertia.
- Sweep-and-prune broad phase; circle, box, and polygon narrow phases; separating-axis manifolds with clipped contact points.
- Sequential accumulated normal/friction impulses, contact warm starting, a restitution threshold, and separate positional correction.
- Joints between two bodies or a body and a fixed world point: rigid rods, ropes (maximum distance), soft springs (frequency and damping ratio), and pins with optional motors.
- Island sleeping: touching or jointed bodies sleep together once still for 0.5 s and wake together when disturbed. It can be switched off.
- Fixed 60 Hz ticks with two physics substeps. Browser frame rate is independent of simulation time; excess catch-up work is capped. Speeds are capped, and dynamic bodies that leave the laboratory (300 m out or 60 m below the floor) are removed.
- A ten-second timeline of recorded states for stepping and scrubbing backward.

**Lab**

- Eight experiments: tower, pendulums, friction, bounce, dominoes, spring lattices, mechanisms, and an empty chamber.
- Click to place a body, or drag to size it; place static scenery; move bodies directly while paused; pull them through a spring while running.
- Body inspector with editable rotation, mass, friction, restitution, static toggle, duplicate/delete, and attached constraints (length, spring frequency and damping, motor speed and torque). Every value can be typed for precision.
- Overlays for contacts, impulses, velocities, motion trails, bounds, sleeping bodies, and a reference grid; a live energy chart (kinetic, potential, total).
- Snapshot-based undo/redo for every edit, including loading a scene or importing a file.
- Zoom, pan, pinch, and follow-the-selection camera.
- Versioned JSON scene import/export (version 2; version 1 files still load), validated before replacing the world. PNG snapshots of the chamber.
- Responsive layout, keyboard shortcuts, an object list for selection, and remembered preferences (overlays, speed, placement options).

## Controls

| Action                                                | Mouse / touch                                   | Keyboard                                               |
| ----------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------ |
| Play / pause                                          | ▶ button                                        | Space                                                  |
| Step forward / back in time                           | ⏭ / ⏮ buttons, timeline                         | N or → / ←                                             |
| Tools: select, circle, box, polygon, impulse, connect | Toolbox                                         | V C B P I J (J again cycles rod → rope → spring → pin) |
| Place a body                                          | Click, or drag to size (Shift for a square box) | S toggles static placement                             |
| Move / pull a body                                    | Drag (moves while paused, pulls while running)  | Q / E rotate 15° (Shift: 1°)                           |
| Pan / zoom                                            | Drag empty space or right-drag; scroll or pinch | + / − zoom, 0 reset view, F follow                     |
| Undo / redo / duplicate / delete                      | Topbar, inspector                               | ⌘Z / ⇧⌘Z / ⌘D / Delete (Ctrl on Windows and Linux)     |

## Structure

```text
crates/poltergeist-core/   Headless physics, collision geometry, joints, timeline, scenes, regression tests
crates/poltergeist-wasm/   Thin scalar WebAssembly API and packed frame output
web/src/engine.ts         WASM adapter, frame decoding, scene validation and (de)serialization
web/src/renderer.ts       Canvas rendering and inspection overlays
web/src/view.ts           Camera: world ↔ screen mapping, zoom, pan
web/src/App.svelte        Sandbox controls, input handling, and the frame loop
web/src/Inspector.svelte  Body and constraint inspector
web/src/EnergyChart.svelte  Live energy chart
web/src/undo.ts           Snapshot undo/redo stack
web/src/prefs.ts          Remembered per-browser preferences
tests/                   Browser workflows on desktop and mobile
scripts/build-wasm.mjs    Reproducible Rust-to-WASM build
```

The browser instantiates a local WASM asset directly: no backend, account, API keys, external fonts, or network services. The UI sends commands to the engine and copies a packed render frame out of WASM memory. The headless core does not depend on browser APIs or the UI framework.

## Engine conventions

Meters, kilograms, seconds, and radians are canonical. Positive Y points upward; only the renderer converts to pixels. Contact normals point from body A to B. Displayed contact and joint impulses are **N·s**, sampled from the latest solver substep, not forces or totals over an entire render frame. Restitution combines with `min`; friction combines with the geometric mean. Potential energy is gravitational energy relative to y = 0 plus energy stored in springs. Newly placed bodies get a mass of 1.4 kg/m² × area.

The engine has stable body/contact iteration order and is single-threaded. This is not a claim of bit-identical native/browser results or cross-platform determinism. Rendering and input state live outside the engine. Presets reset the world; an imported scene's reset restores its original imported snapshot.

## Scene files

Exports are version 2 JSON: settings (`gravity`, `iterations`, `sleeping`), `bodies` (`kind` 0 circle, 1 box, 2 polygon with `a` as radius and `b` as side count), and `joints` with a `kind` (`rod`, `rope`, `spring`, `pin`), body-local anchors, and kind-specific parameters. A joint with `b: 0` is anchored to the world. Body and joint IDs are preserved on import. Version 1 files (rods with world-space anchors) still import.

## Boundaries

This is an extensible prototype, not a replacement for a production physics engine. It is bounded to 256 bodies and 256 joints. Collisions are discrete: fast bodies can tunnel, and small substeps do not guarantee prevention. There is no continuous collision detection, general (irregular) polygon support, compound bodies, joint limits, multithreaded solver, or block solver for stacks. The timeline restores body motion but not contact caches, so a replay resumed from a rewound state can diverge slightly. The pointer spring is intentionally simple. Exporting a scene does not preserve contact caches or execution history.

Useful next steps: continuous collision detection for fast bodies, contact-block solving for stacks, compound and irregular convex shapes, joint limits, and recorded inputs with full-state checkpoints for exact replay. Keep changes measurable with scene regressions and performance measurements on stated hardware.

## References

- [Erin Catto: Sequential Impulses](https://box2d.org/files/ErinCatto_SequentialImpulses_GDC2006.pdf)
- [Erin Catto: Soft Constraints](https://box2d.org/files/ErinCatto_SoftConstraints_GDC2011.pdf)
- [Box2D collision documentation](https://box2d.org/documentation/md_collision.html)
- [Box2D simulation documentation](https://box2d.org/documentation/md_simulation.html)

`.gitignore` excludes dependencies, build outputs, generated WASM, and test reports. Lockfiles are retained for reproducible installs.
