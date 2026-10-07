<script lang="ts">
  let { name, size = 18 }: { name: string; size?: number } = $props();
  const paths: Record<string, string> = {
    play: 'm8 5 11 7-11 7V5Z',
    pause: 'M8 5v14M16 5v14',
    step: 'm5 5 10 7-10 7V5ZM19 5v14',
    reset: 'M3 10a9 9 0 1 1 2 8M3 4v6h6',
    select: 'm5 3 14 10-7 1-3 7-4-18Z',
    circle: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z',
    box: 'M4 4h16v16H4Z',
    impulse: 'm13 2-9 12h7l-1 8 10-13h-7l0-7Z',
    joint: 'M9 8 15 16M9 5a3 3 0 1 1-6 0 3 3 0 0 1 6 0ZM21 19a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
    download: 'M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5',
    upload: 'M12 15V3m-5 5 5-5 5 5M4 16v5h16v-5',
    trash: 'M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7m4-7v7',
    help: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM9 9a3 3 0 0 1 6 0c0 2-3 2-3 4m0 3h.01',
    chevron: 'm9 5 7 7-7 7',
    close: 'm6 6 12 12M6 18 18 6',
    eye: 'M2 12s4-7 10-7 10 7 10 7-4 7-10 7-10-7-10-7ZM15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
    arrow: 'M5 12h14m-5-5 5 5-5 5',
    crosshair: 'M12 2v5m0 10v5M2 12h5m10 0h5M17 12a5 5 0 1 1-10 0 5 5 0 0 1 10 0Z',
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"><path d={paths[name] ?? paths.box} /></svg
>
