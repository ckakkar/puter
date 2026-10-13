import { area } from './engine';
import type { Body } from './engine';
/** Stylized 2D materials, not measured three-dimensional material constants. */
export const materials = [
  { id: 'default', name: 'Default', density: 1.4, friction: 0.55, restitution: 0.12 },
  { id: 'wood', name: 'Wood', density: 2, friction: 0.65, restitution: 0.2 },
  { id: 'rubber', name: 'Rubber', density: 1.2, friction: 0.85, restitution: 0.8 },
  { id: 'steel', name: 'Steel', density: 6, friction: 0.35, restitution: 0.1 },
  { id: 'ice', name: 'Ice', density: 1, friction: 0.02, restitution: 0.05 },
] as const;
export type MaterialId = (typeof materials)[number]['id'];
export function materialMass(id: MaterialId, body: Pick<Body, 'kind' | 'a' | 'b'>): number {
  const m = materials.find((m) => m.id === id) ?? materials[0];
  return Math.min(
    100,
    Math.max(0.05, Number((area(body.kind, body.a, body.b) * m.density).toFixed(2))),
  );
}
