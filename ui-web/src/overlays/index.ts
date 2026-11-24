import './overlays.css';
import type { RenderCtx } from '../state';
import type { ActionDto, ChoiceDto, PhaseDto, StatusDto, UiState } from '../types';

const HOST_ATTR = 'data-overlays';

/** D-8: actions that replace an unfinished puzzle and need confirmation. */
const GATED_ACTIONS: ReadonlySet<ActionDto['type']> = new Set(['newPuzzle', 'openCustomWord', 'selectSize']);

/** D-8: confirm only in these phases. `won` never confirms. */
const GATED_PHASES: ReadonlySet<PhaseDto> = new Set<PhaseDto>(['playing', 'help']);

const HELP_ROWS: ReadonlyArray<{ keys: readonly string[]; text: string }> = [
  { keys: ['←', '→'], text: 'Move cursor' },
  { keys: ['a–z'], text: 'Type into cursor slot (pool letters only)' },
  { keys: ['⌫'], text: 'Clear slot, or step left and clear' },
  { keys: ['Tab'], text: 'Next clue' },
  { keys: ['Ctrl+R'], text: 'Reset round (same word, reshuffled)' },
  { keys: ['?'], text: 'Toggle this help' },
  { keys: ['Esc'], text: 'Close help (Esc also closes any open dialog or input)' },
  { keys: ['⌘Q'], text: 'Quit' },
];

interface Ref {
  ctx: RenderCtx;
}

interface Mounted {
  ref: Ref;
  customInput: HTMLElement;
  customInputValue: HTMLElement;
  customInputCount: HTMLElement;
  customInputStatus: HTMLElement;
  customPick: HTMLElement;
  customPickValue: HTMLElement;
  customPickList: HTMLUListElement;
  customPickStatus: HTMLElement;
  customPickRows: HTMLLIElement[];
  customPickKey: string;
  help: HTMLElement;
  confirm: HTMLElement;
  confirmCancel: HTMLButtonElement;
  confirmShown: boolean;
}

const mounts = new WeakMap<HTMLElement, Mounted>();

/** Gate: confirm-abandon for unfinished puzzles, otherwise dispatch directly. */
export function guardedDispatch(action: ActionDto, ctx: RenderCtx): void {
  if (action.type === 'selectSize' && action.n === ctx.state.length) {
    // Slider released at the current length: nothing to replace.
    clearSliderDraft(ctx);
    return;
  }
  if (GATED_ACTIONS.has(action.type) && GATED_PHASES.has(ctx.state.phase)) {
    ctx.setLocal({ pending: action });
    return;
  }
  if (action.type === 'selectSize') {
    clearSliderDraft(ctx);
  }
  ctx.dispatch(action);
}

export function renderOverlays(root: HTMLElement, ctx: RenderCtx): void {
  const m = mountFor(root, ctx);
  m.ref.ctx = ctx;

  const { state, local } = ctx;
  const inCustomInput = state.phase === 'customWordInput';
  const inCustomPick = state.phase === 'customWordPick';
  const confirmOpen = local.pending !== null && GATED_PHASES.has(state.phase);

  m.customInput.hidden = !inCustomInput;
  m.customPick.hidden = !inCustomPick;
  m.help.hidden = !state.helpOpen;
  m.confirm.hidden = !confirmOpen;

  if (inCustomInput) {
    renderCustomInput(m, state);
  }
  if (inCustomPick) {
    renderCustomPick(m, state);
  }

  // Default focus on Cancel when the confirm dialog opens. WP-13 owns trap and return.
  if (confirmOpen && !m.confirmShown) {
    m.confirmCancel.focus();
  }
  m.confirmShown = confirmOpen;
}

function clearSliderDraft(ctx: RenderCtx): void {
  if (ctx.local.sliderDraft !== null) {
    ctx.setLocal({ sliderDraft: null });
  }
}

function cancelPending(ref: Ref): void {
  ref.ctx.setLocal({ pending: null, sliderDraft: null });
}

function abandon(ref: Ref): void {
  const { ctx } = ref;
  const pending = ctx.local.pending;
  if (pending === null) {
    return;
  }
  ctx.setLocal({ pending: null, sliderDraft: null });
  ctx.dispatch(pending);
}

function send(ref: Ref, action: ActionDto): void {
  guardedDispatch(action, ref.ctx);
}

function mountFor(root: HTMLElement, ctx: RenderCtx): Mounted {
  const existing = Array.from(root.children).find(
    (child): child is HTMLElement => child instanceof HTMLElement && child.hasAttribute(HOST_ATTR),
  );
  if (existing !== undefined) {
    const found = mounts.get(existing);
    if (found !== undefined) {
      return found;
    }
    existing.remove();
  }
  const host = el('div', { [HOST_ATTR]: '' });
  root.append(host);
  const m = build(ctx);
  host.append(m.customInput, m.customPick, m.help, m.confirm);
  mounts.set(host, m);
  return m;
}

function build(ctx: RenderCtx): Mounted {
  const ref: Ref = { ctx };

  // Custom word input (Fig. 2b, input state). Display-only field; keys come from WP-8.
  const customInputValue = el('span', { class: 'overlay__value' });
  const customInputCount = el('span', { class: 'overlay__count' });
  const customInputStatus = el('p', { class: 'overlay__status' });
  const customInput = overlay(
    'overlay--custom',
    el('section', { class: 'overlay__panel', role: 'dialog', 'aria-modal': 'true', 'aria-labelledby': 'overlay-custom-input-title' }, [
      el('h3', { id: 'overlay-custom-input-title', class: 'overlay__title' }, ['Custom word']),
      el('p', { id: 'overlay-custom-input-label', class: 'overlay__label' }, ['Scramble']),
      el('div', { class: 'overlay__field', role: 'textbox', 'aria-readonly': 'true', 'aria-labelledby': 'overlay-custom-input-label' }, [
        customInputValue,
        customInputCount,
      ]),
      customInputStatus,
      el('div', { class: 'overlay__actions' }, [
        button('Cancel', () => send(ref, { type: 'cancelCustomWord' })),
        button('Submit', () => send(ref, { type: 'submitCustomWord' }), 'overlay__primary'),
      ]),
    ]),
  );

  // Custom word pick (Fig. 2b, pick state).
  const customPickValue = el('span', { class: 'overlay__value' });
  const customPickList = el('ul', { class: 'overlay__pick', role: 'listbox', 'aria-label': 'Matching words' });
  const customPickStatus = el('p', { class: 'overlay__status' });
  const customPick = overlay(
    'overlay--custom',
    el('section', { class: 'overlay__panel', role: 'dialog', 'aria-modal': 'true', 'aria-labelledby': 'overlay-custom-pick-title' }, [
      el('h3', { id: 'overlay-custom-pick-title', class: 'overlay__title' }, ['Several words match']),
      el('p', { id: 'overlay-custom-pick-label', class: 'overlay__label' }, ['Scramble']),
      el('div', { class: 'overlay__field', role: 'textbox', 'aria-readonly': 'true', 'aria-labelledby': 'overlay-custom-pick-label' }, [
        customPickValue,
      ]),
      customPickList,
      customPickStatus,
      el('p', { class: 'overlay__hint' }, ['↑ ↓ move · Enter start · u / Esc cancel']),
      el('div', { class: 'overlay__actions' }, [
        button('Cancel', () => send(ref, { type: 'cancelCustomWord' })),
        button('Start', () => send(ref, { type: 'confirmCustomPick' }), 'overlay__primary'),
      ]),
    ]),
  );

  // Help overlay (Fig. 2d).
  const help = overlay(
    'overlay--help',
    el('section', { class: 'overlay__panel', role: 'dialog', 'aria-modal': 'true', 'aria-labelledby': 'overlay-help-title' }, [
      el('div', { class: 'overlay__head' }, [
        el('h3', { id: 'overlay-help-title', class: 'overlay__title' }, ['Keys']),
        button('×', () => send(ref, { type: 'toggleHelp' }), '', 'Close help'),
      ]),
      el('table', { class: 'overlay__keys', 'aria-label': 'Keyboard controls' }, [
        el(
          'tbody',
          {},
          HELP_ROWS.map((row) =>
            el('tr', {}, [el('th', { scope: 'row' }, keyNodes(row.keys)), el('td', {}, [row.text])]),
          ),
        ),
      ]),
    ]),
  );

  // Confirm abandon (Fig. 2e). Cancel is the default focus target.
  const confirmCancel = button('Cancel', () => cancelPending(ref));
  const confirmAbandon = button('Abandon', () => abandon(ref), 'overlay__danger');
  const confirm = overlay(
    'overlay--confirm',
    el(
      'section',
      {
        class: 'overlay__panel overlay__panel--compact',
        role: 'dialog',
        'aria-modal': 'true',
        'aria-labelledby': 'overlay-confirm-title',
        'aria-describedby': 'overlay-confirm-desc',
      },
      [
        el('h3', { id: 'overlay-confirm-title', class: 'overlay__title' }, ['Confirm abandon?']),
        el('p', { id: 'overlay-confirm-desc', class: 'overlay__label' }, [
          'This puzzle is not solved yet. Continuing will replace it, and your progress will be lost.',
        ]),
        el('div', { class: 'overlay__actions' }, [confirmCancel, confirmAbandon]),
      ],
    ),
  );

  return {
    ref,
    customInput,
    customInputValue,
    customInputCount,
    customInputStatus,
    customPick,
    customPickValue,
    customPickList,
    customPickStatus,
    customPickRows: [],
    customPickKey: '',
    help,
    confirm,
    confirmCancel,
    confirmShown: false,
  };
}

function renderCustomInput(m: Mounted, state: UiState): void {
  const input = state.custom.input.toUpperCase();
  m.customInputValue.textContent = input;
  m.customInputCount.textContent = `${input.length}/${state.maxSize}`;
  renderStatus(m.customInputStatus, state.status);
}

function renderCustomPick(m: Mounted, state: UiState): void {
  const { choices, pick } = state.custom;
  m.customPickValue.textContent = state.custom.input.toUpperCase();

  // Rebuild rows only when the candidate set changes; selection is attribute-only.
  const key = JSON.stringify(choices);
  if (key !== m.customPickKey) {
    m.customPickKey = key;
    m.customPickRows = choices.map((choice, idx) => pickRow(m, choice, idx));
    m.customPickList.replaceChildren(...m.customPickRows);
  }
  m.customPickRows.forEach((row, idx) => {
    row.setAttribute('aria-selected', String(idx === pick));
  });
  renderStatus(m.customPickStatus, state.status);
}

function pickRow(m: Mounted, choice: ChoiceDto, idx: number): HTMLLIElement {
  const row = el('li', { class: 'overlay__pick-row', role: 'option', 'aria-selected': 'false' }, [
    el('span', { class: 'overlay__pick-word' }, [choice.word.toUpperCase()]),
    el('span', { class: 'overlay__pick-def' }, [choice.definition]),
  ]);
  row.addEventListener('click', () => send(m.ref, { type: 'customPickSelect', idx }));
  return row;
}

function renderStatus(node: HTMLElement, status: StatusDto): void {
  const text = status.kind === 'none' ? '' : (status.message ?? '');
  node.textContent = text;
  node.dataset.kind = status.kind;
  node.hidden = text === '';
}

function overlay(layerClass: string, panel: HTMLElement): HTMLElement {
  const layer = el('div', { class: `overlay ${layerClass}`, hidden: '' }, [panel]);
  return layer;
}

function button(label: string, onClick: () => void, className = '', ariaLabel?: string): HTMLButtonElement {
  const attrs: Record<string, string> = { type: 'button', class: className };
  if (ariaLabel !== undefined) {
    attrs['aria-label'] = ariaLabel;
  }
  const node = el('button', attrs, [label]);
  node.addEventListener('click', onClick);
  return node;
}

function keyNodes(keys: readonly string[]): (Node | string)[] {
  const out: (Node | string)[] = [];
  keys.forEach((key, i) => {
    if (i > 0) {
      out.push(' ');
    }
    out.push(el('kbd', {}, [key]));
  });
  return out;
}

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Record<string, string> = {},
  children: (Node | string)[] = [],
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [name, value] of Object.entries(attrs)) {
    node.setAttribute(name, value);
  }
  node.append(...children);
  return node;
}
