import type { Frame } from './engine';
/** Rendering is interpolated; exported and inspected state always stays authoritative. */
export function interpolate(previous: Frame, current: Frame, alpha: number): Frame {
  if (current.tick !== previous.tick + 1 || current.bodies.length !== previous.bodies.length)
    return current;
  const t = Math.min(1, Math.max(0, alpha));
  return {
    ...current,
    bodies: current.bodies.map((b, i) => {
      const p = previous.bodies[i];
      if (p.id !== b.id || b.mass === 0 || !b.awake) return b;
      return {
        ...b,
        x: p.x + (b.x - p.x) * t,
        y: p.y + (b.y - p.y) * t,
        angle: p.angle + (b.angle - p.angle) * t,
      };
    }),
    joints: current.joints.map((j, i) => {
      const p = previous.joints[i];
      if (!p || p.id !== j.id) return j;
      return {
        ...j,
        ax: p.ax + (j.ax - p.ax) * t,
        ay: p.ay + (j.ay - p.ay) * t,
        bx: p.bx + (j.bx - p.bx) * t,
        by: p.by + (j.by - p.by) * t,
      };
    }),
  };
}
