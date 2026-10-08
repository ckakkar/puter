/** Body shapes, matching the Rust `Shape` codes. */
export const CIRCLE = 0;
export const BOX = 1;
export const POLYGON = 2;
export interface Body {
  id: number;
  /** 0 circle (a = radius), 1 box (a, b = half extents), 2 regular polygon (a = radius, b = sides). */
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
  awake: boolean;
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
export const jointKinds = ['rod', 'rope', 'spring', 'pin'] as const;
export type JointKind = (typeof jointKinds)[number];
export interface Joint {
  id: number;
  kind: JointKind;
  /** World-space anchors. */
  ax: number;
  ay: number;
  bx: number;
  by: number;
  /** Body-local anchors; `local B` is a world point when `b` is 0. */
  lax: number;
  lay: number;
  lbx: number;
  lby: number;
  length: number;
  impulse: number;
  a: number;
  /** Body ID, or 0 when anchored to the world. */
  b: number;
  /** Spring: frequency (Hz) and damping ratio. Pin: motor speed (rad/s) and max torque (N·m). */
  p1: number;
  p2: number;
}
export interface Frame {
  bodies: Body[];
  contacts: Contact[];
  joints: Joint[];
  time: number;
  tick: number;
  pairs: number;
  depth: number;
  kinetic: number;
  potential: number;
  sleeping: number;
  culled: number;
  historyLength: number;
  historyCursor: number;
}
export interface Settings {
  gravity: number;
  iterations: number;
  sleeping: boolean;
}
export const defaultSettings = (): Settings => ({ gravity: 9.81, iterations: 12, sleeping: true });
export type Tool = 'select' | 'circle' | 'box' | 'polygon' | 'impulse' | 'joint';

interface Exports extends WebAssembly.Exports {
  memory: WebAssembly.Memory;
  reset: (scene: number) => void;
  clear: () => void;
  refresh: () => void;
  step: (dt: number) => void;
  configure: (gravity: number, iterations: number, sleeping: number) => void;
  spawn: (
    id: number,
    kind: number,
    x: number,
    y: number,
    a: number,
    b: number,
    mass: number,
  ) => number;
  motion: (id: number, angle: number, vx: number, vy: number, omega: number) => void;
  place: (id: number, x: number, y: number) => void;
  properties: (id: number, mass: number, friction: number, restitution: number) => void;
  remove: (id: number) => void;
  pick: (x: number, y: number) => number;
  pick_below: (x: number, y: number, exclude: number) => number;
  impulse: (id: number, x: number, y: number, jx: number, jy: number) => void;
  drag_start: (id: number, x: number, y: number) => void;
  drag_to: (x: number, y: number) => void;
  drag_end: () => void;
  joint: (
    id: number,
    kind: number,
    a: number,
    b: number,
    ax: number,
    ay: number,
    bx: number,
    by: number,
    length: number,
    p1: number,
    p2: number,
    local: number,
  ) => number;
  joint_set: (id: number, length: number, p1: number, p2: number) => void;
  joint_remove: (id: number) => void;
  history_seek: (index: number) => number;
  clock: (time: number, tick: number) => void;
  frame_ptr: () => number;
  frame_len: () => number;
}

/** Packed frame layout (version 2): a header, then fixed-size records. */
const FRAME_VERSION = 2;
const HEADER = 14;
const BODY = 14;
const CONTACT = 9;
const JOINT = 16;

export const emptyFrame = (): Frame => ({
  bodies: [],
  contacts: [],
  joints: [],
  time: 0,
  tick: 0,
  pairs: 0,
  depth: 0,
  kinetic: 0,
  potential: 0,
  sleeping: 0,
  culled: 0,
  historyLength: 0,
  historyCursor: 0,
});

export const MIN_SIDES = 3;
export const MAX_SIDES = 8;
/** Bodies further than this from the origin are removed by the engine. */
export const WORLD_LIMIT = 300;
const polygonNames = ['Triangle', 'Square', 'Pentagon', 'Hexagon', 'Heptagon', 'Octagon'];
export function shapeName(body: Pick<Body, 'kind' | 'b'>): string {
  if (body.kind === CIRCLE) return 'Circle';
  if (body.kind === BOX) return 'Box';
  return polygonNames[body.b - MIN_SIDES] ?? 'Polygon';
}
/** Local vertex `i` of a regular polygon; mirrors `collision::polygon_vertex` in Rust. */
export function polygonVertex(radius: number, sides: number, i: number): [number, number] {
  const theta = -Math.PI / 2 + Math.PI / sides + (2 * Math.PI * i) / sides;
  return [radius * Math.cos(theta), radius * Math.sin(theta)];
}
export function area(kind: number, a: number, b: number): number {
  if (kind === CIRCLE) return Math.PI * a * a;
  if (kind === BOX) return 4 * a * b;
  return 0.5 * b * a * a * Math.sin((2 * Math.PI) / b);
}
/** Density used to give newly placed bodies a mass proportional to their size (kg/m²). */
export const DENSITY = 1.4;
export const massFor = (kind: number, a: number, b: number) =>
  Math.min(100, Math.max(0.05, Number((DENSITY * area(kind, a, b)).toFixed(2))));
/** Axis-aligned bounds of a body in world space. */
export function bodyBounds(b: Body): { minX: number; minY: number; maxX: number; maxY: number } {
  if (b.kind === POLYGON) {
    let minX = Infinity,
      minY = Infinity,
      maxX = -Infinity,
      maxY = -Infinity;
    const c = Math.cos(b.angle),
      s = Math.sin(b.angle);
    for (let i = 0; i < b.b; i++) {
      const [vx, vy] = polygonVertex(b.a, b.b, i);
      const x = b.x + c * vx - s * vy,
        y = b.y + s * vx + c * vy;
      minX = Math.min(minX, x);
      maxX = Math.max(maxX, x);
      minY = Math.min(minY, y);
      maxY = Math.max(maxY, y);
    }
    return { minX, minY, maxX, maxY };
  }
  const c = Math.abs(Math.cos(b.angle)),
    s = Math.abs(Math.sin(b.angle));
  const ex = b.kind === CIRCLE ? b.a : c * b.a + s * b.b;
  const ey = b.kind === CIRCLE ? b.a : s * b.a + c * b.b;
  return { minX: b.x - ex, minY: b.y - ey, maxX: b.x + ex, maxY: b.y + ey };
}
/** Converts a world point into a body's local frame, and back. */
export function toLocal(b: Body, x: number, y: number): { x: number; y: number } {
  const c = Math.cos(-b.angle),
    s = Math.sin(-b.angle),
    dx = x - b.x,
    dy = y - b.y;
  return { x: c * dx - s * dy, y: s * dx + c * dy };
}
export function toWorld(b: Body, x: number, y: number): { x: number; y: number } {
  const c = Math.cos(b.angle),
    s = Math.sin(b.angle);
  return { x: b.x + c * x - s * y, y: b.y + s * x + c * y };
}

/** A scene file. Version 2 adds polygons, joint kinds, world anchors, and sleeping. */
export interface SceneBody {
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
export interface SceneJoint {
  id: number;
  kind: JointKind;
  a: number;
  b: number;
  /** Body-local anchors; anchorB is a world point when b is 0. */
  anchorA: [number, number];
  anchorB: [number, number];
  length: number;
  impulse?: number;
  frequency?: number;
  damping?: number;
  motorSpeed?: number;
  maxTorque?: number;
}
export interface SceneDoc extends Settings {
  format: 'poltergeist-scene';
  version: 2;
  bodies: SceneBody[];
  joints: SceneJoint[];
}

const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const within = (v: unknown, lo: number, hi: number): v is number => finite(v) && v >= lo && v <= hi;
const bodyKeys = [
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
] as const;

/** Validates any supported scene document and normalizes it to version 2. Throws on failure. */
export function parseScene(raw: unknown): SceneDoc {
  // Narrowed field by field below.
  const s = raw as any;
  if (
    !s ||
    typeof s !== 'object' ||
    s.format !== 'poltergeist-scene' ||
    (s.version !== 1 && s.version !== 2) ||
    !Array.isArray(s.bodies) ||
    !Array.isArray(s.joints) ||
    s.bodies.length > 256 ||
    s.joints.length > 256
  )
    throw new Error('Choose a Poltergeist scene (v1 or v2) with at most 256 bodies and joints.');
  const sleeping = s.version === 1 ? true : s.sleeping;
  if (
    !within(s.gravity, 0, 30) ||
    !Number.isInteger(s.iterations) ||
    s.iterations < 1 ||
    s.iterations > 32 ||
    typeof sleeping !== 'boolean'
  )
    throw new Error('Invalid simulation settings.');
  const bodies = new Map<number, SceneBody>();
  for (const b of s.bodies) {
    const sizeOk =
      b?.kind === POLYGON
        ? within(b.a, 1e-3, 20) && Number.isInteger(b.b) && b.b >= MIN_SIDES && b.b <= MAX_SIDES
        : within(b?.a, 1e-3, 20) && within(b?.b, 1e-3, 20);
    if (
      !b ||
      !bodyKeys.every((k) => finite(b[k])) ||
      !Number.isInteger(b.id) ||
      b.id <= 0 ||
      b.id > 1e9 ||
      bodies.has(b.id) ||
      ![CIRCLE, BOX, POLYGON].includes(b.kind) ||
      (s.version === 1 && b.kind === POLYGON) ||
      !sizeOk ||
      !within(b.mass, 0, 100) ||
      !within(b.friction, 0, 1.5) ||
      !within(b.restitution, 0, 1) ||
      !within(b.x, -WORLD_LIMIT, WORLD_LIMIT) ||
      !within(b.y, -WORLD_LIMIT, WORLD_LIMIT) ||
      !within(b.vx, -1000, 1000) ||
      !within(b.vy, -1000, 1000) ||
      !within(b.omega, -1000, 1000)
    )
      throw new Error('Scene contains an invalid body.');
    bodies.set(b.id, Object.fromEntries(bodyKeys.map((k) => [k, b[k]])) as unknown as SceneBody);
  }
  const dynamic = (id: number) => (bodies.get(id)?.mass ?? 0) > 0;
  const joints: SceneJoint[] = [];
  const jointIds = new Set<number>();
  const invalid = () => new Error('Scene contains an invalid joint.');
  for (const [index, j] of (s.joints as Record<string, unknown>[]).entries()) {
    if (!j || typeof j !== 'object') throw invalid();
    let joint: SceneJoint;
    if (s.version === 1) {
      // Version 1 joints are rods with world-space anchors.
      if (!['a', 'b', 'ax', 'ay', 'bx', 'by', 'length'].every((k) => finite(j[k]))) throw invalid();
      const a = bodies.get(j.a as number),
        b = bodies.get(j.b as number);
      if (!a || !b) throw invalid();
      const local = (body: SceneBody, x: number, y: number): [number, number] => {
        const c = Math.cos(-body.angle),
          sn = Math.sin(-body.angle);
        return [c * (x - body.x) - sn * (y - body.y), sn * (x - body.x) + c * (y - body.y)];
      };
      joint = {
        id: index + 1,
        kind: 'rod',
        a: a.id,
        b: b.id,
        anchorA: local(a, j.ax as number, j.ay as number),
        anchorB: local(b, j.bx as number, j.by as number),
        length: j.length as number,
      };
    } else {
      const anchor = (v: unknown) =>
        Array.isArray(v) && v.length === 2 && v.every((n) => within(n, -1000, 1000));
      if (
        !Number.isInteger(j.id) ||
        (j.id as number) <= 0 ||
        !jointKinds.includes(j.kind as JointKind) ||
        !finite(j.a) ||
        !finite(j.b) ||
        !anchor(j.anchorA) ||
        !anchor(j.anchorB) ||
        !finite(j.length)
      )
        throw invalid();
      joint = {
        id: j.id as number,
        kind: j.kind as JointKind,
        a: j.a as number,
        b: j.b as number,
        anchorA: [...(j.anchorA as [number, number])],
        anchorB: [...(j.anchorB as [number, number])],
        length: j.length as number,
      };
      if (joint.kind === 'spring') {
        if (!within(j.frequency, 0.05, 30) || !within(j.damping, 0, 5)) throw invalid();
        joint.frequency = j.frequency as number;
        joint.damping = j.damping as number;
      }
      if (joint.kind === 'pin') {
        if (!within(j.motorSpeed, -50, 50) || !within(j.maxTorque, 0, 10_000)) throw invalid();
        joint.motorSpeed = j.motorSpeed as number;
        joint.maxTorque = j.maxTorque as number;
      }
    }
    const distance = joint.kind !== 'pin';
    if (
      jointIds.has(joint.id) ||
      !bodies.has(joint.a) ||
      (joint.b !== 0 && !bodies.has(joint.b)) ||
      joint.a === joint.b ||
      !(dynamic(joint.a) || dynamic(joint.b)) ||
      !within(joint.length, distance ? 0.01 : 0, 2 * WORLD_LIMIT)
    )
      throw invalid();
    jointIds.add(joint.id);
    joints.push(joint);
  }
  return {
    format: 'poltergeist-scene',
    version: 2,
    gravity: s.gravity,
    iterations: s.iterations,
    sleeping,
    bodies: [...bodies.values()],
    joints,
  };
}

function jointParams(j: SceneJoint): [number, number] {
  if (j.kind === 'spring') return [j.frequency ?? 2, j.damping ?? 0.3];
  if (j.kind === 'pin') return [j.motorSpeed ?? 0, j.maxTorque ?? 0];
  return [0, 0];
}

export class Engine {
  settings: Settings = defaultSettings();
  constructor(public api: Exports) {}
  static async load(): Promise<Engine> {
    const response = await fetch('/poltergeist.wasm');
    if (!response.ok)
      throw new Error(`Physics engine could not load (${response.status}). Run npm run wasm.`);
    const result = await WebAssembly.instantiate(await response.arrayBuffer());
    return new Engine(result.instance.exports as Exports);
  }
  configure(settings: Settings) {
    this.settings = { ...settings };
    this.api.configure(settings.gravity, settings.iterations, settings.sleeping ? 1 : 0);
  }
  read(): Frame {
    const ptr = this.api.frame_ptr();
    // Copy immediately: future Rust calls may grow memory or rebuild this buffer.
    const d = new Float32Array(this.api.memory.buffer, ptr, this.api.frame_len()).slice();
    if (d[0] !== FRAME_VERSION) throw new Error('Unsupported physics frame format.');
    const f: Frame = {
      ...emptyFrame(),
      time: d[4],
      tick: d[5],
      pairs: d[6],
      depth: d[7],
      kinetic: d[8],
      potential: d[9],
      sleeping: d[10],
      culled: d[11],
      historyLength: d[12],
      historyCursor: d[13],
    };
    let p = HEADER;
    for (let i = 0; i < d[1]; i++, p += BODY)
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
        awake: d[p + 13] !== 0,
      });
    for (let i = 0; i < d[2]; i++, p += CONTACT)
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
    for (let i = 0; i < d[3]; i++, p += JOINT)
      f.joints.push({
        id: d[p],
        kind: jointKinds[d[p + 1]] ?? 'rod',
        ax: d[p + 2],
        ay: d[p + 3],
        bx: d[p + 4],
        by: d[p + 5],
        lax: d[p + 6],
        lay: d[p + 7],
        lbx: d[p + 8],
        lby: d[p + 9],
        length: d[p + 10],
        impulse: d[p + 11],
        a: d[p + 12],
        b: d[p + 13],
        p1: d[p + 14],
        p2: d[p + 15],
      });
    return f;
  }
  /** The current world as a scene document. */
  snapshot(): SceneDoc {
    const f = this.read();
    return {
      format: 'poltergeist-scene',
      version: 2,
      ...this.settings,
      bodies: f.bodies.map(({ awake: _awake, ...b }) => b),
      joints: f.joints.map((j) => {
        const joint: SceneJoint = {
          id: j.id,
          kind: j.kind,
          a: j.a,
          b: j.b,
          anchorA: [j.lax, j.lay],
          anchorB: [j.lbx, j.lby],
          length: j.length,
          impulse: j.impulse,
        };
        if (j.kind === 'spring') Object.assign(joint, { frequency: j.p1, damping: j.p2 });
        if (j.kind === 'pin') Object.assign(joint, { motorSpeed: j.p1, maxTorque: j.p2 });
        return joint;
      }),
    };
  }
  exportScene(): string {
    return JSON.stringify(this.snapshot(), null, 2);
  }
  /**
   * Replaces the world with a validated scene, keeping body and joint IDs. If the engine
   * rejects anything, the previous world is restored and the error is rethrown.
   */
  load(doc: SceneDoc, settings: Settings = doc) {
    const backup = this.snapshot();
    try {
      this.apply(doc, settings);
    } catch (error) {
      this.apply(backup, backup);
      throw error;
    }
  }
  importScene(text: string): Settings {
    if (text.length > 1_000_000) throw new Error('Scene file is too large (maximum 1 MB).');
    let raw: unknown;
    try {
      raw = JSON.parse(text);
    } catch {
      throw new Error('That file is not valid JSON.');
    }
    // Validate the complete document before mutating the running world.
    const doc = parseScene(raw);
    this.load(doc);
    return { gravity: doc.gravity, iterations: doc.iterations, sleeping: doc.sleeping };
  }
  private apply(doc: SceneDoc, settings: Settings) {
    this.api.clear();
    for (const b of doc.bodies) {
      if (this.api.spawn(b.id, b.kind, b.x, b.y, b.a, b.b, b.mass) !== b.id)
        throw new Error('Scene contains an invalid body.');
      this.api.motion(b.id, b.angle, b.vx, b.vy, b.omega);
      this.api.properties(b.id, b.mass, b.friction, b.restitution);
    }
    for (const j of doc.joints) {
      const [p1, p2] = jointParams(j);
      const kind = jointKinds.indexOf(j.kind);
      const [ax, ay] = j.anchorA,
        [bx, by] = j.anchorB;
      if (this.api.joint(j.id, kind, j.a, j.b, ax, ay, bx, by, j.length, p1, p2, 1) !== j.id)
        throw new Error('Scene contains an invalid joint.');
    }
    this.configure(settings);
    this.api.refresh();
  }
}
