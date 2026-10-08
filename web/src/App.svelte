<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import Slider from './Slider.svelte';
  import Inspector from './Inspector.svelte';
  import type { InspectorActions } from './Inspector.svelte';
  import EnergyChart from './EnergyChart.svelte';
  import type { EnergySample } from './EnergyChart.svelte';
  import {
    BOX,
    CIRCLE,
    POLYGON,
    Engine,
    MAX_SIDES,
    MIN_SIDES,
    bodyBounds,
    emptyFrame,
    jointKinds,
    massFor,
    shapeName,
    toLocal,
    toWorld,
  } from './engine';
  import type { Body, Frame, Joint, JointKind, SceneDoc, Settings, Tool } from './engine';
  import { render } from './renderer';
  import type { Ghost, Overlays, Segment } from './renderer';
  import { homeCamera, panBy, screenToWorld, viewFor, zoomAt } from './view';
  import type { Camera } from './view';
  import { UndoStack } from './undo';
  import type { Checkpoint } from './undo';
  import { loadPrefs, savePrefs, speeds } from './prefs';

  const scenes = [
    {
      name: 'Tower collapse',
      sub: 'Contacts & stability',
      description:
        'Twenty-one blocks. One small disturbance. Observe how a structure loses its balance.',
    },
    {
      name: 'Pendulum garden',
      sub: 'Distance constraints',
      description:
        'Five suspended bodies. Pull one aside and follow the impulses that hold its orbit.',
    },
    {
      name: 'Friction study',
      sub: 'Three surfaces, three outcomes',
      description:
        'Identical blocks on identical slopes. Only friction changes: 0.02, 0.20, and 0.90.',
    },
    {
      name: 'Bounce chamber',
      sub: 'Restitution & motion',
      description:
        'A collection of elastic bodies. Watch energy move between translation and rotation.',
    },
    {
      name: 'Domino run',
      sub: 'Momentum in sequence',
      description:
        'A ball rolls down a ramp into eighteen thin dominoes. Follow the impulse down the line.',
    },
    {
      name: 'Spring lattices',
      sub: 'Soft constraints',
      description:
        'Identical lattices held together by 3.5 Hz and 8 Hz springs. A weight lands on each.',
    },
    {
      name: 'Mechanisms',
      sub: 'Pins, motors & ropes',
      description:
        'A motor-driven paddle, a see-saw catapult, a rope pendulum, and a hanging spring.',
    },
    {
      name: 'Empty chamber',
      sub: 'Build your own',
      description: 'Floor, walls, nothing else. Place bodies, connect them, and see what happens.',
    },
  ];
  const tools: { id: Tool; label: string; key: string; icon: string }[] = [
    { id: 'select', label: 'Select & drag', key: 'V', icon: 'select' },
    { id: 'circle', label: 'Add circle', key: 'C', icon: 'circle' },
    { id: 'box', label: 'Add box', key: 'B', icon: 'box' },
    { id: 'polygon', label: 'Add polygon', key: 'P', icon: 'polygon' },
    { id: 'impulse', label: 'Apply impulse', key: 'I', icon: 'impulse' },
    { id: 'joint', label: 'Connect bodies', key: 'J', icon: 'joint' },
  ];
  const jointInfo: Record<JointKind, { label: string; hint: string }> = {
    rod: { label: 'Rod', hint: 'Holds two anchors at a fixed distance' },
    rope: { label: 'Rope', hint: 'Limits how far apart two anchors can get' },
    spring: { label: 'Spring', hint: 'Pulls back toward its rest length' },
    pin: { label: 'Pin', hint: 'Fixes a point; rotation stays free' },
  };
  const overlayLabels: Record<keyof Overlays, string> = {
    contacts: 'Contacts & normals',
    impulses: 'Contact impulses',
    velocities: 'Velocity vectors',
    trails: 'Motion trails',
    bounds: 'Collision bounds',
    sleep: 'Sleeping bodies',
    grid: 'Reference grid',
  };
  const shapeTools: Tool[] = ['circle', 'box', 'polygon'];
  const mod =
    typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform) ? '⌘' : 'Ctrl+';
  const shortcuts = [
    ...tools.map((t) => ({ key: t.key, label: t.label })),
    { key: 'SPACE', label: 'Play / pause' },
    { key: 'N  →', label: 'Step forward' },
    { key: '←', label: 'Step back in time' },
    { key: 'R', label: 'Reset experiment' },
    { key: `${mod}Z`, label: 'Undo' },
    { key: `⇧${mod}Z`, label: 'Redo' },
    { key: `${mod}D`, label: 'Duplicate body' },
    { key: 'DEL', label: 'Delete body' },
    { key: 'Q  E', label: 'Rotate body 15°' },
    { key: 'S', label: 'Static placement' },
    { key: '+  −', label: 'Zoom in / out' },
    { key: '0', label: 'Reset view' },
    { key: 'F', label: 'Follow selection' },
    { key: 'ESC', label: 'Cancel / deselect' },
  ];
  const DT = 1 / 60;
  /** Placement stays within reach of the laboratory; the engine culls beyond 300 m. */
  const PLACE_LIMIT = 250;

  type Point = { x: number; y: number };
  type Gesture =
    | { kind: 'none' }
    | { kind: 'spring'; id: number }
    | {
        kind: 'move';
        id: number;
        dx: number;
        dy: number;
        sx: number;
        sy: number;
        before: Checkpoint;
        moved: boolean;
      }
    | {
        kind: 'pan';
        sx: number;
        sy: number;
        lastX: number;
        lastY: number;
        moved: boolean;
        deselect: boolean;
      }
    | { kind: 'place'; x: number; y: number; sx: number; sy: number }
    | { kind: 'impulse'; id: number; lx: number; ly: number }
    | { kind: 'link'; sx: number; sy: number }
    | { kind: 'pinch'; distance: number; mid: Point; camera: Camera };

  const prefs = loadPrefs({
    overlays: {
      contacts: true,
      impulses: false,
      velocities: false,
      trails: false,
      bounds: false,
      sleep: false,
      grid: true,
    },
    speed: 1,
    placeStatic: false,
    sides: 5,
    jointKind: 'rod',
  });

  let engine: Engine;
  let canvas: HTMLCanvasElement;
  let viewport: HTMLDivElement;
  let fileInput: HTMLInputElement;
  let helpDialog: HTMLDialogElement;
  let ready = $state(false);
  let fatal = $state('');
  let frame: Frame = $state.raw(emptyFrame());
  let running = $state(true);
  let scene = $state(0);
  let customScene = $state(false);
  let nudged = $state(false);
  let tool: Tool = $state('select');
  let placeStatic = $state(prefs.placeStatic);
  let sides = $state(prefs.sides);
  let jointKind: JointKind = $state(prefs.jointKind);
  let selected = $state(0);
  let settings: Settings = $state({ gravity: 9.81, iterations: 12, sleeping: true });
  let speed = $state(prefs.speed);
  let simMs = $state(0);
  let fps = $state(60);
  let help = $state(false);
  let toast = $state('');
  let overlays: Overlays = $state(prefs.overlays);
  let zoomPercent = $state(100);
  let follow = $state(false);
  let canUndo = $state(false);
  let canRedo = $state(false);
  /** First anchor of a joint being made: a body-local point, or a world point when id is 0. */
  let pendingLink: { id: number; local: Point } | null = $state(null);
  let energyVersion = $state(0);
  const energy: EnergySample[] = [];
  const trails = new Map<number, number[]>();
  let toastTimer: ReturnType<typeof setTimeout>;
  let camera: Camera = homeCamera();
  let view = viewFor(800, 600, camera);
  let canvasWidth = 800;
  let canvasHeight = 600;
  let pointer = { x: -4, y: 4, sx: 0, sy: 0, inside: false, shift: false };
  let hovered = 0;
  let gesture: Gesture = { kind: 'none' };
  const touches = new Map<number, Point>();
  let raf = 0;
  let animationTime = 0;
  let accumulator = 0;
  let refreshTime = 0;
  let lastTick = -1;
  let lastCulled = 0;
  let energyDirty = false;
  let spaceHandled = false;
  /** Whether the focused control was reached by pointer (click/tap) rather than keyboard. */
  let lastInput: 'pointer' | 'keyboard' = 'pointer';
  let focusByPointer = true;
  let initialCustomScene = '';
  const undoStack = new UndoStack();

  const body = $derived(frame.bodies.find((b) => b.id === selected));
  const bodyContacts = $derived(frame.contacts.filter((c) => c.a === selected || c.b === selected));
  const bodyJoints = $derived(frame.joints.filter((j) => j.a === selected || j.b === selected));
  const dynamicCount = $derived(frame.bodies.filter((b) => b.mass > 0).length);
  const rewound = $derived(
    frame.historyLength > 1 && frame.historyCursor < frame.historyLength - 1,
  );
  const rewindSeconds = $derived((frame.historyLength - 1 - frame.historyCursor) * DT);
  const currentTitle = $derived(customScene ? 'Custom experiment' : scenes[scene].name);
  const currentDescription = $derived(
    customScene
      ? 'An imported or edited experiment. Reset restores how it began.'
      : scenes[scene].description,
  );
  const toolHint = $derived.by(() => {
    if (tool === 'select')
      return running
        ? 'Click a body to inspect it; drag to pull it. Drag empty space to pan, scroll to zoom.'
        : 'Paused: drag bodies to rearrange them. Drag empty space to pan, scroll to zoom.';
    if (tool === 'impulse')
      return 'Drag from a body toward where it should go. Longer arrows push harder.';
    if (tool === 'joint') {
      if (jointKind === 'pin')
        return 'Click a body to pin it to whatever is behind it, or to the world.';
      return pendingLink
        ? 'Click a second body, or empty space to anchor to the world. Esc cancels.'
        : `Click two bodies (or a body and empty space) to add a ${jointKind}.`;
    }
    const name = tool === 'polygon' ? `${sides}-sided polygon` : tool;
    return `Click to drop a ${placeStatic ? 'static ' : ''}${name}, or drag to size it.${tool === 'box' ? ' Hold Shift for a square.' : ''}`;
  });
  const fmt = (n: number, digits = 2) => (Number.isFinite(n) ? n.toFixed(digits) : '—');

  $effect(() => {
    savePrefs({ overlays: { ...overlays }, speed, placeStatic, sides, jointKind });
  });

  function notify(message: string) {
    toast = message;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ''), 4000);
  }
  function sync() {
    if (ready) frame = engine.read();
  }
  function liveBody(id: number): Body | undefined {
    return engine.read().bodies.find((b) => b.id === id);
  }

  // Undo ---------------------------------------------------------------------------------
  function checkpoint(): Checkpoint {
    const f = engine.read();
    return {
      scene: JSON.stringify({ ...engine.snapshot(), time: f.time, tick: f.tick }),
      sceneIndex: scene,
      custom: customScene,
      initial: initialCustomScene,
    };
  }
  function record(label: string, key = '', before: Checkpoint | (() => Checkpoint) = checkpoint) {
    undoStack.push(before, label, key);
    canUndo = undoStack.canUndo;
    canRedo = undoStack.canRedo;
  }
  function restore(c: Checkpoint): boolean {
    const doc = JSON.parse(c.scene) as SceneDoc & { time: number; tick: number };
    endGesture();
    pendingLink = null;
    try {
      engine.load(doc, settings);
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Unable to restore that step.');
      return false;
    }
    engine.api.clock(doc.time, doc.tick);
    scene = c.sceneIndex;
    customScene = c.custom;
    initialCustomScene = c.initial;
    accumulator = 0;
    sync();
    if (!frame.bodies.some((b) => b.id === selected)) selected = 0;
    return true;
  }
  function undo() {
    if (!ready) return;
    const entry = undoStack.undo(checkpoint());
    if (entry && restore(entry)) notify(`Undid: ${entry.label}.`);
    canUndo = undoStack.canUndo;
    canRedo = undoStack.canRedo;
  }
  function redo() {
    if (!ready) return;
    const entry = undoStack.redo(checkpoint());
    if (entry && restore(entry)) notify(`Redid: ${entry.label}.`);
    canUndo = undoStack.canUndo;
    canRedo = undoStack.canRedo;
  }

  // Scenes and time ------------------------------------------------------------------------
  function loadScene(index: number, remember = true) {
    if (!ready) return;
    const before = remember ? checkpoint() : null;
    endGesture();
    engine.api.reset(index);
    engine.configure(settings);
    scene = index;
    customScene = false;
    nudged = false;
    initialCustomScene = '';
    selected = 0;
    pendingLink = null;
    follow = false;
    accumulator = 0;
    sync();
    if (before) record(`Load ${scenes[index].name}`, '', before);
  }
  function resetScene() {
    if (!ready) return;
    if (customScene && initialCustomScene) {
      const before = checkpoint();
      endGesture();
      settings = engine.importScene(initialCustomScene);
      selected = 0;
      pendingLink = null;
      accumulator = 0;
      sync();
      record('Reset experiment', '', before);
    } else loadScene(scene);
  }
  function pause() {
    if (!ready) return;
    running = !running;
    accumulator = 0;
    if (gesture.kind === 'spring') endGesture();
    sync();
  }
  function seek(index: number) {
    if (gesture.kind === 'spring') endGesture();
    engine.api.history_seek(index);
    sync();
  }
  function stepForward() {
    if (!ready) return;
    running = false;
    accumulator = 0;
    const f = engine.read();
    if (f.historyCursor < f.historyLength - 1) seek(f.historyCursor + 1);
    else {
      engine.api.step(DT);
      sync();
    }
  }
  function stepBack() {
    if (!ready) return;
    running = false;
    accumulator = 0;
    const f = engine.read();
    if (f.historyCursor > 0) seek(f.historyCursor - 1);
    else notify('That is the start of the recorded timeline (up to 10 seconds are kept).');
  }
  function configure(change: Partial<Settings>) {
    if (!ready) return;
    settings = { ...settings, ...change };
    engine.configure(settings);
    sync();
  }

  // Tools ----------------------------------------------------------------------------------
  function endGesture() {
    if (gesture.kind === 'spring' && ready) engine.api.drag_end();
    gesture = { kind: 'none' };
  }
  function setTool(next: Tool) {
    if (next === 'joint' && tool === 'joint') {
      // Pressing J again cycles through connection types.
      setJointKind(jointKinds[(jointKinds.indexOf(jointKind) + 1) % jointKinds.length]);
      return;
    }
    tool = next;
    pendingLink = null;
    endGesture();
  }
  function setJointKind(kind: JointKind) {
    jointKind = kind;
    pendingLink = null;
  }
  function ghostFor(start: Point, end: Point, dragged: boolean, square: boolean): Ghost {
    const kind = tool === 'circle' ? CIRCLE : tool === 'box' ? BOX : POLYGON;
    const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
    const reach = Math.hypot(end.x - start.x, end.y - start.y);
    if (kind === BOX) {
      if (!dragged) return { kind, x: start.x, y: start.y, a: 0.48, b: 0.38, fixed: placeStatic };
      let a = clamp(Math.abs(end.x - start.x) / 2, 0.05, 10),
        b = clamp(Math.abs(end.y - start.y) / 2, 0.05, 10);
      if (square) a = b = Math.max(a, b);
      const x = start.x + Math.sign(end.x - start.x || 1) * a,
        y = start.y + Math.sign(end.y - start.y || 1) * b;
      return { kind, x, y, a, b, fixed: placeStatic };
    }
    const fallback = kind === CIRCLE ? 0.42 : 0.5;
    const a = dragged ? clamp(reach, kind === CIRCLE ? 0.05 : 0.08, 5) : fallback;
    return { kind, x: start.x, y: start.y, a, b: kind === CIRCLE ? a : sides, fixed: placeStatic };
  }
  function placeBody(g: Ghost) {
    if (Math.abs(g.x) > PLACE_LIMIT || Math.abs(g.y) > PLACE_LIMIT) {
      notify('Place bodies within 250 m of the laboratory.');
      return;
    }
    const before = checkpoint();
    const mass = g.fixed ? 0 : massFor(g.kind, g.a, g.b);
    const id = engine.api.spawn(0, g.kind, g.x, g.y, g.a, g.b, mass);
    if (!id) {
      notify('The chamber is full (256 bodies). Delete a body to make room.');
      return;
    }
    record(`Add ${shapeName(g).toLowerCase()}`, '', before);
    selected = id;
    sync();
  }
  function applyImpulse(g: Extract<Gesture, { kind: 'impulse' }>, p: Point) {
    const b = liveBody(g.id);
    if (!b) return;
    const anchor = toWorld(b, g.lx, g.ly);
    const jx = (p.x - anchor.x) * 5,
      jy = (p.y - anchor.y) * 5;
    if (Math.hypot(jx, jy) < 1e-3) return;
    engine.api.impulse(g.id, anchor.x, anchor.y, jx, jy);
    sync();
  }
  function connect(p: Point, id: number) {
    const f = engine.read();
    const find = (bid: number) => f.bodies.find((b) => b.id === bid);
    if (jointKind === 'pin') {
      const target = find(id);
      if (!target) {
        notify('Click a body to pin it.');
        return;
      }
      const below = engine.api.pick_below(p.x, p.y, id);
      if (target.mass === 0 && (find(below)?.mass ?? 0) === 0) {
        notify('Pin a dynamic body, or pin something onto one.');
        return;
      }
      const before = checkpoint();
      if (!engine.api.joint(0, 3, id, below, p.x, p.y, p.x, p.y, 0, 0, 0, 0)) {
        notify('That pin is not possible.');
        return;
      }
      record('Add pin', '', before);
      selected = id;
      notify(below ? `Pinned #${id} to #${below}.` : `Pinned #${id} to the world.`);
      sync();
      return;
    }
    if (!pendingLink) {
      const b = find(id);
      pendingLink = b ? { id, local: toLocal(b, p.x, p.y) } : { id: 0, local: p };
      if (b) selected = id;
      return;
    }
    const first = pendingLink;
    const a = find(first.id);
    if (first.id && !a) {
      pendingLink = null;
      return;
    }
    if (id === first.id) {
      notify(id ? 'Choose a different second body.' : 'Connect at least one body.');
      return;
    }
    const b = find(id);
    if ((a?.mass ?? 0) === 0 && (b?.mass ?? 0) === 0) {
      notify('Connect at least one dynamic body.');
      pendingLink = null;
      return;
    }
    const pa = a ? toWorld(a, first.local.x, first.local.y) : first.local;
    if (Math.hypot(p.x - pa.x, p.y - pa.y) < 0.02) {
      notify('Move the anchors farther apart before connecting them.');
      return;
    }
    if (f.joints.length >= 256) {
      notify('Joint limit reached (256).');
      pendingLink = null;
      return;
    }
    const [p1, p2] = jointKind === 'spring' ? [2.5, 0.3] : [0, 0];
    const before = checkpoint();
    const kind = jointKinds.indexOf(jointKind);
    const made = engine.api.joint(0, kind, first.id, id, pa.x, pa.y, p.x, p.y, 0, p1, p2, 0);
    pendingLink = null;
    if (!made) {
      notify('That connection is not possible.');
      return;
    }
    record(`Add ${jointKind}`, '', before);
    selected = id || first.id;
    notify(
      `${jointInfo[jointKind].label} created${id && first.id ? '' : ' (anchored to the world)'}.`,
    );
    sync();
  }

  // Body edits -----------------------------------------------------------------------------
  function deleteBody() {
    const b = body && liveBody(body.id);
    if (!b) return;
    record(`Delete ${shapeName(b).toLowerCase()}`);
    engine.api.remove(b.id);
    selected = 0;
    pendingLink = null;
    sync();
  }
  function duplicate() {
    const b = body && liveBody(body.id);
    if (!b) return;
    const bounds = bodyBounds(b);
    const before = checkpoint();
    const id = engine.api.spawn(
      0,
      b.kind,
      b.x + bounds.maxX - bounds.minX + 0.15,
      b.y,
      b.a,
      b.b,
      b.mass,
    );
    if (!id) {
      notify('The chamber is full (256 bodies).');
      return;
    }
    engine.api.motion(id, b.angle, 0, 0, 0);
    engine.api.properties(id, b.mass, b.friction, b.restitution);
    record(`Duplicate ${shapeName(b).toLowerCase()}`, '', before);
    selected = id;
    sync();
  }
  function rotateTo(degrees: number, key = 'rotate') {
    const b = body && liveBody(body.id);
    if (!b) return;
    record('Rotate body', `${key}:${b.id}`);
    engine.api.motion(b.id, (degrees * Math.PI) / 180, b.vx, b.vy, b.omega);
    sync();
  }
  const inspector: InspectorActions = {
    property(key, value) {
      const b = body && liveBody(body.id);
      if (!b) return;
      record(`Edit ${key}`, `prop:${b.id}:${key}`);
      engine.api.properties(
        b.id,
        key === 'mass' ? value : b.mass,
        key === 'friction' ? value : b.friction,
        key === 'restitution' ? value : b.restitution,
      );
      sync();
    },
    setStatic(fixed) {
      const b = body && liveBody(body.id);
      if (!b) return;
      record(fixed ? 'Make static' : 'Make dynamic');
      engine.api.properties(b.id, fixed ? 0 : massFor(b.kind, b.a, b.b), b.friction, b.restitution);
      sync();
    },
    rotate: (degrees) => rotateTo(degrees),
    duplicate,
    remove: deleteBody,
    updateJoint(j, change) {
      record('Edit constraint', `joint:${j.id}:${Object.keys(change).join()}`);
      engine.api.joint_set(j.id, change.length ?? j.length, change.p1 ?? j.p1, change.p2 ?? j.p2);
      sync();
    },
    removeJoint(j: Joint) {
      record(`Remove ${j.kind}`);
      engine.api.joint_remove(j.id);
      sync();
    },
    select(id) {
      selected = id;
    },
  };
  function kick() {
    if (!ready) return;
    const f = engine.read();
    const target =
      f.bodies.find((b) => b.mass > 0 && b.kind === CIRCLE) ?? f.bodies.find((b) => b.mass > 0);
    if (target) {
      engine.api.impulse(target.id, target.x, target.y, 16, 3);
      selected = target.id;
      nudged = true;
      running = true;
      sync();
      notify('Impulse applied: 16 N·s →, 3 N·s ↑');
    }
  }

  // Camera ---------------------------------------------------------------------------------
  function updateView() {
    view = viewFor(canvasWidth, canvasHeight, camera);
    zoomPercent = Math.round(camera.zoom * 100);
  }
  function zoomBy(factor: number) {
    camera = zoomAt(camera, view, canvasWidth / 2, canvasHeight / 2, factor);
    updateView();
  }
  function resetView() {
    camera = homeCamera();
    follow = false;
    updateView();
  }
  function toggleFollow() {
    if (!follow && !selected) {
      notify('Select a body to follow it.');
      return;
    }
    follow = !follow;
  }

  // Pointer input --------------------------------------------------------------------------
  function screenPoint(event: MouseEvent): Point {
    const rect = canvas.getBoundingClientRect();
    return {
      x: ((event.clientX - rect.left) * canvasWidth) / rect.width,
      y: ((event.clientY - rect.top) * canvasHeight) / rect.height,
    };
  }
  function updateCursor() {
    const dragging = gesture.kind === 'pan' || gesture.kind === 'spring' || gesture.kind === 'move';
    canvas.style.cursor = dragging
      ? 'grabbing'
      : tool === 'select'
        ? hovered
          ? 'grab'
          : 'default'
        : (tool === 'impulse' || tool === 'joint') && hovered
          ? 'pointer'
          : 'crosshair';
  }
  function startPinch() {
    endGesture();
    const [a, b] = [...touches.values()];
    gesture = {
      kind: 'pinch',
      distance: Math.max(1, Math.hypot(a.x - b.x, a.y - b.y)),
      mid: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 },
      camera: { ...camera },
    };
  }
  function pointerDown(event: PointerEvent) {
    if (!ready) return;
    const s = screenPoint(event);
    touches.set(event.pointerId, s);
    canvas.setPointerCapture(event.pointerId);
    if (touches.size === 2) {
      startPinch();
      return;
    }
    if (touches.size > 2 || gesture.kind !== 'none') return;
    const p = screenToWorld(view, s.x, s.y);
    pointer = { ...pointer, ...p, sx: s.x, sy: s.y, inside: true, shift: event.shiftKey };
    const pan = (deselect: boolean): Gesture => ({
      kind: 'pan',
      sx: s.x,
      sy: s.y,
      lastX: s.x,
      lastY: s.y,
      moved: false,
      deselect,
    });
    if (event.button === 1 || event.button === 2) {
      gesture = pan(false);
      updateCursor();
      return;
    }
    if (event.button !== 0) return;
    const id = engine.api.pick(p.x, p.y);
    if (tool === 'select') {
      if (!id) gesture = pan(true);
      else {
        selected = id;
        const b = liveBody(id)!;
        if (running && b.mass > 0) {
          engine.api.drag_start(id, p.x, p.y);
          gesture = { kind: 'spring', id };
        } else
          gesture = {
            kind: 'move',
            id,
            dx: b.x - p.x,
            dy: b.y - p.y,
            sx: s.x,
            sy: s.y,
            before: checkpoint(),
            moved: false,
          };
        sync();
      }
    } else if (shapeTools.includes(tool))
      gesture = { kind: 'place', x: p.x, y: p.y, sx: s.x, sy: s.y };
    else if (tool === 'impulse') {
      if (!id) gesture = pan(false);
      else {
        const b = liveBody(id)!;
        const local = toLocal(b, p.x, p.y);
        gesture = { kind: 'impulse', id, lx: local.x, ly: local.y };
        selected = id;
        sync();
      }
    } else if (tool === 'joint') {
      const starting = !pendingLink && jointKind !== 'pin';
      connect(p, id);
      if (starting && pendingLink) gesture = { kind: 'link', sx: s.x, sy: s.y };
    }
    updateCursor();
  }
  function pointerMove(event: PointerEvent) {
    const s = screenPoint(event);
    if (touches.has(event.pointerId)) touches.set(event.pointerId, s);
    if (gesture.kind === 'pinch') {
      if (touches.size < 2) return;
      const [a, b] = [...touches.values()];
      const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
      const factor = Math.hypot(a.x - b.x, a.y - b.y) / gesture.distance;
      let next = zoomAt(
        gesture.camera,
        viewFor(canvasWidth, canvasHeight, gesture.camera),
        gesture.mid.x,
        gesture.mid.y,
        factor,
      );
      next = panBy(
        next,
        viewFor(canvasWidth, canvasHeight, next),
        mid.x - gesture.mid.x,
        mid.y - gesture.mid.y,
      );
      camera = next;
      follow = false;
      updateView();
      return;
    }
    const p = screenToWorld(view, s.x, s.y);
    pointer = { ...pointer, ...p, sx: s.x, sy: s.y, inside: true, shift: event.shiftKey };
    if (!ready) return;
    const g = gesture;
    if (g.kind === 'spring') engine.api.drag_to(p.x, p.y);
    else if (g.kind === 'move') {
      if (!g.moved) {
        if (Math.hypot(s.x - g.sx, s.y - g.sy) < 3) return;
        g.moved = true;
        record('Move body', '', g.before);
      }
      engine.api.place(g.id, p.x + g.dx, p.y + g.dy);
    } else if (g.kind === 'pan') {
      camera = panBy(camera, view, s.x - g.lastX, s.y - g.lastY);
      g.lastX = s.x;
      g.lastY = s.y;
      if (Math.hypot(s.x - g.sx, s.y - g.sy) > 3) {
        g.moved = true;
        follow = false;
      }
      updateView();
    } else if (g.kind === 'none')
      hovered =
        tool === 'select' || tool === 'impulse' || tool === 'joint' ? engine.api.pick(p.x, p.y) : 0;
    updateCursor();
  }
  function pointerUp(event: PointerEvent) {
    const cancelled = event.type === 'pointercancel';
    touches.delete(event.pointerId);
    if (canvas.hasPointerCapture(event.pointerId)) canvas.releasePointerCapture(event.pointerId);
    if (gesture.kind === 'pinch') {
      if (touches.size < 2) gesture = { kind: 'none' };
      return;
    }
    if (!ready || touches.size > 0) return;
    const s = screenPoint(event);
    const p = screenToWorld(view, s.x, s.y);
    const g = gesture;
    gesture = { kind: 'none' };
    if (g.kind === 'spring') engine.api.drag_end();
    else if (g.kind === 'move' && g.moved) sync();
    else if (g.kind === 'pan' && !g.moved && g.deselect && !cancelled) selected = 0;
    else if (g.kind === 'place' && !cancelled) {
      const dragged = Math.hypot(s.x - g.sx, s.y - g.sy) > 6;
      placeBody(ghostFor(g, p, dragged, event.shiftKey));
    } else if (g.kind === 'impulse' && !cancelled) applyImpulse(g, p);
    else if (g.kind === 'link' && !cancelled && Math.hypot(s.x - g.sx, s.y - g.sy) > 6)
      connect(p, engine.api.pick(p.x, p.y));
    updateCursor();
  }
  function wheel(event: WheelEvent) {
    if (!ready) return;
    event.preventDefault();
    const s = screenPoint(event);
    const factor = Math.exp(-event.deltaY * (event.deltaMode === 1 ? 0.05 : 0.0015));
    camera = zoomAt(camera, view, s.x, s.y, factor);
    updateView();
  }

  // Files ----------------------------------------------------------------------------------
  function download(blob: Blob, name: string) {
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  function exportScene() {
    if (!ready) return;
    download(
      new Blob([engine.exportScene()], { type: 'application/json' }),
      'poltergeist-scene.json',
    );
    notify('Scene exported with body motion, settings, and joints.');
  }
  function saveImage() {
    canvas.toBlob((blob) => {
      if (blob)
        download(
          blob,
          `poltergeist-${customScene ? 'custom' : scenes[scene].name.toLowerCase().replace(/\s+/g, '-')}.png`,
        );
    }, 'image/png');
  }
  async function importScene(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file || !ready) return;
    try {
      if (file.size > 1_000_000) throw new Error('Scene file is too large (maximum 1 MB).');
      const text = await file.text();
      const before = checkpoint();
      endGesture();
      settings = engine.importScene(text);
      record('Import scene', '', before);
      initialCustomScene = text;
      customScene = true;
      running = false;
      selected = 0;
      pendingLink = null;
      accumulator = 0;
      sync();
      notify('Scene loaded. Press play to begin.');
    } catch (error) {
      notify(error instanceof Error ? error.message : 'Unable to load scene.');
    }
    input.value = '';
  }

  // Keyboard -------------------------------------------------------------------------------
  function isTextEntry(target: EventTarget | null) {
    if (!(target instanceof HTMLElement)) return false;
    if (target.isContentEditable || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT')
      return true;
    return (
      target instanceof HTMLInputElement &&
      !['range', 'checkbox', 'radio', 'button'].includes(target.type)
    );
  }
  function keydown(event: KeyboardEvent) {
    if (isTextEntry(event.target) || help || !ready) return;
    const key = event.key.toLowerCase();
    if ((event.metaKey || event.ctrlKey) && !event.altKey) {
      if (key === 'z') {
        event.preventDefault();
        if (event.shiftKey) redo();
        else undo();
      } else if (key === 'y') {
        event.preventDefault();
        redo();
      } else if (key === 'd') {
        event.preventDefault();
        duplicate();
      }
      // Leave every other browser shortcut (reload, find, …) alone.
      return;
    }
    if (event.altKey || event.metaKey || event.ctrlKey) return;
    const target = event.target instanceof HTMLElement ? event.target : null;
    const control = target?.matches('button, input, select') ?? false;
    if (event.code === 'Space') {
      // Keyboard users can still press a control they tabbed to; after a click, Space means play/pause.
      if (control && !focusByPointer) return;
      event.preventDefault();
      spaceHandled = true;
      if (!event.repeat) pause();
      return;
    }
    const slider = target instanceof HTMLInputElement && target.type === 'range';
    if (slider && key.startsWith('arrow')) return;
    // Held keys repeat only for continuous actions.
    const repeatable = ['arrowleft', 'arrowright', ',', '.', 'q', 'e', '+', '=', '-', '_'];
    if (event.repeat && !repeatable.includes(key)) return;
    if (key === 'arrowright' || key === '.' || key === 'n') stepForward();
    else if (key === 'arrowleft' || key === ',') stepBack();
    else if (key === 'r') resetScene();
    else if (key === 'delete' || key === 'backspace') {
      event.preventDefault();
      deleteBody();
    } else if (key === 'escape') {
      if (pendingLink) pendingLink = null;
      else if (gesture.kind !== 'none') endGesture();
      else if (selected) selected = 0;
      else setTool('select');
    } else if (key === '?') openHelp();
    else if (key === 'q' || key === 'e') {
      if (!body) return;
      const step = event.shiftKey ? 1 : 15;
      rotateTo((body.angle * 180) / Math.PI + (key === 'q' ? step : -step), 'rotate-key');
    } else if (key === '+' || key === '=') zoomBy(1.25);
    else if (key === '-' || key === '_') zoomBy(1 / 1.25);
    else if (key === '0') resetView();
    else if (key === 'f') toggleFollow();
    else if (key === 's') {
      placeStatic = !placeStatic;
      notify(placeStatic ? 'New bodies will be static.' : 'New bodies will be dynamic.');
    } else {
      const t = tools.find((t) => t.key.toLowerCase() === key);
      if (t) setTool(t.id);
    }
  }
  function keyup(event: KeyboardEvent) {
    // Stop the browser from also "clicking" a mouse-focused button on Space release.
    if (event.code === 'Space' && spaceHandled) {
      event.preventDefault();
      spaceHandled = false;
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

  // Frame loop -----------------------------------------------------------------------------
  /** Records energy samples and motion trails whenever simulated time moves. */
  function trace(f: Frame) {
    if (f.tick === lastTick) return;
    if (f.tick < lastTick) trails.clear();
    while (energy.length && energy[energy.length - 1].tick >= f.tick) energy.pop();
    lastTick = f.tick;
    energyDirty = true;
    energy.push({ tick: f.tick, t: f.time, k: f.kinetic, p: f.potential });
    if (energy.length > 1200) energy.splice(0, energy.length - 900);
    if (!overlays.trails) return;
    for (const b of f.bodies) {
      if (b.mass === 0 || !b.awake) continue;
      let t = trails.get(b.id);
      if (!t) trails.set(b.id, (t = []));
      t.push(b.x, b.y);
      if (t.length > 180) t.splice(0, 2);
    }
  }
  function previews(f: Frame) {
    const find = (id: number) => f.bodies.find((b) => b.id === id);
    let ghost: Ghost | null = null;
    if (shapeTools.includes(tool) && pointer.inside) {
      const g = gesture;
      ghost =
        g.kind === 'place'
          ? ghostFor(
              g,
              pointer,
              Math.hypot(pointer.sx - g.sx, pointer.sy - g.sy) > 6,
              pointer.shift,
            )
          : g.kind === 'none'
            ? ghostFor(pointer, pointer, false, false)
            : null;
    }
    let link: Segment | null = null;
    if (pendingLink) {
      const a = pendingLink.id ? find(pendingLink.id) : undefined;
      const start = a ? toWorld(a, pendingLink.local.x, pendingLink.local.y) : pendingLink.local;
      if (!pendingLink.id || a) link = { x1: start.x, y1: start.y, x2: pointer.x, y2: pointer.y };
    }
    let push: Segment | null = null;
    if (gesture.kind === 'impulse') {
      const b = find(gesture.id);
      if (b) {
        const anchor = toWorld(b, gesture.lx, gesture.ly);
        push = { x1: anchor.x, y1: anchor.y, x2: pointer.x, y2: pointer.y };
      }
    }
    const pin =
      tool === 'joint' && jointKind === 'pin' && pointer.inside && hovered
        ? { x: pointer.x, y: pointer.y }
        : null;
    return { ghost, link, push, pin };
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
      updateView();
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
        while (accumulator >= DT && count < 8) {
          engine.api.step(DT);
          accumulator -= DT;
          count++;
        }
        if (count) simMs = simMs * 0.8 + ((performance.now() - start) / count) * 0.2;
      } else accumulator = 0;
      const current = ready ? engine.read() : emptyFrame();
      if (follow) {
        const b = current.bodies.find((b) => b.id === selected);
        if (b) {
          camera = {
            ...camera,
            x: camera.x + (b.x - camera.x) * 0.18,
            y: camera.y + (b.y - camera.y) * 0.18,
          };
          view = viewFor(canvasWidth, canvasHeight, camera);
        } else follow = false;
      }
      trace(current);
      if (current.culled > lastCulled) {
        const n = current.culled - lastCulled;
        notify(
          `${n} ${n === 1 ? 'body' : 'bodies'} left the laboratory and ${n === 1 ? 'was' : 'were'} removed.`,
        );
      }
      lastCulled = current.culled;
      if (now - refreshTime > 80) {
        frame = current;
        refreshTime = now;
        if (energyDirty) {
          energyVersion++;
          energyDirty = false;
        }
        if (!overlays.trails) trails.clear();
        else if (trails.size) {
          const ids = new Set(current.bodies.map((b) => b.id));
          for (const id of trails.keys()) if (!ids.has(id)) trails.delete(id);
        }
        if (elapsed > 0) fps = Math.round(fps * 0.85 + (1 / elapsed) * 0.15);
      }
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      render(ctx, {
        frame: current,
        view,
        selected,
        hovered,
        overlays,
        trails,
        ...previews(current),
      });
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    Engine.load()
      .then((value) => {
        if (!alive) return;
        engine = value;
        ready = true;
        loadScene(0, false);
      })
      .catch((error) => {
        fatal = error.message;
      });
    const visibility = () => {
      accumulator = 0;
      animationTime = 0;
      if (gesture.kind === 'spring') endGesture();
    };
    const pointerInput = () => (lastInput = 'pointer');
    const keyboardInput = (e: KeyboardEvent) => {
      if (!['Shift', 'Control', 'Alt', 'Meta'].includes(e.key)) lastInput = 'keyboard';
    };
    const focusIn = () => (focusByPointer = lastInput === 'pointer');
    document.addEventListener('pointerdown', pointerInput, true);
    document.addEventListener('keydown', keyboardInput, true);
    document.addEventListener('focusin', focusIn);
    canvas.addEventListener('wheel', wheel, { passive: false });
    window.addEventListener('keydown', keydown);
    window.addEventListener('keyup', keyup);
    document.addEventListener('visibilitychange', visibility);
    window.addEventListener('blur', visibility);
    return () => {
      alive = false;
      cancelAnimationFrame(raf);
      observer.disconnect();
      clearTimeout(toastTimer);
      canvas.removeEventListener('wheel', wheel);
      document.removeEventListener('pointerdown', pointerInput, true);
      document.removeEventListener('keydown', keyboardInput, true);
      document.removeEventListener('focusin', focusIn);
      window.removeEventListener('keydown', keydown);
      window.removeEventListener('keyup', keyup);
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
      <span class="build-label">EXPERIMENTAL / V0.2</span>
      <button
        class="icon-button"
        onclick={undo}
        disabled={!ready || !canUndo}
        title="Undo · {mod}Z"
        aria-label="Undo"><Icon name="undo" size={17} /></button
      ><button
        class="icon-button"
        onclick={redo}
        disabled={!ready || !canRedo}
        title="Redo · ⇧{mod}Z"
        aria-label="Redo"><Icon name="redo" size={17} /></button
      >
      <span class="topbar-rule small"></span>
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
        onclick={saveImage}
        disabled={!ready}
        title="Save image"
        aria-label="Save image"><Icon name="image" size={17} /></button
      >
      <button
        class="icon-button"
        onclick={openHelp}
        title="Help & shortcuts · ?"
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
        <div class="section-label">
          EXPERIMENTS <span>{String(scenes.length).padStart(2, '0')}</span>
        </div>
        <div class="scene-list">
          {#each scenes as item, index}<button
              class:active={scene === index && !customScene}
              class="scene-button"
              onclick={() => loadScene(index)}
              disabled={!ready}
              aria-pressed={scene === index && !customScene}
              ><span class="scene-number">{String(index + 1).padStart(2, '0')}</span><span
                class="scene-copy"><strong>{item.name}</strong><small>{item.sub}</small></span
              ><Icon name="chevron" size={13} /></button
            >{/each}
        </div>
      </section>
      <section class="panel-section">
        <div class="section-label">TOOLBOX <span>DRAG TO SIZE</span></div>
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
        <div class="chamber-title">
          <span class="eyebrow"
            >LIVE EXPERIMENT <span class="slash">/</span>
            {customScene ? 'CUSTOM' : String(scene + 1).padStart(2, '0')}</span
          >
          <h2>{currentTitle}</h2>
          <p class="scene-description">{currentDescription}</p>
        </div>
        <span class="status" class:paused={!running || help} class:replay={rewound}
          ><span></span>{rewound
            ? `REPLAY −${rewindSeconds.toFixed(2)} S`
            : running && !help
              ? 'SIMULATING'
              : 'PAUSED'}</span
        >
      </div>
      <div class="viewport" bind:this={viewport}>
        <canvas
          bind:this={canvas}
          aria-label="Interactive physics chamber. Use toolbox buttons to place bodies. Select a body in the Objects list to inspect it."
          onpointerdown={pointerDown}
          onpointermove={pointerMove}
          onpointerup={pointerUp}
          onpointercancel={pointerUp}
          onpointerleave={() => (pointer.inside = false)}
          oncontextmenu={(e) => e.preventDefault()}
        ></canvas>
        <div class="viewport-overlay">
          <div class="overlay-row">
            {#if shapeTools.includes(tool)}
              <div class="tool-options" role="group" aria-label="Placement options">
                <div class="segmented">
                  <button
                    class:active={!placeStatic}
                    aria-pressed={!placeStatic}
                    onclick={() => (placeStatic = false)}>Dynamic</button
                  ><button
                    class:active={placeStatic}
                    aria-pressed={placeStatic}
                    onclick={() => (placeStatic = true)}
                    title="Static bodies never move · S">Static</button
                  >
                </div>
                {#if tool === 'polygon'}
                  <div class="stepper">
                    <button
                      aria-label="Fewer sides"
                      disabled={sides <= MIN_SIDES}
                      onclick={() => (sides = Math.max(MIN_SIDES, sides - 1))}
                      ><Icon name="minus" size={12} /></button
                    ><span aria-live="polite">{sides} sides</span><button
                      aria-label="More sides"
                      disabled={sides >= MAX_SIDES}
                      onclick={() => (sides = Math.min(MAX_SIDES, sides + 1))}
                      ><Icon name="plus" size={12} /></button
                    >
                  </div>
                {/if}
              </div>
            {:else if tool === 'joint'}
              <div class="tool-options" role="group" aria-label="Connection type">
                <div class="segmented">
                  {#each jointKinds as kind}<button
                      class:active={jointKind === kind}
                      aria-pressed={jointKind === kind}
                      title={jointInfo[kind].hint}
                      onclick={() => setJointKind(kind)}
                      ><Icon name={kind} size={13} />{jointInfo[kind].label}</button
                    >{/each}
                </div>
              </div>
            {:else}<span></span>{/if}
            <div class="view-controls" role="group" aria-label="View">
              <button aria-label="Zoom out" title="Zoom out · −" onclick={() => zoomBy(1 / 1.25)}
                ><Icon name="minus" size={13} /></button
              ><span class="zoom-level" data-testid="zoom">{zoomPercent}%</span><button
                aria-label="Zoom in"
                title="Zoom in · +"
                onclick={() => zoomBy(1.25)}><Icon name="plus" size={13} /></button
              ><button aria-label="Reset view" title="Reset view · 0" onclick={resetView}
                ><Icon name="fit" size={14} /></button
              ><button
                aria-label="Follow selected body"
                title="Follow selection · F"
                class:active={follow}
                aria-pressed={follow}
                onclick={toggleFollow}><Icon name="follow" size={14} /></button
              >
            </div>
          </div>
          {#if ready && scene === 0 && !customScene && !nudged}<div class="experiment-note">
              <span>TRY THIS</span>
              <p>Give the sphere a push.<br />Watch the tower answer.</p>
              <button onclick={kick}>Apply a nudge <Icon name="arrow" size={14} /></button>
            </div>{/if}
        </div>
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
            onclick={stepBack}
            disabled={!ready || frame.historyCursor === 0}
            title="Step back one tick · ←"
            aria-label="Step back one tick"><Icon name="step-back" size={18} /></button
          ><button
            class="icon-button"
            onclick={stepForward}
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
        <div class="timeline" class:rewound>
          <input
            type="range"
            aria-label="Timeline"
            min="0"
            max={Math.max(1, frame.historyLength - 1)}
            step="1"
            value={frame.historyCursor}
            disabled={!ready || frame.historyLength < 2}
            aria-valuetext={rewound ? `${rewindSeconds.toFixed(2)} seconds ago` : 'Live'}
            oninput={(e) => {
              running = false;
              accumulator = 0;
              seek(+e.currentTarget.value);
            }}
          />
          <span class="timeline-label">{rewound ? `−${rewindSeconds.toFixed(2)} s` : 'LIVE'}</span>
        </div>
        <div class="speed-control">
          <label for="speed">SPEED</label><select id="speed" bind:value={speed}
            >{#each speeds as s}<option value={s}>{s}×</option>{/each}</select
          >
        </div>
      </div>
      <div class="chamber-hint">
        <Icon name="crosshair" size={14} /><span>{toolHint}</span><span class="tick-label"
          >TICK {String(frame.tick).padStart(5, '0')}</span
        >
      </div>
    </section>

    <aside class="right-panel">
      <Inspector {body} contacts={bodyContacts} joints={bodyJoints} actions={inspector} />
      <section class="panel-section energy-section">
        <div class="section-label">ENERGY <span>LAST 10 S</span></div>
        <EnergyChart samples={energy} version={energyVersion} />
      </section>
      <section class="panel-section settings">
        <div class="section-label">WORLD SETTINGS</div>
        <Slider
          id="gravity"
          label="Gravity"
          value={settings.gravity}
          min={0}
          max={20}
          step={0.01}
          inputMax={30}
          unit="m/s²"
          onchange={(v) => configure({ gravity: v })}
        />
        <Slider
          id="iterations"
          label="Solver iterations"
          value={settings.iterations}
          min={2}
          max={24}
          step={1}
          inputMin={1}
          inputMax={32}
          digits={0}
          onchange={(v) => configure({ iterations: Math.round(v) })}
        />
        <label class="toggle-row"
          ><span class="overlay-dot sleep"></span><span>Let resting bodies sleep</span><input
            type="checkbox"
            checked={settings.sleeping}
            onchange={(e) => configure({ sleeping: e.currentTarget.checked })}
            aria-label="Let resting bodies sleep"
          /><span class="toggle-track"></span></label
        >
        <div class="data-row muted">
          <span>Fixed timestep</span><strong>1/60 <small>s</small></strong>
        </div>
        <div class="data-row muted"><span>Substeps</span><strong>2</strong></div>
      </section>
      <section class="panel-section objects-section">
        <div class="section-label">OBJECTS <span>{frame.bodies.length}</span></div>
        <div class="object-list">
          {#each frame.bodies as object (object.id)}<button
              class:active={selected === object.id}
              class="object-button"
              onclick={() => {
                selected = object.id;
                setTool('select');
              }}
              aria-label={`Inspect ${shapeName(object).toLowerCase()} ${object.id}`}
              ><Icon
                name={object.kind === CIRCLE ? 'circle' : object.kind === BOX ? 'box' : 'polygon'}
                size={12}
              /><span>{shapeName(object)} #{String(object.id).padStart(2, '0')}</span
              >{#if !object.awake}<i class="asleep-dot" title="Sleeping"></i>{/if}<small
                >{object.mass === 0 ? 'STATIC' : `${fmt(object.mass, 1)} kg`}</small
              ></button
            >{/each}
        </div>
      </section>
    </aside>
  </main>

  <footer class="statusbar">
    <div class="status-metrics">
      <span><i class="engine-dot"></i>{fps} <small>FPS</small></span><span
        >{dynamicCount}<small>BODIES</small></span
      ><span>{frame.sleeping}<small>ASLEEP</small></span><span
        >{frame.contacts.length}<small>CONTACTS</small></span
      ><span>{frame.joints.length}<small>JOINTS</small></span><span
        >{fmt(simMs)}<small>ms / TICK</small></span
      >
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
    Place bodies (drag to size them), connect them with rods, ropes, springs, or pins, and inspect
    what happens. Dragging pulls through a spring while running and moves bodies directly while
    paused. Drag empty space or right-drag to pan; scroll or pinch to zoom. Rewind up to ten seconds
    with the timeline.
  </p>
  <div class="shortcut-grid">
    {#each shortcuts as item}<div>
        <span>{item.label}</span><kbd>{item.key}</kbd>
      </div>{/each}
  </div>
  <p class="microcopy">
    This version uses discrete collisions and a compact sequential impulse solver with sleeping
    islands. Very fast bodies may tunnel. Scene exports save body state, settings, and joints;
    preferences such as overlays are remembered in this browser.
  </p>
  <button class="primary-button" onclick={closeHelp}
    >Back to the experiment <Icon name="arrow" size={16} /></button
  >
</dialog>
