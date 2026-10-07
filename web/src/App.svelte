<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import { Engine, emptyFrame } from './engine';
  import type { Frame, Tool } from './engine';
  import { render, screenToWorld, viewFor } from './renderer';
  import type { Overlays, Pointer } from './renderer';

  const scenes = [
    {
      name: 'Tower collapse',
      sub: 'Contacts & stability',
      number: '01',
      description:
        'Twenty-one blocks. One small disturbance. Observe how a structure loses its balance.',
    },
    {
      name: 'Pendulum garden',
      sub: 'Distance constraints',
      number: '02',
      description:
        'Five suspended bodies. Pull one aside and follow the impulses that hold its orbit.',
    },
    {
      name: 'Friction study',
      sub: 'Three surfaces, three outcomes',
      number: '03',
      description:
        'Identical blocks on identical slopes. Only friction changes: 0.02, 0.20, and 0.90.',
    },
    {
      name: 'Bounce chamber',
      sub: 'Restitution & motion',
      number: '04',
      description:
        'A collection of elastic bodies. Watch energy move between translation and rotation.',
    },
  ];
  const tools: { id: Tool; label: string; key: string; icon: string }[] = [
    { id: 'select', label: 'Select & drag', key: 'V', icon: 'select' },
    { id: 'circle', label: 'Add circle', key: 'C', icon: 'circle' },
    { id: 'box', label: 'Add box', key: 'B', icon: 'box' },
    { id: 'impulse', label: 'Apply impulse', key: 'I', icon: 'impulse' },
    { id: 'joint', label: 'Connect bodies', key: 'J', icon: 'joint' },
  ];
  const overlayLabels: Record<keyof Overlays, string> = {
    contacts: 'Contacts & normals',
    impulses: 'Contact impulses',
    velocities: 'Velocity vectors',
    bounds: 'Collision bounds',
    grid: 'Reference grid',
  };
  let engine: Engine;
  let canvas: HTMLCanvasElement;
  let viewport: HTMLDivElement;
  let fileInput: HTMLInputElement;
  let ready = $state(false);
  let fatal = $state('');
  let frame: Frame = $state(emptyFrame());
  let running = $state(true);
  let scene = $state(0);
  let customScene = $state(false);
  let nudged = $state(false);
  let tool: Tool = $state('select');
  let selected = $state(0);
  let jointStart = $state(0);
  let gravity = $state(9.81);
  let iterations = $state(12);
  let speed = $state(1);
  let simMs = $state(0);
  let fps = $state(60);
  let help = $state(false);
  let toast = $state('');
  let toastTimer: ReturnType<typeof setTimeout>;
  let overlays: Overlays = $state({
    contacts: true,
    impulses: false,
    velocities: false,
    bounds: false,
    grid: true,
  });
  let pointer: Pointer = { x: -4, y: 4, active: false };
  let impulseBody = 0;
  let view = viewFor(800, 600);
  let canvasWidth = 800;
  let canvasHeight = 600;
  let raf = 0;
  let animationTime = 0;
  let accumulator = 0;
  let refreshTime = 0;
  let initialCustomScene = '';
  let helpDialog: HTMLDialogElement;
  const body = $derived(frame.bodies.find((b) => b.id === selected));
  const bodyContacts = $derived(frame.contacts.filter((c) => c.a === selected || c.b === selected));
  const dynamicCount = $derived(frame.bodies.filter((b) => b.mass > 0).length);
  const currentTitle = $derived(customScene ? 'Custom experiment' : scenes[scene].name);
  const toolHint = $derived(
    tool === 'select'
      ? 'Select a body to inspect. Drag to pull it.'
      : tool === 'impulse'
        ? 'Drag from a body in the direction you want to push.'
        : tool === 'joint'
          ? jointStart
            ? 'Select a second body to connect.'
            : 'Select two different bodies to add a distance joint.'
          : `Click inside the chamber to add a ${tool}.`,
  );
  const fmt = (n: number, digits = 2) => (Number.isFinite(n) ? n.toFixed(digits) : '—');

  function notify(message: string) {
    toast = message;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ''), 4000);
  }
  function sync() {
    if (ready) frame = engine.read();
  }
  function loadScene(index: number) {
    if (!ready) return;
    scene = index;
    customScene = false;
    nudged = false;
    initialCustomScene = '';
    selected = 0;
    jointStart = 0;
    pointer.active = false;
    engine.api.reset(index);
    engine.api.configure(gravity, iterations);
    accumulator = 0;
    sync();
  }
  function resetScene() {
    if (customScene && initialCustomScene) {
      const s = engine.importScene(initialCustomScene);
      gravity = s.gravity;
      iterations = s.iterations;
      selected = 0;
      jointStart = 0;
      accumulator = 0;
      sync();
    } else loadScene(scene);
  }
  function pause() {
    if (!ready) return;
    running = !running;
    accumulator = 0;
    engine.api.drag_end();
    pointer.active = false;
  }
  function singleStep() {
    if (!ready) return;
    running = false;
    accumulator = 0;
    engine.api.step(1 / 60);
    sync();
  }
  function configure() {
    if (ready) {
      engine.api.configure(gravity, iterations);
      sync();
    }
  }
  function setTool(next: Tool) {
    tool = next;
    jointStart = 0;
    if (ready) engine.api.drag_end();
    pointer.active = false;
  }
  function updateBody(key: 'mass' | 'friction' | 'restitution', value: number) {
    if (!body) return;
    engine.api.properties(
      body.id,
      key === 'mass' ? value : body.mass,
      key === 'friction' ? value : body.friction,
      key === 'restitution' ? value : body.restitution,
    );
    sync();
  }
  function deleteBody() {
    if (!body) return;
    if (body.mass === 0) {
      notify('Static scenery stays in place. Select a dynamic body.');
      return;
    }
    engine.api.remove(body.id);
    selected = 0;
    jointStart = 0;
    sync();
  }
  function kick() {
    if (!ready) return;
    const target =
      frame.bodies.find((b) => b.mass > 0 && b.kind === 0) ?? frame.bodies.find((b) => b.mass > 0);
    if (target) {
      engine.api.impulse(target.id, target.x, target.y, 16, 3);
      selected = target.id;
      nudged = true;
      sync();
      notify('Impulse applied: 16 N·s →, 3 N·s ↑');
    }
  }
  function pointerPosition(event: PointerEvent) {
    const rect = canvas.getBoundingClientRect();
    return screenToWorld(
      view,
      ((event.clientX - rect.left) * canvasWidth) / rect.width,
      ((event.clientY - rect.top) * canvasHeight) / rect.height,
    );
  }
  function pointerDown(event: PointerEvent) {
    if (!ready || event.button !== 0) return;
    const p = pointerPosition(event);
    pointer = { ...p, active: true, startX: p.x, startY: p.y };
    canvas.setPointerCapture(event.pointerId);
    const id = engine.api.pick(p.x, p.y);
    if (tool === 'circle' || tool === 'box') {
      if (p.x < -8.6 || p.x > 8.6 || p.y < 0.5 || p.y > 8.5) {
        notify('Place bodies inside the chamber, above the floor.');
        return;
      }
      const spawned = engine.api.spawn(
        tool === 'circle' ? 0 : 1,
        p.x,
        p.y,
        tool === 'circle' ? 0.42 : 0.48,
        0.38,
        1,
      );
      if (!spawned) notify('The chamber is full (256 bodies). Delete a body to make room.');
      else selected = spawned;
      sync();
    } else if (tool === 'joint') {
      if (!id) return;
      if (!jointStart) {
        jointStart = id;
        selected = id;
      } else if (id !== jointStart) {
        const a = frame.bodies.find((b) => b.id === jointStart)!;
        const b = frame.bodies.find((b) => b.id === id)!;
        if (a.mass === 0 && b.mass === 0) {
          notify('Connect at least one dynamic body.');
          jointStart = 0;
          return;
        }
        const length = Math.hypot(a.x - b.x, a.y - b.y);
        if (length < 0.01) {
          notify('Move the bodies farther apart before connecting them.');
          return;
        }
        if (frame.joints.length >= 256) {
          notify('Joint limit reached (256).');
          jointStart = 0;
          return;
        }
        engine.api.joint(a.id, b.id, a.x, a.y, b.x, b.y, length);
        jointStart = 0;
        selected = id;
        sync();
        notify('Distance joint created.');
      }
    } else {
      selected = id;
      if (id && tool === 'select') engine.api.drag_start(id, p.x, p.y);
      if (id && tool === 'impulse') impulseBody = id;
      sync();
    }
  }
  function pointerMove(event: PointerEvent) {
    const p = pointerPosition(event);
    pointer = { ...pointer, ...p };
    if (ready && pointer.active && tool === 'select') engine.api.drag_to(p.x, p.y);
  }
  function pointerUp(event: PointerEvent) {
    if (!ready) return;
    if (
      pointer.active &&
      tool === 'impulse' &&
      impulseBody &&
      pointer.startX !== undefined &&
      pointer.startY !== undefined &&
      event.type !== 'pointercancel'
    ) {
      const p = pointerPosition(event);
      engine.api.impulse(
        impulseBody,
        pointer.startX,
        pointer.startY,
        (p.x - pointer.startX) * 5,
        (p.y - pointer.startY) * 5,
      );
      sync();
    }
    engine.api.drag_end();
    pointer.active = false;
    impulseBody = 0;
    if (canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
  }
  function exportScene() {
    if (!ready) return;
    const url = URL.createObjectURL(
      new Blob([engine.exportScene(gravity, iterations)], { type: 'application/json' }),
    );
    const a = document.createElement('a');
    a.href = url;
    a.download = 'poltergeist-scene.json';
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    notify('Scene exported with body motion and joints.');
  }
  async function importScene(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file || !ready) return;
    try {
      if (file.size > 1_000_000) throw new Error('Scene file is too large (maximum 1 MB).');
      const text = await file.text();
      const settings = engine.importScene(text);
      gravity = settings.gravity;
      iterations = settings.iterations;
      initialCustomScene = text;
      customScene = true;
      running = false;
      selected = 0;
      jointStart = 0;
      accumulator = 0;
      pointer.active = false;
      sync();
      notify('Scene loaded. Press play to begin.');
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Unable to load scene.');
    }
    input.value = '';
  }
  function keydown(event: KeyboardEvent) {
    if (
      event.target instanceof HTMLElement &&
      (['INPUT', 'TEXTAREA', 'SELECT', 'BUTTON'].includes(event.target.tagName) ||
        event.target.isContentEditable)
    )
      return;
    if (help) {
      if (event.key === 'Escape') closeHelp();
      return;
    }
    if (event.repeat) return;
    if (event.code === 'Space') {
      event.preventDefault();
      pause();
    } else if (event.key.toLowerCase() === 'n') singleStep();
    else if (event.key.toLowerCase() === 'r') resetScene();
    else if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault();
      deleteBody();
    } else if (event.key === 'Escape') {
      setTool('select');
      selected = 0;
    } else if (event.key === '?') openHelp();
    else {
      const t = tools.find((t) => t.key.toLowerCase() === event.key.toLowerCase());
      if (t) setTool(t.id);
    }
  }
  function openHelp() {
    help = true;
    helpDialog.showModal();
  }
  function closeHelp() {
    help = false;
    helpDialog.close();
  }

  onMount(() => {
    let alive = true;
    const resize = () => {
      const rect = viewport.getBoundingClientRect();
      canvasWidth = Math.max(rect.width, 1);
      canvasHeight = Math.max(rect.height, 1);
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.round(canvasWidth * dpr);
      canvas.height = Math.round(canvasHeight * dpr);
      view = viewFor(canvasWidth, canvasHeight);
    };
    const observer = new ResizeObserver(resize);
    observer.observe(viewport);
    resize();
    const ctx = canvas.getContext('2d')!;
    const loop = (now: number) => {
      if (!alive) return;
      const elapsed = animationTime ? Math.min((now - animationTime) / 1000, 0.1) : 0;
      animationTime = now;
      if (ready && running && !document.hidden && !help) {
        accumulator = Math.min(accumulator + elapsed * speed, 8 / 60);
        const start = performance.now();
        let count = 0;
        while (accumulator >= 1 / 60 && count < 8) {
          engine.api.step(1 / 60);
          accumulator -= 1 / 60;
          count++;
        }
        if (count) {
          simMs = simMs * 0.8 + ((performance.now() - start) / count) * 0.2;
        }
      } else accumulator = 0;
      const current = ready ? engine.read() : emptyFrame();
      if (now - refreshTime > 80) {
        frame = current;
        refreshTime = now;
        if (elapsed > 0) fps = Math.round(fps * 0.85 + (1 / elapsed) * 0.15);
      }
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      render(ctx, current, view, selected, overlays, tool, pointer, jointStart);
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    Engine.load()
      .then((value) => {
        if (!alive) return;
        engine = value;
        ready = true;
        loadScene(0);
      })
      .catch((error) => {
        fatal = error.message;
      });
    const visibility = () => {
      accumulator = 0;
      animationTime = 0;
      if (ready) engine.api.drag_end();
      pointer.active = false;
    };
    window.addEventListener('keydown', keydown);
    document.addEventListener('visibilitychange', visibility);
    window.addEventListener('blur', visibility);
    return () => {
      alive = false;
      cancelAnimationFrame(raf);
      observer.disconnect();
      clearTimeout(toastTimer);
      window.removeEventListener('keydown', keydown);
      document.removeEventListener('visibilitychange', visibility);
      window.removeEventListener('blur', visibility);
    };
  });
</script>

<div class="app-shell">
  <header class="topbar">
    <a class="brand" href="/" aria-label="Poltergeist home">
      <svg class="ghost" width="28" height="30" viewBox="0 0 32 34" fill="none" aria-hidden="true"
        ><path
          d="M5 29V14a11 11 0 0 1 22 0v15l-5.5-3.5L16 29l-5.5-3.5L5 29Z"
          stroke="currentColor"
          stroke-width="1.8"
        /><path
          d="M12 13v3m8-3v3"
          stroke="currentColor"
          stroke-width="2.4"
          stroke-linecap="round"
        /></svg
      >
      <span>poltergeist<span class="brand-dot">.</span></span>
    </a>
    <span class="topbar-rule"></span><span class="topbar-caption">AN INTERACTIVE PHYSICS LAB</span>
    <div class="topbar-actions">
      <span class="build-label">EXPERIMENTAL / V0.1</span>
      <button class="text-button" onclick={exportScene} disabled={!ready} aria-label="Export scene"
        ><Icon name="download" size={15} /> <span>Export scene</span></button
      >
      <button
        class="icon-button"
        onclick={() => fileInput.click()}
        disabled={!ready}
        title="Import scene"
        aria-label="Import scene"><Icon name="upload" size={17} /></button
      >
      <button
        class="icon-button"
        onclick={openHelp}
        title="Help & shortcuts"
        aria-label="Help & shortcuts"><Icon name="help" size={18} /></button
      >
    </div>
  </header>

  <main class="lab-layout">
    <aside class="left-panel">
      <div class="intro">
        <div class="eyebrow"><span class="tiny-dot"></span> THE SANDBOX</div>
        <h1>Physics, under<br /><span>observation.</span></h1>
        <p>Build a little chaos.<br />See what holds it together.</p>
      </div>
      <section class="panel-section scene-section">
        <div class="section-label">EXPERIMENTS <span>04</span></div>
        <div class="scene-list">
          {#each scenes as item, index}<button
              class:active={scene === index && !customScene}
              class="scene-button"
              onclick={() => loadScene(index)}
              disabled={!ready}
              aria-pressed={scene === index && !customScene}
              ><span class="scene-number">{item.number}</span><span class="scene-copy"
                ><strong>{item.name}</strong><small>{item.sub}</small></span
              ><Icon name="chevron" size={13} /></button
            >{/each}
        </div>
      </section>
      <section class="panel-section">
        <div class="section-label">TOOLBOX <span>CLICK TO PLACE</span></div>
        <div class="tool-list">
          {#each tools as item}<button
              class:active={tool === item.id}
              class="tool-button"
              aria-label={item.label}
              onclick={() => setTool(item.id)}
              disabled={!ready}
              aria-pressed={tool === item.id}
              ><Icon name={item.icon} size={17} /><span>{item.label}</span><kbd>{item.key}</kbd
              ></button
            >{/each}
        </div>
      </section>
      <section class="panel-section overlay-section">
        <div class="section-label">REVEAL THE INVISIBLE <Icon name="eye" size={14} /></div>
        {#each Object.keys(overlayLabels) as key}{@const k = key as keyof Overlays}<label
            class="toggle-row"
            ><span class="overlay-dot {k}"></span><span>{overlayLabels[k]}</span><input
              type="checkbox"
              bind:checked={overlays[k]}
              aria-label={overlayLabels[k]}
            /><span class="toggle-track"></span></label
          >{/each}
      </section>
      <div class="left-footer">
        <span class="engine-dot"></span><span>RUST ENGINE · WEBASSEMBLY</span><span
          class="footer-arrow">↗</span
        >
      </div>
    </aside>

    <section class="chamber" aria-label="Physics sandbox">
      <div class="chamber-header">
        <div>
          <span class="eyebrow"
            >LIVE EXPERIMENT <span class="slash">/</span>
            {customScene ? 'CUSTOM' : scenes[scene].number}</span
          >
          <h2>{currentTitle}</h2>
        </div>
        <span class="status" class:paused={!running || help}
          ><span></span>{running && !help ? 'SIMULATING' : 'PAUSED'}</span
        >
      </div>
      <div class="viewport" bind:this={viewport}>
        <canvas
          bind:this={canvas}
          class:placing={tool === 'circle' || tool === 'box'}
          class:impulsing={tool === 'impulse'}
          aria-label="Interactive physics chamber. Use toolbox buttons to place bodies. Select a body in the Objects list to inspect it."
          onpointerdown={pointerDown}
          onpointermove={pointerMove}
          onpointerup={pointerUp}
          onpointercancel={pointerUp}
        ></canvas>
        <div class="viewport-meta"><span>WORLD SPACE</span><span>METERS / SECONDS</span></div>
        <div class="viewport-legend">
          <span><i class="body-color"></i>BODY</span>{#if overlays.contacts}<span
              ><i class="contact-color"></i>CONTACT</span
            >{/if}{#if overlays.impulses}<span><i class="impulse-color"></i>IMPULSE</span>{/if}
        </div>
        {#if !ready}<div class="loading-state" role="status">
            <span class="loader"></span>
            <h3>{fatal ? 'Engine unavailable' : 'Waking the ghost'}</h3>
            <p>{fatal || 'Loading the physics engine…'}</p>
            {#if fatal}<button class="primary-button" onclick={() => location.reload()}
                >Try again</button
              >{/if}
          </div>{/if}
        {#if ready && scene === 0 && !customScene && !nudged}<div class="experiment-note">
            <span>TRY THIS</span>
            <p>Give the sphere a push.<br />Watch the tower answer.</p>
            <button onclick={kick}>Apply a nudge <Icon name="arrow" size={14} /></button>
          </div>{/if}
        {#if toast}<div class="toast" role="status">{toast}</div>{/if}
      </div>
      <div class="transport">
        <div class="playback">
          <button
            class="play-button"
            onclick={pause}
            disabled={!ready}
            aria-label={running ? 'Pause simulation' : 'Play simulation'}
            title="Play / pause · Space"
            ><Icon name={running ? 'pause' : 'play'} size={18} /></button
          ><button
            class="icon-button"
            onclick={singleStep}
            disabled={!ready}
            title="Advance one tick · N"
            aria-label="Advance one tick"><Icon name="step" size={18} /></button
          ><span class="control-divider"></span><button
            class="icon-button"
            onclick={resetScene}
            disabled={!ready}
            title="Reset experiment · R"
            aria-label="Reset experiment"><Icon name="reset" size={17} /></button
          >
        </div>
        <div class="time-readout">
          <span>ELAPSED</span><strong data-testid="elapsed"
            >{fmt(frame.time)}<small>s</small></strong
          >
        </div>
        <div class="speed-control">
          <label for="speed">SPEED</label><select id="speed" bind:value={speed}
            ><option value={0.25}>0.25×</option><option value={0.5}>0.5×</option><option value={1}
              >1×</option
            ><option value={2}>2×</option></select
          >
        </div>
        <div class="transport-hint"><kbd>SPACE</kbd> to {running ? 'pause' : 'play'}</div>
      </div>
      <div class="chamber-hint">
        <Icon name="crosshair" size={14} /><span>{toolHint}</span><span class="tick-label"
          >TICK {String(frame.tick).padStart(5, '0')}</span
        >
      </div>
    </section>

    <aside class="right-panel">
      <section class="panel-section inspector">
        <div class="section-label">BODY INSPECTOR <span class="live-badge">LIVE</span></div>
        {#if body}
          <div class="selected-heading">
            <div class="selected-icon">
              <Icon name={body.kind === 0 ? 'circle' : 'box'} size={23} />
            </div>
            <div>
              <h3>
                {body.kind === 0 ? 'Circle' : 'Box'}
                <span>#{String(body.id).padStart(2, '0')}</span>
              </h3>
              <small>{body.mass === 0 ? 'STATIC BODY' : 'DYNAMIC BODY'}</small>
            </div>
            <button
              class="icon-button delete-button"
              onclick={deleteBody}
              disabled={body.mass === 0}
              aria-label="Delete selected body"
              title="Delete body"><Icon name="trash" size={15} /></button
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
          <div class="data-row">
            <span>Rotation</span><strong
              >{fmt((body.angle * 180) / Math.PI, 1)}<small>°</small></strong
            >
          </div>
          <div class="data-row">
            <span>Angular velocity</span><strong>{fmt(body.omega)}<small>rad/s</small></strong>
          </div>
          <div class="body-properties">
            {#if body.mass > 0}<label class="slider-label" for="mass"
                >Mass <span>{fmt(body.mass, 1)} <small>kg</small></span></label
              ><input
                id="mass"
                type="range"
                min=".2"
                max="10"
                step=".1"
                value={body.mass}
                oninput={(e) => updateBody('mass', +e.currentTarget.value)}
              />{/if}
            <label class="slider-label" for="friction"
              >Friction <span>{fmt(body.friction)}</span></label
            ><input
              id="friction"
              type="range"
              min="0"
              max="1.5"
              step=".01"
              value={body.friction}
              oninput={(e) => updateBody('friction', +e.currentTarget.value)}
            />
            <label class="slider-label" for="bounce"
              >Restitution <span>{fmt(body.restitution)}</span></label
            ><input
              id="bounce"
              type="range"
              min="0"
              max="1"
              step=".01"
              value={body.restitution}
              oninput={(e) => updateBody('restitution', +e.currentTarget.value)}
            />
          </div>
          <div class="contact-summary">
            <span class="contact-color-dot"></span><span
              >{bodyContacts.length} active contact{bodyContacts.length === 1 ? '' : 's'}</span
            ><strong
              >{fmt(
                bodyContacts.reduce((s, c) => s + c.normalImpulse, 0),
                3,
              )}<small>N·s</small></strong
            >
          </div>
          <p class="microcopy">Normal impulses from the latest solver substep.</p>
        {:else}
          <div class="empty-inspector">
            <div class="inspector-illustration"><span></span><Icon name="select" size={24} /></div>
            <h3>Look a little closer.</h3>
            <p>Select any body to inspect its motion, material, and contacts.</p>
            <span class="empty-tag">EVERY COLLISION TELLS A STORY</span>
          </div>
        {/if}
      </section>
      <section class="panel-section settings">
        <div class="section-label">WORLD SETTINGS</div>
        <label class="slider-label" for="gravity"
          >Gravity <span>{fmt(gravity)} <small>m/s²</small></span></label
        ><input
          id="gravity"
          type="range"
          min="0"
          max="20"
          step=".01"
          value={gravity}
          oninput={(e) => {
            gravity = +e.currentTarget.value;
            configure();
          }}
        />
        <label class="slider-label" for="iterations"
          >Solver iterations <span>{iterations}</span></label
        ><input
          id="iterations"
          type="range"
          min="2"
          max="24"
          step="1"
          value={iterations}
          oninput={(e) => {
            iterations = +e.currentTarget.value;
            configure();
          }}
        />
        <div class="data-row muted">
          <span>Fixed timestep</span><strong>1/60 <small>s</small></strong>
        </div>
        <div class="data-row muted"><span>Substeps</span><strong>2</strong></div>
      </section>
      <section class="panel-section objects-section">
        <div class="section-label">OBJECTS <span>{frame.bodies.length}</span></div>
        <div class="object-list">
          {#each frame.bodies as object}<button
              class:active={selected === object.id}
              class="object-button"
              onclick={() => {
                selected = object.id;
                setTool('select');
              }}
              aria-label={`Inspect ${object.kind === 0 ? 'circle' : 'box'} ${object.id}`}
              ><Icon name={object.kind === 0 ? 'circle' : 'box'} size={12} /><span
                >{object.kind === 0 ? 'Circle' : 'Box'} #{String(object.id).padStart(2, '0')}</span
              ><small>{object.mass === 0 ? 'STATIC' : `${fmt(object.mass, 1)} kg`}</small></button
            >{/each}
        </div>
      </section>
    </aside>
  </main>

  <footer class="statusbar">
    <div class="status-metrics">
      <span><i class="engine-dot"></i>{fps} <small>FPS</small></span><span
        >{dynamicCount}<small>BODIES</small></span
      ><span>{frame.contacts.length}<small>CONTACTS</small></span><span
        >{frame.joints.length}<small>JOINTS</small></span
      ><span>{fmt(simMs)}<small>ms / TICK</small></span>
    </div>
    <span class="status-message">A LITTLE CHAOS. A LOT OF PHYSICS.</span><span
      class="status-version">LOCAL · CLIENT-SIDE</span
    >
  </footer>
</div>
<input
  class="visually-hidden"
  type="file"
  accept=".json,application/json"
  bind:this={fileInput}
  onchange={importScene}
  aria-label="Choose scene file"
/>
<dialog bind:this={helpDialog} oncancel={() => (help = false)} class="help-dialog">
  <div class="dialog-header">
    <span class="eyebrow">FIELD GUIDE</span><button
      class="icon-button"
      onclick={closeHelp}
      aria-label="Close help"><Icon name="close" /></button
    >
  </div>
  <h2>Make something move.</h2>
  <p>
    Create bodies, connect them, and inspect what happens. Dragging pulls through a spring; the
    impulse tool applies a push when you release.
  </p>
  <div class="shortcut-grid">
    {#each [...tools.map( (t) => ({ key: t.key, label: t.label }) ), { key: 'SPACE', label: 'Play / pause' }, { key: 'N', label: 'Advance one tick' }, { key: 'R', label: 'Reset experiment' }, { key: 'DEL', label: 'Delete selected body' }, { key: 'ESC', label: 'Clear selection' }] as item}<div
      >
        <span>{item.label}</span><kbd>{item.key}</kbd>
      </div>{/each}
  </div>
  <p class="microcopy">
    This first version uses discrete collisions and a compact sequential impulse solver. Very fast
    bodies may tunnel; sleeping and continuous collision detection are future work. Scene exports
    save the current body state, settings, and joints.
  </p>
  <button class="primary-button" onclick={closeHelp}
    >Back to the experiment <Icon name="arrow" size={16} /></button
  >
</dialog>
