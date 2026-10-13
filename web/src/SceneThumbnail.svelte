<script lang="ts">
  import { onMount } from 'svelte';
  import { emptyFrame } from './engine';
  import type { SceneDoc } from './engine';
  import { render } from './renderer';
  import { viewFor } from './view';
  let { scene }: { scene: SceneDoc } = $props();
  let canvas: HTMLCanvasElement;
  let mounted = $state(false);
  onMount(() => (mounted = true));
  $effect(() => {
    if (!mounted) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const bodies = scene.bodies.map((b) => ({ ...b, awake: true }));
    render(ctx, {
      frame: { ...emptyFrame(), bodies },
      view: viewFor(160, 100),
      selected: 0,
      hovered: 0,
      overlays: {
        contacts: false,
        impulses: false,
        velocities: false,
        bounds: false,
        trails: false,
        sleep: false,
        grid: false,
      },
      ghost: null,
      link: null,
      push: null,
      pin: null,
      trails: new Map(),
    });
  });
</script>

<canvas class="scene-thumbnail" width="160" height="100" aria-hidden="true" bind:this={canvas}
></canvas>
