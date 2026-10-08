import { CIRCLE, BOX, bodyBounds, polygonVertex } from './engine';
import type { Body, Frame, Joint } from './engine';
import type { View } from './view';

export interface Overlays {
  contacts: boolean;
  impulses: boolean;
  velocities: boolean;
  bounds: boolean;
  trails: boolean;
  sleep: boolean;
  grid: boolean;
}
/** A body about to be placed, in world units. */
export interface Ghost {
  kind: number;
  x: number;
  y: number;
  a: number;
  b: number;
  fixed: boolean;
}
export interface Segment {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}
export interface RenderState {
  frame: Frame;
  view: View;
  selected: number;
  hovered: number;
  overlays: Overlays;
  ghost: Ghost | null;
  /** Pending joint from its first anchor to the pointer. */
  link: Segment | null;
  /** Impulse being aimed, from the body anchor to the pointer. */
  push: Segment | null;
  /** Where a pin would be placed. */
  pin: { x: number; y: number } | null;
  /** Recent positions per body ID, as flat [x0, y0, x1, y1, …] arrays. */
  trails: Map<number, number[]>;
}

const palette = ['#e2a46c', '#86b5ae', '#aaa1c2', '#b8b28c'];
export const bodyColor = (id: number) => palette[id % palette.length];

/** A grid spacing that keeps dots at least ~18 px apart. */
function gridStep(scale: number): number {
  for (const step of [0.25, 0.5, 1, 2, 5, 10, 20, 50]) if (step * scale >= 18) return step;
  return 100;
}
/** A round scale-bar length between roughly 50 and 140 px. */
function scaleBar(scale: number): number {
  for (const m of [0.05, 0.1, 0.2, 0.5, 1, 2, 5, 10, 20, 50, 100]) if (m * scale >= 50) return m;
  return 200;
}
const unit = (m: number) => (m < 1 ? `${Math.round(m * 100)} cm` : `${m} m`);

export function render(ctx: CanvasRenderingContext2D, s: RenderState) {
  const { frame, view, overlays } = s;
  const { width, height, scale, ox, oy } = view;
  ctx.fillStyle = '#111619';
  ctx.fillRect(0, 0, width, height);
  const sx = (x: number) => ox + x * scale;
  const sy = (y: number) => oy - y * scale;
  const line = (x: number, y: number, bx: number, by: number, color: string, w = 1) => {
    ctx.beginPath();
    ctx.moveTo(sx(x), sy(y));
    ctx.lineTo(sx(bx), sy(by));
    ctx.strokeStyle = color;
    ctx.lineWidth = w;
    ctx.stroke();
  };
  const dot = (x: number, y: number, r: number, color: string) => {
    ctx.beginPath();
    ctx.arc(sx(x), sy(y), r, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();
  };
  const arrow = (x: number, y: number, dx: number, dy: number, color: string, w = 1.5) => {
    const ex = sx(x + dx),
      ey = sy(y + dy),
      angle = Math.atan2(-dy, dx);
    ctx.beginPath();
    ctx.moveTo(sx(x), sy(y));
    ctx.lineTo(ex, ey);
    ctx.moveTo(ex - 6 * Math.cos(angle - 0.45), ey - 6 * Math.sin(angle - 0.45));
    ctx.lineTo(ex, ey);
    ctx.lineTo(ex - 6 * Math.cos(angle + 0.45), ey - 6 * Math.sin(angle + 0.45));
    ctx.strokeStyle = color;
    ctx.lineWidth = w;
    ctx.stroke();
  };
  /** Traces a body outline in its local frame (call inside a translated, rotated context). */
  const outline = (kind: number, a: number, b: number) => {
    ctx.beginPath();
    if (kind === CIRCLE) ctx.arc(0, 0, a * scale, 0, Math.PI * 2);
    else if (kind === BOX) ctx.rect(-a * scale, -b * scale, 2 * a * scale, 2 * b * scale);
    else {
      for (let i = 0; i < b; i++) {
        const [vx, vy] = polygonVertex(a, b, i);
        // Canvas y points down; the context is already rotated by -angle.
        if (i === 0) ctx.moveTo(vx * scale, -vy * scale);
        else ctx.lineTo(vx * scale, -vy * scale);
      }
      ctx.closePath();
    }
  };

  // Visible world rectangle, for culling the grid.
  const left = -ox / scale,
    right = (width - ox) / scale,
    top = oy / scale,
    bottom = (oy - height) / scale;
  if (overlays.grid) {
    const step = gridStep(scale);
    ctx.fillStyle = '#283238';
    ctx.beginPath();
    for (let x = Math.ceil(left / step) * step; x <= right; x += step)
      for (let y = Math.ceil(bottom / step) * step; y <= top; y += step)
        ctx.rect(sx(x) - 0.8, sy(y) - 0.8, 1.6, 1.6);
    ctx.fill();
    line(left, 0, right, 0, '#344048');
  }
  if (overlays.trails) {
    ctx.lineCap = 'round';
    for (const [id, points] of s.trails) {
      const n = points.length / 2;
      if (n < 2) continue;
      ctx.strokeStyle = bodyColor(id);
      ctx.lineWidth = 1.4;
      // Draw in a few bands so the tail fades without one stroke per segment.
      const bands = 5;
      for (let band = 0; band < bands; band++) {
        const from = Math.floor((band * (n - 1)) / bands),
          to = Math.floor(((band + 1) * (n - 1)) / bands);
        if (to <= from) continue;
        ctx.globalAlpha = 0.08 + (0.5 * (band + 1)) / bands;
        ctx.beginPath();
        ctx.moveTo(sx(points[from * 2]), sy(points[from * 2 + 1]));
        for (let i = from + 1; i <= to; i++) ctx.lineTo(sx(points[i * 2]), sy(points[i * 2 + 1]));
        ctx.stroke();
      }
    }
    ctx.globalAlpha = 1;
    ctx.lineCap = 'butt';
  }
  for (const j of frame.joints) drawJoint(j);
  for (const b of frame.bodies) drawBody(b);
  for (const c of frame.contacts) {
    if (overlays.contacts) {
      dot(c.x, c.y, 2.6, '#a6d4ca');
      line(c.x, c.y, c.x + (c.nx * 18) / scale, c.y + (c.ny * 18) / scale, '#a6d4ca', 1.2);
    }
    if (overlays.impulses && c.normalImpulse > 0.0001) {
      const length = Math.min(c.normalImpulse * 1.8, 1.2) * Math.max(1, 45 / scale);
      arrow(c.x, c.y, c.nx * length, c.ny * length, '#edab64');
    }
  }
  for (const b of frame.bodies) {
    if (b.id === s.selected) {
      const box = bodyBounds(b);
      ctx.font = '10px ui-monospace, monospace';
      ctx.fillStyle = '#d8c6b1';
      ctx.textAlign = 'center';
      ctx.fillText(`#${String(b.id).padStart(2, '0')}`, sx(b.x), sy(box.maxY) - 9);
    }
  }
  if (s.link) {
    ctx.setLineDash([5, 4]);
    line(s.link.x1, s.link.y1, s.link.x2, s.link.y2, '#a1c1df', 1.3);
    ctx.setLineDash([]);
    dot(s.link.x1, s.link.y1, 3.5, '#a1c1df');
  }
  if (s.push) {
    const { x1, y1, x2, y2 } = s.push;
    arrow(x1, y1, x2 - x1, y2 - y1, '#edab64', 2);
    const magnitude = Math.hypot(x2 - x1, y2 - y1) * 5;
    ctx.font = '10px ui-monospace, monospace';
    ctx.fillStyle = '#edab64';
    ctx.textAlign = 'left';
    ctx.fillText(`${magnitude.toFixed(1)} N·s`, sx(x2) + 8, sy(y2) - 8);
  }
  if (s.pin) drawPin(s.pin.x, s.pin.y, '#e2a46c99', false, 0);
  if (s.ghost) {
    const g = s.ghost;
    ctx.save();
    ctx.translate(sx(g.x), sy(g.y));
    outline(g.kind, g.a, g.b);
    ctx.setLineDash([4, 4]);
    ctx.strokeStyle = g.fixed ? '#8a979c' : '#e2a46c99';
    ctx.lineWidth = 1.2;
    ctx.stroke();
    ctx.restore();
  }
  // The scale bar adapts to the zoom level and uses screen pixels.
  const meters = scaleBar(scale);
  const barX = 25,
    barY = height - 20;
  ctx.font = '10px ui-monospace, monospace';
  ctx.textAlign = 'left';
  ctx.fillStyle = '#728189';
  ctx.fillText(unit(meters), barX, barY - 7);
  ctx.strokeStyle = '#56636a';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(barX, barY);
  ctx.lineTo(barX + meters * scale, barY);
  ctx.moveTo(barX, barY - 3);
  ctx.lineTo(barX, barY + 3);
  ctx.moveTo(barX + meters * scale, barY - 3);
  ctx.lineTo(barX + meters * scale, barY + 3);
  ctx.stroke();

  function drawBody(b: Body) {
    const color = bodyColor(b.id);
    const fixed = b.mass === 0;
    const asleep = overlays.sleep && !b.awake;
    const selected = b.id === s.selected;
    ctx.save();
    ctx.translate(sx(b.x), sy(b.y));
    ctx.rotate(-b.angle);
    outline(b.kind, b.a, b.b);
    ctx.fillStyle = fixed ? '#262e32' : asleep ? `${color}10` : `${color}25`;
    ctx.strokeStyle = selected
      ? '#f7d0a1'
      : b.id === s.hovered
        ? '#d9c8b4'
        : fixed
          ? '#465257'
          : asleep
            ? `${color}66`
            : color;
    ctx.lineWidth = selected ? 2 : b.id === s.hovered ? 1.7 : 1.3;
    ctx.fill();
    ctx.stroke();
    if (!fixed) {
      const reach = b.kind === BOX ? b.a : b.kind === CIRCLE ? b.a : b.a * 0.8;
      ctx.beginPath();
      ctx.moveTo(0, 0);
      ctx.lineTo(Math.min(reach * scale, 9), 0);
      ctx.strokeStyle = `${color}88`;
      ctx.lineWidth = 1;
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(0, 0, 1.6, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.fill();
    }
    ctx.restore();
    if (asleep) {
      ctx.font = '9px ui-monospace, monospace';
      ctx.fillStyle = '#8fa7b3';
      ctx.textAlign = 'left';
      ctx.fillText('z', sx(b.x) + 4, sy(b.y) - 4);
    }
    if (overlays.bounds) {
      const box = bodyBounds(b);
      ctx.setLineDash([3, 4]);
      ctx.strokeStyle = '#59778588';
      ctx.lineWidth = 1;
      ctx.strokeRect(
        sx(box.minX),
        sy(box.maxY),
        (box.maxX - box.minX) * scale,
        (box.maxY - box.minY) * scale,
      );
      ctx.setLineDash([]);
    }
    if (overlays.velocities && !fixed && (b.vx || b.vy))
      arrow(b.x, b.y, b.vx * 0.2, b.vy * 0.2, '#8eabd2');
  }
  function drawPin(x: number, y: number, color: string, toWorld: boolean, motor: number) {
    const px = sx(x),
      py = sy(y);
    if (toWorld) {
      // A small ground symbol marks a pin fixed to the world.
      ctx.strokeStyle = '#6c7c83';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(px - 7, py + 9);
      ctx.lineTo(px + 7, py + 9);
      for (let i = -6; i <= 6; i += 4) {
        ctx.moveTo(px + i, py + 9);
        ctx.lineTo(px + i - 3, py + 13);
      }
      ctx.moveTo(px, py + 5);
      ctx.lineTo(px, py + 9);
      ctx.stroke();
    }
    ctx.beginPath();
    ctx.arc(px, py, 4.5, 0, Math.PI * 2);
    ctx.fillStyle = '#111619';
    ctx.fill();
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.6;
    ctx.stroke();
    ctx.beginPath();
    ctx.arc(px, py, 1.5, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();
    if (motor) {
      // A curved arrow shows the motor's direction; positive speeds turn counter-clockwise,
      // which is also counter-clockwise on screen.
      const ccw = motor > 0,
        r = 10,
        from = ccw ? -0.4 : Math.PI + 0.4,
        to = ccw ? from - 4.4 : from + 4.4;
      ctx.beginPath();
      ctx.arc(px, py, r, from, to, ccw);
      ctx.strokeStyle = '#e7aa70';
      ctx.lineWidth = 1.3;
      ctx.stroke();
      const tx = px + r * Math.cos(to),
        ty = py + r * Math.sin(to);
      const [dx, dy] = ccw ? [Math.sin(to), -Math.cos(to)] : [-Math.sin(to), Math.cos(to)];
      ctx.beginPath();
      ctx.moveTo(tx - dx * 4 - dy * 3, ty - dy * 4 + dx * 3);
      ctx.lineTo(tx, ty);
      ctx.lineTo(tx - dx * 4 + dy * 3, ty - dy * 4 - dx * 3);
      ctx.stroke();
    }
  }
  function drawJoint(j: Joint) {
    const fixedEnd = j.b === 0;
    if (j.kind === 'pin') {
      drawPin(j.ax, j.ay, '#b5c3c5', fixedEnd, j.p2 > 0 ? j.p1 : 0);
      return;
    }
    const dx = j.bx - j.ax,
      dy = j.by - j.ay;
    const distance = Math.hypot(dx, dy);
    if (j.kind === 'rod') line(j.ax, j.ay, j.bx, j.by, '#81949b', 1.5);
    else if (j.kind === 'rope') {
      // Slack rope sags in proportion to the unused length.
      const sag = Math.sqrt(Math.max(0, j.length * j.length - distance * distance)) * 0.5;
      ctx.beginPath();
      ctx.moveTo(sx(j.ax), sy(j.ay));
      ctx.quadraticCurveTo(sx((j.ax + j.bx) / 2), sy((j.ay + j.by) / 2 - sag), sx(j.bx), sy(j.by));
      ctx.strokeStyle = '#b3a58c';
      ctx.lineWidth = 1.4;
      ctx.setLineDash([3, 3]);
      ctx.stroke();
      ctx.setLineDash([]);
    } else {
      // Spring: a zigzag whose color warms as it stretches. Short springs get fewer,
      // shallower coils so dense lattices stay legible.
      const coils = Math.max(2, Math.min(9, Math.round((distance * scale) / 10))),
        amplitude = Math.min(0.12, j.length * 0.1, 6 / scale);
      const ux = distance > 1e-6 ? dx / distance : 1,
        uy = distance > 1e-6 ? dy / distance : 0;
      const strain = j.length > 0 ? (distance - j.length) / j.length : 0;
      ctx.beginPath();
      ctx.moveTo(sx(j.ax), sy(j.ay));
      const lead = 0.12;
      for (let i = 0; i <= coils * 2; i++) {
        const t = lead + ((1 - 2 * lead) * i) / (coils * 2);
        const side = i === 0 || i === coils * 2 ? 0 : i % 2 ? 1 : -1;
        ctx.lineTo(
          sx(j.ax + dx * t - uy * amplitude * side),
          sy(j.ay + dy * t + ux * amplitude * side),
        );
      }
      ctx.lineTo(sx(j.bx), sy(j.by));
      ctx.strokeStyle = strain > 0.08 ? '#d7a77a' : strain < -0.08 ? '#86b5ae' : '#9fb4b0';
      ctx.lineWidth = 1.3;
      ctx.stroke();
    }
    // Anchors at a body's center are already marked by the body's own center dot.
    const offCenter = (x: number, y: number) => Math.hypot(x, y) > 1e-3;
    if (offCenter(j.lax, j.lay)) dot(j.ax, j.ay, 2.5, '#b5c3c5');
    if (fixedEnd) {
      ctx.fillStyle = '#b5c3c5';
      ctx.fillRect(sx(j.bx) - 3.5, sy(j.by) - 3.5, 7, 7);
    } else if (offCenter(j.lbx, j.lby)) dot(j.bx, j.by, 2.5, '#b5c3c5');
  }
}
