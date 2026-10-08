<script lang="ts">
  let { name, size = 18 }: { name: string; size?: number } = $props();
  const paths: Record<string, string> = {
    play: 'm8 5 11 7-11 7V5Z',
    pause: 'M8 5v14M16 5v14',
    step: 'm5 5 10 7-10 7V5ZM19 5v14',
    'step-back': 'm19 5-10 7 10 7V5ZM5 5v14',
    reset: 'M3 10a9 9 0 1 1 2 8M3 4v6h6',
    select: 'm5 3 14 10-7 1-3 7-4-18Z',
    circle: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z',
    box: 'M4 4h16v16H4Z',
    polygon: 'M12 3l7.8 4.5v9L12 21l-7.8-4.5v-9Z',
    impulse: 'm13 2-9 12h7l-1 8 10-13h-7l0-7Z',
    joint: 'M9 8 15 16M9 5a3 3 0 1 1-6 0 3 3 0 0 1 6 0ZM21 19a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
    rod: 'M7 17 17 7M7 19a2 2 0 1 1-4 0 2 2 0 0 1 4 0ZM21 5a2 2 0 1 1-4 0 2 2 0 0 1 4 0Z',
    rope: 'M5 6q7 16 14 0M7 6a2 2 0 1 1-4 0 2 2 0 0 1 4 0ZM21 6a2 2 0 1 1-4 0 2 2 0 0 1 4 0Z',
    spring: 'M2 12h3l2-5 3 10 3-10 3 10 2-5h4',
    pin: 'M15 9a3 3 0 1 1-6 0 3 3 0 0 1 6 0ZM12 12v8M7 20h10',
    download: 'M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5',
    upload: 'M12 15V3m-5 5 5-5 5 5M4 16v5h16v-5',
    trash: 'M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7m4-7v7',
    help: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM9 9a3 3 0 0 1 6 0c0 2-3 2-3 4m0 3h.01',
    chevron: 'm9 5 7 7-7 7',
    close: 'm6 6 12 12M6 18 18 6',
    eye: 'M2 12s4-7 10-7 10 7 10 7-4 7-10 7-10-7-10-7ZM15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
    arrow: 'M5 12h14m-5-5 5 5-5 5',
    crosshair: 'M12 2v5m0 10v5M2 12h5m10 0h5M17 12a5 5 0 1 1-10 0 5 5 0 0 1 10 0Z',
    undo: 'M9 14 4 9l5-5M4 9h11a5 5 0 0 1 0 10h-3',
    redo: 'm15 14 5-5-5-5m5 5H9a5 5 0 0 0 0 10h3',
    image: 'M4 7h3l2-3h6l2 3h3v13H4ZM15.5 13a3.5 3.5 0 1 1-7 0 3.5 3.5 0 0 1 7 0Z',
    plus: 'M12 5v14M5 12h14',
    minus: 'M5 12h14',
    fit: 'M4 9V4h5m6 0h5v5M4 15v5h5m6 0h5v-5',
    follow:
      'M19 12a7 7 0 1 1-14 0 7 7 0 0 1 14 0ZM12 2v3m0 14v3M2 12h3m14 0h3M13 12a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z',
    copy: 'M8 8h12v12H8ZM16 8V4H4v12h4',
    moon: 'M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5Z',
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
