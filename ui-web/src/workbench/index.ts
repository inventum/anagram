import type { RenderCtx } from '../state';
import type { ActionDto, WorkbenchDto } from '../types';
import { ensureWorkbenchStyles } from './style';

// Pointer-only letter workbench (SPEC-ui §3.4, §3.6). No key listeners here.
// Hidden when state.workbench is absent. Hover is CSS :hover only (no dispatch).

const contexts = new WeakMap<HTMLElement, RenderCtx>();
const built = new WeakSet<HTMLElement>();

interface Parts {
  grid: HTMLElement;
  held: HTMLElement;
  pool: HTMLElement;
  starts: HTMLElement;
  ends: HTMLElement;
}

const parts = new WeakMap<HTMLElement, Parts>();

function make(tag: string, cls?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = text;
  return node;
}

function row(label: string, body: HTMLElement): HTMLElement {
  const r = make('div', 'wb-row');
  r.append(make('span', 'wb-label', label), body);
  return r;
}

function build(root: HTMLElement): Parts {
  root.classList.add('wb-panel');

  const clear = make('button', 'wb-clear', 'Clear') as HTMLButtonElement;
  clear.type = 'button';
  clear.id = 'wb-clear';
  clear.dataset.wbAction = 'clear';

  const head = make('div', 'wb-head');
  head.append(make('h2', 'wb-title', 'Letter workbench'), clear);

  const grid = make('div', 'wb-grid');
  grid.id = 'wb-grid';

  const held = make('div', 'wb-hold');
  held.id = 'wb-held';

  const pool = make('div', 'wb-pool');
  pool.id = 'wb-pool';

  const starts = make('div', 'wb-chips');
  starts.id = 'wb-starts';
  const ends = make('div', 'wb-chips');
  ends.id = 'wb-ends';

  const side = make('div', 'wb-side');
  const patterns = make('div', 'wb-side');
  patterns.append(make('span', 'wb-label', 'Patterns'), row('Prefix', starts), row('Suffix', ends));
  side.append(row('Hand', held), row('Pool', pool), patterns);

  const body = make('div', 'wb-body');
  body.append(grid, side);

  root.replaceChildren(head, body);
  return { grid, held, pool, starts, ends };
}

function tile(ch: string, id: string, extraCls: string, action?: { name: string; idx: number }): HTMLElement {
  const t = make('div', `wb-tile ${extraCls}`.trim(), ch);
  t.id = id;
  if (action) {
    t.dataset.wbAction = action.name;
    t.dataset.idx = String(action.idx);
  }
  return t;
}

function renderGrid(grid: HTMLElement, wb: WorkbenchDto): void {
  const isGrid = wb.layout === 'grid';
  grid.hidden = !isGrid;
  grid.style.gridTemplateColumns = `repeat(${wb.cols}, minmax(0, 1fr))`;
  grid.style.gridTemplateRows = `repeat(${wb.rows}, minmax(0, 1fr))`;
  const count = isGrid ? wb.rows * wb.cols : 0;
  const nodes: HTMLElement[] = [];
  for (let i = 0; i < count; i++) {
    const ch = wb.cells[i] ?? null;
    const cell = make('div', ch === null ? 'wb-cell wb-empty' : 'wb-cell wb-filled', ch ?? '');
    cell.id = `wb-cell-${i}`;
    cell.dataset.wbAction = 'cell';
    cell.dataset.idx = String(i);
    nodes.push(cell);
  }
  grid.replaceChildren(...nodes);
}

function renderChips(box: HTMLElement, items: string[], prefix: string): void {
  if (items.length === 0) {
    box.replaceChildren(make('span', 'wb-none', 'none'));
    return;
  }
  box.replaceChildren(
    ...items.map((text, i) => {
      const chip = make('span', 'wb-chip', text);
      chip.id = `wb-${prefix}-${i}`;
      return chip;
    }),
  );
}

function actionFor(target: HTMLElement): ActionDto | null {
  const idx = Number(target.dataset.idx);
  switch (target.dataset.wbAction) {
    case 'cell':
      return { type: 'workbenchClickCell', idx };
    case 'pool':
      return { type: 'workbenchClickPool', idx };
    case 'clear':
      return { type: 'workbenchClear' };
    default:
      return null;
  }
}

// Any click inside the panel first sets the grid focus ring, then runs the
// target's action. IPC is FIFO-queued (D-15), so the order is preserved.
function onPanelClick(this: HTMLElement, e: MouseEvent): void {
  const ctx = contexts.get(this);
  if (!ctx || !ctx.state.workbench) return;
  ctx.dispatch({ type: 'workbenchSetFocus', focus: 'grid' });
  const target = e.target instanceof Element ? e.target.closest<HTMLElement>('[data-wb-action]') : null;
  if (!target || !this.contains(target)) return;
  const action = actionFor(target);
  if (action) ctx.dispatch(action);
}

export function renderWorkbench(el: HTMLElement, ctx: RenderCtx): void {
  ensureWorkbenchStyles();
  contexts.set(el, ctx);

  const wb = ctx.state.workbench;
  if (!wb) {
    el.hidden = true;
    el.style.display = 'none';
    return;
  }
  el.hidden = false;
  el.style.display = '';

  if (!built.has(el)) {
    parts.set(el, build(el));
    built.add(el);
    el.addEventListener('click', onPanelClick);
  }
  const p = parts.get(el);
  if (!p) return;

  el.classList.toggle('wb-focused', wb.focus === 'grid');

  renderGrid(p.grid, wb);

  if (wb.held !== null) {
    p.held.replaceChildren(tile(wb.held, 'wb-held-tile', 'wb-held-tile'));
  } else {
    p.held.replaceChildren();
  }

  p.pool.replaceChildren(
    ...wb.pool.map((ch, i) => tile(ch, `wb-pool-${i}`, 'wb-pool-tile', { name: 'pool', idx: i })),
  );

  renderChips(p.starts, wb.affixes.starts, 'start');
  renderChips(p.ends, wb.affixes.ends, 'end');
}
