<script lang="ts" module>
  export interface EnergySample {
    tick: number;
    t: number;
    k: number;
    p: number;
  }
  /** Seconds of simulated time shown. */
  export const WINDOW = 10;
</script>

<script lang="ts">
  let { samples, version }: { samples: EnergySample[]; version: number } = $props();
  // Categorical slots validated together on the panel surface (#191d20, dark).
  const series = [
    { label: 'Kinetic', color: '#3987e5', value: (s: EnergySample) => s.k },
    { label: 'Potential', color: '#d95926', value: (s: EnergySample) => s.p },
    { label: 'Total', color: '#199e70', value: (s: EnergySample) => s.k + s.p },
  ];
  const height = 118;
  const pad = { top: 8, right: 60, bottom: 16, left: 34 };
  let canvas: HTMLCanvasElement;
  let width = $state(220);
  let hoverX = $state<number | null>(null);

  const formatEnergy = (v: number) => {
    const a = Math.abs(v);
    if (a >= 1e4) return `${(v / 1000).toFixed(0)}k`;
    if (a >= 1000) return `${(v / 1000).toFixed(1)}k`;
    return a >= 100 ? v.toFixed(0) : v.toFixed(1);
  };
  function niceCeil(v: number) {
    if (v <= 0) return 1;
    const p = 10 ** Math.floor(Math.log10(v));
    for (const m of [1, 2, 2.5, 5, 10]) if (m * p >= v) return m * p;
    return 10 * p;
  }
  /** Samples inside the visible time window. */
  const visible = $derived.by(() => {
    void version;
    if (!samples.length) return [];
    const end = samples[samples.length - 1].t;
    let i = samples.length - 1;
    while (i > 0 && samples[i - 1].t >= end - WINDOW) i--;
    return samples.slice(i);
  });
  const domain = $derived.by(() => {
    const t1 = visible.length ? visible[visible.length - 1].t : WINDOW;
    const t0 = Math.min(visible.length ? visible[0].t : 0, t1 - 1);
    let lo = 0,
      hi = 0;
    for (const s of visible)
      for (const sr of series) {
        const v = sr.value(s);
        lo = Math.min(lo, v);
        hi = Math.max(hi, v);
      }
    const top = niceCeil(hi);
    const bottom = lo < 0 ? -niceCeil(-lo) : 0;
    return { t0, t1, top, bottom };
  });
  const plotWidth = $derived(Math.max(40, width - pad.left - pad.right));
  const plotHeight = height - pad.top - pad.bottom;
  const xOf = (t: number) =>
    pad.left + ((t - domain.t0) / Math.max(1e-6, domain.t1 - domain.t0)) * plotWidth;
  const yOf = (v: number) =>
    pad.top + (1 - (v - domain.bottom) / Math.max(1e-6, domain.top - domain.bottom)) * plotHeight;
  /** The sample nearest the pointer, so the crosshair snaps to real data. */
  const hovered = $derived.by(() => {
    if (hoverX === null || visible.length < 2) return null;
    let best = visible[0];
    for (const s of visible)
      if (Math.abs(xOf(s.t) - hoverX) < Math.abs(xOf(best.t) - hoverX)) best = s;
    return best;
  });
  const latest = $derived(visible.length ? visible[visible.length - 1] : null);

  $effect(() => {
    const ctx = canvas?.getContext('2d');
    if (!ctx) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = Math.round(width * dpr);
    canvas.height = Math.round(height * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);
    // Recessive grid: baseline plus the top and middle values.
    ctx.font = '8px ui-monospace, monospace';
    ctx.textAlign = 'left';
    const ticks = [domain.top, (domain.top + domain.bottom) / 2, domain.bottom];
    if (domain.bottom < 0) ticks.push(0);
    for (const v of ticks) {
      const y = Math.round(yOf(v)) + 0.5;
      ctx.strokeStyle = v === 0 ? '#3a4448' : '#262e32';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(pad.left, y);
      ctx.lineTo(pad.left + plotWidth, y);
      ctx.stroke();
      ctx.fillStyle = '#6f828a';
      ctx.textAlign = 'right';
      ctx.fillText(`${formatEnergy(v)} J`, pad.left - 5, y + 3);
    }
    ctx.textAlign = 'left';
    ctx.fillStyle = '#6f828a';
    ctx.fillText(`−${Math.min(WINDOW, domain.t1 - domain.t0).toFixed(0)} s`, pad.left, height - 3);
    ctx.textAlign = 'right';
    ctx.fillText('now', pad.left + plotWidth, height - 3);
    if (visible.length < 2) return;
    ctx.lineWidth = 2;
    ctx.lineJoin = 'round';
    ctx.lineCap = 'round';
    for (const sr of series) {
      ctx.strokeStyle = sr.color;
      ctx.beginPath();
      visible.forEach((s, i) => {
        const x = xOf(s.t),
          y = yOf(sr.value(s));
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.stroke();
    }
    // Direct labels at the line ends, nudged apart so they never collide.
    const end = visible[visible.length - 1];
    const labels = series.map((sr) => ({ sr, y: yOf(sr.value(end)) })).sort((a, b) => a.y - b.y);
    for (let i = 1; i < labels.length; i++)
      labels[i].y = Math.max(labels[i].y, labels[i - 1].y + 10);
    const overflow = labels[labels.length - 1].y - (height - pad.bottom + 2);
    if (overflow > 0) for (const l of labels) l.y -= overflow;
    ctx.textAlign = 'left';
    ctx.font = '8px ui-monospace, monospace';
    for (const { sr, y } of labels) {
      ctx.strokeStyle = sr.color;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(pad.left + plotWidth + 5, y);
      ctx.lineTo(pad.left + plotWidth + 11, y);
      ctx.stroke();
      ctx.fillStyle = '#a7b4b6';
      ctx.fillText(sr.label, pad.left + plotWidth + 14, y + 3);
    }
    if (hovered) {
      const x = Math.round(xOf(hovered.t)) + 0.5;
      ctx.strokeStyle = '#6c7c83';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(x, pad.top);
      ctx.lineTo(x, pad.top + plotHeight);
      ctx.stroke();
      for (const sr of series) {
        ctx.beginPath();
        ctx.arc(x, yOf(sr.value(hovered)), 4, 0, Math.PI * 2);
        ctx.fillStyle = sr.color;
        ctx.fill();
        ctx.strokeStyle = '#191d20';
        ctx.lineWidth = 2;
        ctx.stroke();
      }
    }
  });
</script>

<div class="energy-chart" bind:clientWidth={width}>
  <canvas
    bind:this={canvas}
    style:width="{width}px"
    style:height="{height}px"
    aria-hidden="true"
    onpointermove={(e) => (hoverX = e.offsetX)}
    onpointerleave={() => (hoverX = null)}
  ></canvas>
  {#if visible.length < 2}
    <p class="energy-empty">Run the simulation to chart energy over time.</p>
  {/if}
  {#if hovered}
    <!-- Sits in the corner opposite the crosshair so it never hides the hovered data. -->
    <div
      class="energy-tooltip"
      class:right={xOf(hovered.t) <= pad.left + plotWidth / 2}
      role="status"
    >
      <span class="energy-time">t = {hovered.t.toFixed(2)} s</span>
      {#each series as sr}<span class="energy-row"
          ><i style:background={sr.color}></i><strong>{formatEnergy(sr.value(hovered))} J</strong
          ><small>{sr.label}</small></span
        >{/each}
    </div>
  {/if}
  <table class="energy-table">
    <caption class="visually-hidden">Current mechanical energy</caption>
    <tbody>
      {#each series as sr}<tr
          ><th scope="row"><i style:background={sr.color}></i>{sr.label}</th><td
            >{latest ? formatEnergy(sr.value(latest)) : '—'}<small>J</small></td
          ></tr
        >{/each}
    </tbody>
  </table>
</div>
