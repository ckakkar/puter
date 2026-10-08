/** A whole-world snapshot plus the UI context needed to restore it faithfully. */
export interface Checkpoint {
  scene: string;
  sceneIndex: number;
  custom: boolean;
  /** The imported file a custom experiment resets to. */
  initial: string;
}
interface Entry extends Checkpoint {
  label: string;
}

/**
 * Snapshot-based undo. Each entry is the world *before* an edit. Consecutive edits that
 * share a key within a short window (a slider drag, repeated rotation) merge into one step.
 */
export class UndoStack {
  private past: Entry[] = [];
  private future: Entry[] = [];
  private lastKey = '';
  private lastTime = 0;
  constructor(
    private limit = 80,
    private mergeWindow = 1500,
  ) {}
  get canUndo() {
    return this.past.length > 0;
  }
  get canRedo() {
    return this.future.length > 0;
  }
  /**
   * Records the state before an edit (computed only when needed). Returns false when the
   * edit merges into the previous step.
   */
  push(
    before: Checkpoint | (() => Checkpoint),
    label: string,
    key = '',
    now = Date.now(),
  ): boolean {
    const merge = key !== '' && key === this.lastKey && now - this.lastTime < this.mergeWindow;
    this.lastKey = key;
    this.lastTime = now;
    if (merge) return false;
    this.past.push({ ...(typeof before === 'function' ? before() : before), label });
    if (this.past.length > this.limit) this.past.shift();
    this.future = [];
    return true;
  }
  /** Returns the checkpoint to restore, filing `current` for redo. */
  undo(current: Checkpoint): Entry | undefined {
    const entry = this.past.pop();
    if (entry) this.future.push({ ...current, label: entry.label });
    this.lastKey = '';
    return entry;
  }
  redo(current: Checkpoint): Entry | undefined {
    const entry = this.future.pop();
    if (entry) this.past.push({ ...current, label: entry.label });
    this.lastKey = '';
    return entry;
  }
  clear() {
    this.past = [];
    this.future = [];
    this.lastKey = '';
  }
}
