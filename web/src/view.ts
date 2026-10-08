/** Screen mapping: world meters (y up) to CSS pixels (y down). */
export interface View {
  width: number;
  height: number;
  scale: number;
  ox: number;
  oy: number;
}
/** Camera center in world meters and a zoom factor relative to fitting the chamber. */
export interface Camera {
  x: number;
  y: number;
  zoom: number;
}
export const MIN_ZOOM = 0.15;
export const MAX_ZOOM = 8;
/** Frames the 20 m × 9.6 m laboratory. */
export const homeCamera = (): Camera => ({ x: 0, y: 3.8, zoom: 1 });

export function viewFor(width: number, height: number, camera: Camera = homeCamera()): View {
  const scale = Math.min(width / 20, height / 9.6) * camera.zoom;
  return {
    width,
    height,
    scale,
    ox: width / 2 - camera.x * scale,
    oy: height / 2 + camera.y * scale,
  };
}
export function screenToWorld(view: View, x: number, y: number): { x: number; y: number } {
  return { x: (x - view.ox) / view.scale, y: (view.oy - y) / view.scale };
}
export function worldToScreen(view: View, x: number, y: number): { x: number; y: number } {
  return { x: view.ox + x * view.scale, y: view.oy - y * view.scale };
}
/** Zooms by `factor` while keeping the world point under screen point (sx, sy) fixed. */
export function zoomAt(camera: Camera, view: View, sx: number, sy: number, factor: number): Camera {
  const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, camera.zoom * factor));
  const p = screenToWorld(view, sx, sy);
  const scale = (view.scale / camera.zoom) * zoom;
  return {
    zoom,
    x: p.x - (sx - view.width / 2) / scale,
    y: p.y + (sy - view.height / 2) / scale,
  };
}
/** Moves the camera so content follows a screen-space drag of (dx, dy) pixels. */
export function panBy(camera: Camera, view: View, dx: number, dy: number): Camera {
  return { ...camera, x: camera.x - dx / view.scale, y: camera.y + dy / view.scale };
}
