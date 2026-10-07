import type { Frame, Tool } from './engine';
export interface View {
  width: number;
  height: number;
  scale: number;
  ox: number;
  oy: number;
}
export interface Overlays {
  contacts: boolean;
  bounds: boolean;
  velocities: boolean;
  impulses: boolean;
  grid: boolean;
}
export interface Pointer {
  x: number;
  y: number;
  startX?: number;
  startY?: number;
  active: boolean;
}
export function viewFor(width: number, height: number): View {
  const scale = Math.min(width / 20, height / 9.6);
  return { width, height, scale, ox: width / 2, oy: height / 2 + 3.8 * scale };
}
export function screenToWorld(view: View, x: number, y: number): { x: number; y: number } {
  return { x: (x - view.ox) / view.scale, y: (view.oy - y) / view.scale };
}
const palette = ['#e2a46c', '#86b5ae', '#aaa1c2', '#b8b28c'];
export function render(
  ctx: CanvasRenderingContext2D,
  frame: Frame,
  view: View,
  selected: number,
  overlays: Overlays,
  tool: Tool,
  pointer: Pointer,
  jointStart: number,
) {
  const { width, height, scale, ox, oy } = view;
  ctx.clearRect(0, 0, width, height);
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
  const arrow = (x: number, y: number, dx: number, dy: number, color: string) => {
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
    ctx.lineWidth = 1.5;
    ctx.stroke();
  };
  if (overlays.grid) {
    ctx.fillStyle = '#283238';
    for (let x = -11; x <= 11; x += 0.5)
      for (let y = -1; y <= 10; y += 0.5) {
        ctx.beginPath();
        ctx.arc(sx(x), sy(y), 0.8, 0, Math.PI * 2);
        ctx.fill();
      }
    line(-10, 0, 10, 0, '#344048');
  }
  for (const j of frame.joints) {
    line(j.ax, j.ay, j.bx, j.by, '#81949b', 1.5);
    for (const [x, y] of [
      [j.ax, j.ay],
      [j.bx, j.by],
    ]) {
      ctx.beginPath();
      ctx.arc(sx(x), sy(y), 3, 0, Math.PI * 2);
      ctx.fillStyle = '#b5c3c5';
      ctx.fill();
    }
  }
  for (const b of frame.bodies) {
    ctx.save();
    ctx.translate(sx(b.x), sy(b.y));
    ctx.rotate(-b.angle);
    const color = palette[b.id % palette.length];
    ctx.beginPath();
    if (b.kind === 0) ctx.arc(0, 0, b.a * scale, 0, Math.PI * 2);
    else ctx.rect(-b.a * scale, -b.b * scale, 2 * b.a * scale, 2 * b.b * scale);
    ctx.fillStyle = b.mass === 0 ? '#262e32' : `${color}25`;
    ctx.strokeStyle = b.id === selected ? '#f7d0a1' : b.mass === 0 ? '#465257' : color;
    ctx.lineWidth = b.id === selected ? 2 : 1.3;
    ctx.fill();
    ctx.stroke();
    if (b.mass > 0) {
      ctx.beginPath();
      ctx.moveTo(0, 0);
      ctx.lineTo(Math.min(b.a * scale, 9), 0);
      ctx.strokeStyle = `${color}88`;
      ctx.lineWidth = 1;
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(0, 0, 1.6, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.fill();
    }
    ctx.restore();
    if (overlays.bounds) {
      const c = Math.abs(Math.cos(b.angle)),
        s = Math.abs(Math.sin(b.angle));
      const ax = b.kind === 0 ? b.a : c * b.a + s * b.b,
        ay = b.kind === 0 ? b.a : s * b.a + c * b.b;
      ctx.setLineDash([3, 4]);
      ctx.strokeStyle = '#59778588';
      ctx.lineWidth = 1;
      ctx.strokeRect(sx(b.x - ax), sy(b.y + ay), 2 * ax * scale, 2 * ay * scale);
      ctx.setLineDash([]);
    }
    if (overlays.velocities && b.mass > 0) arrow(b.x, b.y, b.vx * 0.2, b.vy * 0.2, '#8eabd2');
    if (b.id === selected) {
      ctx.font = '10px ui-monospace, monospace';
      ctx.fillStyle = '#d8c6b1';
      ctx.textAlign = 'center';
      ctx.fillText(`#${String(b.id).padStart(2, '0')}`, sx(b.x), sy(b.y + b.b) - 11);
    }
  }
  for (const c of frame.contacts) {
    if (overlays.contacts) {
      ctx.beginPath();
      ctx.arc(sx(c.x), sy(c.y), 2.6, 0, Math.PI * 2);
      ctx.fillStyle = '#a6d4ca';
      ctx.fill();
      line(c.x, c.y, c.x + c.nx * 0.25, c.y + c.ny * 0.25, '#a6d4ca', 1.2);
    }
    if (overlays.impulses && c.normalImpulse > 0.0001) {
      const length = Math.min(c.normalImpulse * 1.8, 1.2);
      arrow(c.x, c.y, c.nx * length, c.ny * length, '#edab64');
    }
  }
  const first = frame.bodies.find((b) => b.id === jointStart);
  if (first) line(first.x, first.y, pointer.x, pointer.y, '#a1c1df');
  if (
    pointer.active &&
    tool === 'impulse' &&
    pointer.startX !== undefined &&
    pointer.startY !== undefined
  ) {
    arrow(
      pointer.startX,
      pointer.startY,
      pointer.x - pointer.startX,
      pointer.y - pointer.startY,
      '#edab64',
    );
  }
  if (!pointer.active && (tool === 'circle' || tool === 'box')) {
    ctx.save();
    ctx.strokeStyle = '#e2a46c88';
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    if (tool === 'circle') ctx.arc(sx(pointer.x), sy(pointer.y), 0.42 * scale, 0, Math.PI * 2);
    else ctx.rect(sx(pointer.x - 0.48), sy(pointer.y + 0.38), 0.96 * scale, 0.76 * scale);
    ctx.stroke();
    ctx.restore();
  }
  // Scale and coordinate axes use screen pixels, independent of physical units.
  ctx.font = '10px ui-monospace, monospace';
  ctx.textAlign = 'left';
  ctx.fillStyle = '#728189';
  ctx.fillText('1 m', 25, height - 27);
  ctx.strokeStyle = '#56636a';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(25, height - 20);
  ctx.lineTo(25 + scale, height - 20);
  ctx.moveTo(25, height - 23);
  ctx.lineTo(25, height - 17);
  ctx.moveTo(25 + scale, height - 23);
  ctx.lineTo(25 + scale, height - 17);
  ctx.stroke();
}
