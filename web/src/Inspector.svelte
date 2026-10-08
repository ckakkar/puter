<script lang="ts" module>
  import type { Body, Contact, Joint, JointKind } from './engine';
  export interface InspectorActions {
    property: (key: 'mass' | 'friction' | 'restitution', value: number) => void;
    setStatic: (fixed: boolean) => void;
    rotate: (degrees: number) => void;
    duplicate: () => void;
    remove: () => void;
    updateJoint: (joint: Joint, change: Partial<Pick<Joint, 'length' | 'p1' | 'p2'>>) => void;
    removeJoint: (joint: Joint) => void;
    select: (id: number) => void;
  }
</script>

<script lang="ts">
  import Icon from './Icon.svelte';
  import Slider from './Slider.svelte';
  import { BOX, CIRCLE, shapeName } from './engine';

  let {
    body,
    contacts,
    joints,
    actions,
  }: { body: Body | undefined; contacts: Contact[]; joints: Joint[]; actions: InspectorActions } =
    $props();
  const fmt = (n: number, digits = 2) => (Number.isFinite(n) ? n.toFixed(digits) : '—');
  const jointLabels: Record<JointKind, string> = {
    rod: 'Rod',
    rope: 'Rope',
    spring: 'Spring',
    pin: 'Pin',
  };
  const degrees = $derived.by(() => {
    if (!body) return 0;
    const d = (body.angle * 180) / Math.PI;
    return ((((d + 180) % 360) + 360) % 360) - 180;
  });
  const speed = $derived(body ? Math.hypot(body.vx, body.vy) : 0);
  const kinetic = $derived.by(() => {
    if (!body || body.mass === 0) return 0;
    const unitInertia =
      body.kind === CIRCLE
        ? 0.5 * body.a * body.a
        : body.kind === BOX
          ? (body.a * body.a + body.b * body.b) / 3
          : (body.a * body.a * (1 + 2 * Math.cos(Math.PI / body.b) ** 2)) / 6;
    return 0.5 * body.mass * speed * speed + 0.5 * body.mass * unitInertia * body.omega ** 2;
  });
  const size = $derived.by(() => {
    if (!body) return '';
    if (body.kind === CIRCLE) return `r ${fmt(body.a)} m`;
    if (body.kind === BOX) return `${fmt(2 * body.a)} × ${fmt(2 * body.b)} m`;
    return `r ${fmt(body.a)} m`;
  });
  const status = $derived(
    !body
      ? ''
      : body.mass === 0
        ? 'STATIC BODY'
        : body.awake
          ? 'DYNAMIC · AWAKE'
          : 'DYNAMIC · ASLEEP',
  );
  const other = (j: Joint) => (body && j.a === body.id ? j.b : j.a);
  // Constraint cards start open only when there are few of them; toggled cards remember it.
  let toggled = $state(new Set<number>());
  const isOpen = (id: number) => toggled.has(id) !== joints.length <= 2;
  function toggle(id: number) {
    const next = new Set(toggled);
    if (!next.delete(id)) next.add(id);
    toggled = next;
  }
</script>

<section class="panel-section inspector">
  <div class="section-label">BODY INSPECTOR <span class="live-badge">LIVE</span></div>
  {#if body}
    <div class="selected-heading">
      <div class="selected-icon">
        <Icon
          name={body.kind === CIRCLE ? 'circle' : body.kind === BOX ? 'box' : 'polygon'}
          size={23}
        />
      </div>
      <div class="selected-title">
        <h3>{shapeName(body)} <span>#{String(body.id).padStart(2, '0')}</span></h3>
        <small class:asleep={!body.awake && body.mass > 0}>{status}</small>
      </div>
      <button
        class="icon-button"
        onclick={actions.duplicate}
        aria-label="Duplicate selected body"
        title="Duplicate · ⌘D"><Icon name="copy" size={15} /></button
      ><button
        class="icon-button delete-button"
        onclick={actions.remove}
        aria-label="Delete selected body"
        title="Delete · Del"><Icon name="trash" size={15} /></button
      >
    </div>
    <div class="vector-group">
      <div class="section-label">POSITION <span>m</span></div>
      <div class="vector-values">
        <span><small>X</small>{fmt(body.x)}</span><span><small>Y</small>{fmt(body.y)}</span>
      </div>
    </div>
    <div class="vector-group">
      <div class="section-label">VELOCITY <span>m/s</span></div>
      <div class="vector-values">
        <span><small>X</small>{fmt(body.vx)}</span><span><small>Y</small>{fmt(body.vy)}</span>
      </div>
    </div>
    <div class="data-row"><span>Speed</span><strong>{fmt(speed)}<small>m/s</small></strong></div>
    <div class="data-row">
      <span>Angular velocity</span><strong>{fmt(body.omega)}<small>rad/s</small></strong>
    </div>
    <div class="data-row">
      <span>Kinetic energy</span><strong>{fmt(kinetic)}<small>J</small></strong>
    </div>
    <div class="data-row"><span>Size</span><strong>{size}</strong></div>
    <div class="body-properties">
      <label class="toggle-row static-toggle"
        ><span class="overlay-dot"></span><span>Static (immovable)</span><input
          type="checkbox"
          checked={body.mass === 0}
          onchange={(e) => actions.setStatic(e.currentTarget.checked)}
          aria-label="Static body"
        /><span class="toggle-track"></span></label
      >
      <Slider
        id="rotation"
        label="Rotation"
        value={degrees}
        min={-180}
        max={180}
        step={1}
        unit="°"
        digits={1}
        onchange={actions.rotate}
      />
      {#if body.mass > 0}<Slider
          id="mass"
          label="Mass"
          value={body.mass}
          min={0.1}
          max={20}
          step={0.1}
          inputMin={0.05}
          inputMax={100}
          unit="kg"
          onchange={(v) => actions.property('mass', v)}
        />{/if}
      <Slider
        id="friction"
        label="Friction"
        value={body.friction}
        min={0}
        max={1.5}
        step={0.01}
        onchange={(v) => actions.property('friction', v)}
      />
      <Slider
        id="bounce"
        label="Restitution"
        value={body.restitution}
        min={0}
        max={1}
        step={0.01}
        onchange={(v) => actions.property('restitution', v)}
      />
    </div>
    <div class="contact-summary">
      <span class="contact-color-dot"></span><span
        >{contacts.length} active contact{contacts.length === 1 ? '' : 's'}</span
      ><strong
        >{fmt(
          contacts.reduce((s, c) => s + c.normalImpulse, 0),
          3,
        )}<small>N·s</small></strong
      >
    </div>
    <p class="microcopy">Normal impulses from the latest solver substep.</p>
    {#if joints.length}
      <div class="joint-list">
        <div class="section-label">CONSTRAINTS <span>{joints.length}</span></div>
        {#each joints as j (j.id)}
          <div class="joint-card">
            <div class="joint-head">
              <Icon name={j.kind} size={14} />
              <span
                >{jointLabels[j.kind]} →
                {#if other(j) === 0}world{:else}<button
                    class="link-button"
                    onclick={() => actions.select(other(j))}
                    >#{String(other(j)).padStart(2, '0')}</button
                  >{/if}</span
              >
              <small>{fmt(Math.abs(j.impulse), 2)} N·s</small>
              <button
                class="icon-button joint-toggle"
                class:open={isOpen(j.id)}
                onclick={() => toggle(j.id)}
                aria-expanded={isOpen(j.id)}
                aria-label="Edit {j.kind} joint {j.id}"
                title="Edit constraint"><Icon name="chevron" size={12} /></button
              ><button
                class="icon-button joint-remove"
                onclick={() => actions.removeJoint(j)}
                aria-label="Remove {j.kind} joint {j.id}"
                title="Remove constraint"><Icon name="close" size={13} /></button
              >
            </div>
            {#if isOpen(j.id) && j.kind !== 'pin'}<Slider
                id="joint-{j.id}-length"
                label="Length"
                value={j.length}
                min={0.05}
                max={Math.max(10, Math.ceil(j.length))}
                step={0.01}
                inputMin={0.01}
                inputMax={600}
                unit="m"
                onchange={(v) => actions.updateJoint(j, { length: v })}
              />{/if}
            {#if isOpen(j.id) && j.kind === 'spring'}<Slider
                id="joint-{j.id}-frequency"
                label="Frequency"
                value={j.p1}
                min={0.2}
                max={10}
                step={0.1}
                inputMin={0.05}
                inputMax={30}
                unit="Hz"
                onchange={(v) => actions.updateJoint(j, { p1: v })}
              /><Slider
                id="joint-{j.id}-damping"
                label="Damping ratio"
                value={j.p2}
                min={0}
                max={1.5}
                step={0.01}
                inputMax={5}
                onchange={(v) => actions.updateJoint(j, { p2: v })}
              />{/if}
            {#if isOpen(j.id) && j.kind === 'pin'}<Slider
                id="joint-{j.id}-motor"
                label="Motor speed"
                value={j.p1}
                min={-10}
                max={10}
                step={0.1}
                inputMin={-50}
                inputMax={50}
                unit="rad/s"
                onchange={(v) => actions.updateJoint(j, { p1: v, p2: j.p2 > 0 ? j.p2 : 200 })}
              /><Slider
                id="joint-{j.id}-torque"
                label="Motor torque"
                value={j.p2}
                min={0}
                max={1000}
                step={5}
                inputMax={10000}
                unit="N·m"
                digits={0}
                onchange={(v) => actions.updateJoint(j, { p2: v })}
              />{/if}
          </div>
        {/each}
      </div>
    {/if}
  {:else}
    <div class="empty-inspector">
      <div class="inspector-illustration"><span></span><Icon name="select" size={24} /></div>
      <h3>Look a little closer.</h3>
      <p>Select any body to inspect its motion, material, and contacts.</p>
      <span class="empty-tag">EVERY COLLISION TELLS A STORY</span>
    </div>
  {/if}
</section>
