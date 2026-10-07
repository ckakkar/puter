export interface Body {
  id: number;
  kind: number;
  x: number;
  y: number;
  angle: number;
  a: number;
  b: number;
  mass: number;
  friction: number;
  restitution: number;
  vx: number;
  vy: number;
  omega: number;
}
export interface Contact {
  x: number;
  y: number;
  nx: number;
  ny: number;
  depth: number;
  normalImpulse: number;
  tangentImpulse: number;
  a: number;
  b: number;
}
export interface Joint {
  ax: number;
  ay: number;
  bx: number;
  by: number;
  length: number;
  impulse: number;
  a: number;
  b: number;
}
export interface Frame {
  bodies: Body[];
  contacts: Contact[];
  joints: Joint[];
  time: number;
  tick: number;
  pairs: number;
  depth: number;
  energy: number;
}
export type Tool = 'select' | 'circle' | 'box' | 'impulse' | 'joint';
interface Exports extends WebAssembly.Exports {
  memory: WebAssembly.Memory;
  reset: (scene: number) => void;
  clear: () => void;
  step: (dt: number) => void;
  configure: (gravity: number, iterations: number) => void;
  spawn: (kind: number, x: number, y: number, a: number, b: number, mass: number) => number;
  motion: (id: number, angle: number, vx: number, vy: number, omega: number) => void;
  properties: (id: number, mass: number, friction: number, restitution: number) => void;
  remove: (id: number) => void;
  pick: (x: number, y: number) => number;
  impulse: (id: number, x: number, y: number, jx: number, jy: number) => void;
  drag_start: (id: number, x: number, y: number) => void;
  drag_to: (x: number, y: number) => void;
  drag_end: () => void;
  joint: (
    a: number,
    b: number,
    ax: number,
    ay: number,
    bx: number,
    by: number,
    length: number,
  ) => void;
  frame_ptr: () => number;
  frame_len: () => number;
}
export const emptyFrame = (): Frame => ({
  bodies: [],
  contacts: [],
  joints: [],
  time: 0,
  tick: 0,
  pairs: 0,
  depth: 0,
  energy: 0,
});
export class Engine {
  constructor(public api: Exports) {}
  static async load(): Promise<Engine> {
    const response = await fetch('/poltergeist.wasm');
    if (!response.ok)
      throw new Error(`Physics engine could not load (${response.status}). Run npm run wasm.`);
    const result = await WebAssembly.instantiate(await response.arrayBuffer());
    return new Engine(result.instance.exports as Exports);
  }
  read(): Frame {
    const ptr = this.api.frame_ptr();
    // Copy immediately: future Rust calls may grow memory or rebuild this buffer.
    const d = new Float32Array(this.api.memory.buffer, ptr, this.api.frame_len()).slice();
    if (d[0] !== 1) throw new Error('Unsupported physics frame format.');
    const f: Frame = {
      ...emptyFrame(),
      time: d[4],
      tick: d[5],
      pairs: d[6],
      depth: d[7],
      energy: d[8],
    };
    let p = 9;
    for (let i = 0; i < d[1]; i++, p += 13)
      f.bodies.push({
        id: d[p],
        kind: d[p + 1],
        x: d[p + 2],
        y: d[p + 3],
        angle: d[p + 4],
        a: d[p + 5],
        b: d[p + 6],
        mass: d[p + 7],
        friction: d[p + 8],
        restitution: d[p + 9],
        vx: d[p + 10],
        vy: d[p + 11],
        omega: d[p + 12],
      });
    for (let i = 0; i < d[2]; i++, p += 9)
      f.contacts.push({
        x: d[p],
        y: d[p + 1],
        nx: d[p + 2],
        ny: d[p + 3],
        depth: d[p + 4],
        normalImpulse: d[p + 5],
        tangentImpulse: d[p + 6],
        a: d[p + 7],
        b: d[p + 8],
      });
    for (let i = 0; i < d[3]; i++, p += 8)
      f.joints.push({
        ax: d[p],
        ay: d[p + 1],
        bx: d[p + 2],
        by: d[p + 3],
        length: d[p + 4],
        impulse: d[p + 5],
        a: d[p + 6],
        b: d[p + 7],
      });
    return f;
  }
  exportScene(gravity: number, iterations: number): string {
    const f = this.read();
    return JSON.stringify(
      {
        format: 'poltergeist-scene',
        version: 1,
        gravity,
        iterations,
        bodies: f.bodies,
        joints: f.joints,
      },
      null,
      2,
    );
  }
  importScene(text: string): { gravity: number; iterations: number } {
    if (text.length > 1_000_000) throw new Error('Scene file is too large (maximum 1 MB).');
    const s = JSON.parse(text);
    const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
    if (
      s.format !== 'poltergeist-scene' ||
      s.version !== 1 ||
      !Array.isArray(s.bodies) ||
      !Array.isArray(s.joints) ||
      s.bodies.length > 256 ||
      s.joints.length > 256
    )
      throw new Error('Choose a Poltergeist v1 scene with at most 256 bodies and joints.');
    if (
      !finite(s.gravity) ||
      s.gravity < 0 ||
      s.gravity > 30 ||
      !Number.isInteger(s.iterations) ||
      s.iterations < 1 ||
      s.iterations > 32
    )
      throw new Error('Invalid simulation settings.');
    const ids = new Set<number>();
    for (const b of s.bodies) {
      if (
        !b ||
        ![
          'id',
          'kind',
          'x',
          'y',
          'angle',
          'a',
          'b',
          'mass',
          'friction',
          'restitution',
          'vx',
          'vy',
          'omega',
        ].every((k) => finite(b[k])) ||
        !Number.isInteger(b.id) ||
        b.id <= 0 ||
        ids.has(b.id) ||
        ![0, 1].includes(b.kind) ||
        b.a <= 0 ||
        b.b <= 0 ||
        b.a > 20 ||
        b.b > 20 ||
        b.mass < 0 ||
        b.mass > 100 ||
        b.friction < 0 ||
        b.friction > 1.5 ||
        b.restitution < 0 ||
        b.restitution > 1 ||
        Math.abs(b.x) > 100 ||
        Math.abs(b.y) > 100 ||
        Math.abs(b.vx) > 100 ||
        Math.abs(b.vy) > 100 ||
        Math.abs(b.omega) > 100
      )
        throw new Error('Scene contains an invalid body.');
      ids.add(b.id);
    }
    for (const j of s.joints) {
      if (
        !j ||
        !['a', 'b', 'ax', 'ay', 'bx', 'by', 'length'].every((k) => finite(j[k])) ||
        !ids.has(j.a) ||
        !ids.has(j.b) ||
        j.a === j.b ||
        j.length <= 0 ||
        j.length > 100 ||
        [j.ax, j.ay, j.bx, j.by].some((v) => Math.abs(v) > 100)
      )
        throw new Error('Scene contains an invalid joint.');
    }
    // Validate the complete document before mutating the running world.
    this.api.clear();
    const mapping = new Map<number, number>();
    for (const b of s.bodies as Body[]) {
      const id = this.api.spawn(b.kind, b.x, b.y, b.a, b.b, b.mass);
      mapping.set(b.id, id);
      this.api.motion(id, b.angle, b.vx, b.vy, b.omega);
      this.api.properties(id, b.mass, b.friction, b.restitution);
    }
    for (const j of s.joints as Joint[])
      this.api.joint(mapping.get(j.a)!, mapping.get(j.b)!, j.ax, j.ay, j.bx, j.by, j.length);
    this.api.configure(s.gravity, s.iterations);
    return { gravity: s.gravity, iterations: s.iterations };
  }
}
