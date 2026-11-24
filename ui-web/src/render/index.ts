import type { RenderCtx } from '../state';
import type { RoundDto, UiState } from '../types';
import { guardedDispatch, renderOverlays } from '../overlays';
import { renderWorkbench } from '../workbench';

// Play window render (SPEC-ui §3.1–§3.3, §5.5, §7; Fig. 1, 2a, 2c).
// Render only: no game logic, no word lists, no answer checks.
// DOM is built once per root and updated in place so focus survives re-renders.

const TEMPLATE = `
<main class="play-window" aria-label="Anagram play window">
  <div class="toolbar" role="toolbar" aria-label="Puzzle controls">
    <div class="length">
      <label class="label" for="len-slider">Length</label>
      <input id="len-slider" type="range" step="1" aria-describedby="len-value" data-role="slider" />
      <output id="len-value" for="len-slider" data-role="slider-out"></output>
    </div>
    <div class="toolbar__actions">
      <button class="btn" type="button" data-role="custom">Custom word</button>
      <button class="btn" type="button" data-role="new">New puzzle</button>
    </div>
  </div>
  <header class="app-header">
    <h1 class="puzzle-title">ANAGRAM <span data-role="title-len"></span></h1>
    <div class="app-header__actions">
      <p class="counter" data-role="counter"></p>
      <button class="btn" type="button" aria-label="Next clue (Tab)" data-role="next">&#x21E5; Next</button>
      <button class="btn btn--icon" type="button" aria-label="Help (question mark)" data-role="help">?</button>
    </div>
  </header>
  <div class="play">
    <div class="board">
      <div class="round" data-role="round">
        <p class="label" id="pool-label">Scrambled</p>
        <ul class="pool" aria-labelledby="pool-label" data-role="pool"></ul>
        <p class="label" id="answer-label">Your answer</p>
        <ol class="answer" aria-labelledby="answer-label" data-role="answer"></ol>
        <p class="solution" data-role="solution"><span class="label">Solution</span> <strong data-role="solution-text"></strong></p>
      </div>
      <p class="status" role="status" data-role="status"><span class="status__icon" aria-hidden="true" data-role="status-icon"></span><span data-role="status-text"></span></p>
    </div>
    <aside class="side" aria-label="Clue">
      <section class="panel" aria-labelledby="clue-label">
        <p class="label" id="clue-label">Clue</p>
        <p class="clue__text" data-role="clue"></p>
      </section>
    </aside>
  </div>
  <div class="workbench-slot" data-role="workbench"></div>
  <footer class="keyhints">
    <span><kbd>&#x2190;</kbd><kbd>&#x2192;</kbd> move</span>
    <span><kbd>&#x232B;</kbd> clear</span>
    <span><kbd>Tab</kbd> clue</span>
    <span><kbd>Ctrl+R</kbd> reset</span>
    <span><kbd>?</kbd> help</span>
  </footer>
</main>`;

interface Refs {
  ctx: RenderCtx;
  slider: HTMLInputElement;
  sliderOut: HTMLOutputElement;
  titleLen: HTMLElement;
  counter: HTMLElement;
  round: HTMLElement;
  pool: HTMLElement;
  answer: HTMLElement;
  solution: HTMLElement;
  solutionText: HTMLElement;
  statusEl: HTMLElement;
  statusIcon: HTMLElement;
  statusText: HTMLElement;
  clue: HTMLElement;
  workbench: HTMLElement;
}

const registry = new WeakMap<HTMLElement, Refs>();

function q<T extends HTMLElement>(scope: ParentNode, selector: string): T {
  const node = scope.querySelector<T>(selector);
  if (node === null) {
    throw new Error(`render: missing element ${selector}`);
  }
  return node;
}

function build(root: HTMLElement, ctx: RenderCtx): Refs {
  root.insertAdjacentHTML('beforeend', TEMPLATE);
  const win = q<HTMLElement>(root, '.play-window');

  const refs: Refs = {
    ctx,
    slider: q<HTMLInputElement>(win, '[data-role="slider"]'),
    sliderOut: q<HTMLOutputElement>(win, '[data-role="slider-out"]'),
    titleLen: q(win, '[data-role="title-len"]'),
    counter: q(win, '[data-role="counter"]'),
    round: q(win, '[data-role="round"]'),
    pool: q(win, '[data-role="pool"]'),
    answer: q(win, '[data-role="answer"]'),
    solution: q(win, '[data-role="solution"]'),
    solutionText: q(win, '[data-role="solution-text"]'),
    statusEl: q(win, '[data-role="status"]'),
    statusIcon: q(win, '[data-role="status-icon"]'),
    statusText: q(win, '[data-role="status-text"]'),
    clue: q(win, '[data-role="clue"]'),
    workbench: q(win, '[data-role="workbench"]'),
  };

  // Slider: `input` only previews (draft); `change` (release) commits.
  refs.slider.addEventListener('input', () => {
    const n = Number(refs.slider.value);
    if (refs.ctx.local.sliderDraft !== n) {
      refs.ctx.setLocal({ sliderDraft: n });
    }
  });
  refs.slider.addEventListener('change', () => {
    const n = Number(refs.slider.value);
    const c = refs.ctx;
    c.setLocal({ sliderDraft: null });
    if (n !== c.state.length) {
      guardedDispatch({ type: 'selectSize', n }, c);
    }
  });

  q<HTMLButtonElement>(win, '[data-role="custom"]').addEventListener('click', () => {
    guardedDispatch({ type: 'openCustomWord' }, refs.ctx);
  });
  q<HTMLButtonElement>(win, '[data-role="new"]').addEventListener('click', () => {
    guardedDispatch({ type: 'newPuzzle' }, refs.ctx);
  });
  q<HTMLButtonElement>(win, '[data-role="next"]').addEventListener('click', () => {
    refs.ctx.dispatch({ type: 'requestClue' });
  });
  q<HTMLButtonElement>(win, '[data-role="help"]').addEventListener('click', () => {
    refs.ctx.dispatch({ type: 'toggleHelp' });
  });
  refs.answer.addEventListener('click', () => {
    refs.ctx.dispatch({ type: 'workbenchSetFocus', focus: 'answer' });
  });

  return refs;
}

// Keep existing child nodes where possible; add or remove only the difference.
function syncList(parent: HTMLElement, count: number, makeNode: () => HTMLElement): HTMLElement[] {
  while (parent.childElementCount > count) {
    parent.lastElementChild?.remove();
  }
  while (parent.childElementCount < count) {
    parent.appendChild(makeNode());
  }
  return Array.from(parent.children) as HTMLElement[];
}

function makeTile(): HTMLElement {
  const li = document.createElement('li');
  li.className = 'tile';
  return li;
}

function makeSlot(): HTMLElement {
  const li = document.createElement('li');
  li.className = 'slot';
  li.innerHTML = '<span class="slot__letter"></span><span class="mark" aria-hidden="true"></span><span class="sr-only"></span>';
  return li;
}

function renderRound(refs: Refs, r: RoundDto, s: UiState): void {
  const won = s.phase === 'won';

  const tiles = syncList(refs.pool, r.pool.length, makeTile);
  r.pool.forEach((ch, i) => {
    tiles[i].textContent = ch;
  });

  const slots = syncList(refs.answer, r.answer.length, makeSlot);
  refs.answer.classList.toggle('is-wrong', s.status.kind === 'wrongAnswer');
  refs.answer.classList.toggle('is-won', won);
  r.answer.forEach((ch, i) => {
    const slot = slots[i];
    const correct = won || r.correct?.[i] === true;
    const cursor = i === r.cursor;
    const [letter, mark, srText] = Array.from(slot.children) as HTMLElement[];

    slot.className = ['slot', ch === null ? 'is-empty' : 'is-filled', correct ? 'is-correct' : '']
      .filter((c) => c !== '')
      .join(' ');
    if (cursor) {
      slot.setAttribute('aria-current', 'true');
    } else {
      slot.removeAttribute('aria-current');
    }
    letter.textContent = ch ?? '';
    mark.textContent = correct ? '\u2713' : '';
    srText.textContent = `Slot ${i + 1}${ch === null ? ', empty' : ''}${cursor ? ', cursor' : ''}${correct ? ', correct' : ''}`;
  });

  const showSolution = won && r.solution !== undefined;
  refs.solution.hidden = !showSolution;
  refs.solutionText.textContent = r.solution ?? '';
}

function update(refs: Refs, ctx: RenderCtx): void {
  refs.ctx = ctx;
  const s = ctx.state;
  const won = s.phase === 'won';

  // Toolbar: slider shows the draft while dragging, otherwise the live length.
  const shown = ctx.local.sliderDraft ?? s.length;
  refs.slider.min = String(s.minSize);
  refs.slider.max = String(s.maxSize);
  if (refs.slider.value !== String(shown)) {
    refs.slider.value = String(shown);
  }
  refs.sliderOut.textContent = String(shown);
  refs.sliderOut.setAttribute('aria-label', `Puzzle length ${shown}`);

  // Header.
  refs.titleLen.textContent = `\u00B7 ${s.length} letters`;
  refs.counter.textContent = `Clues ${s.clues.used}/${s.clues.budget}`;

  // Board: hidden in custom-word phases (round absent, SPEC-ui §5.5).
  const r = s.round;
  refs.round.hidden = r === undefined;
  if (r !== undefined) {
    renderRound(refs, r, s);
  }

  // Status line: kind as icon plus text (never color alone).
  refs.statusEl.className = `status status--${s.status.kind}`;
  refs.statusIcon.textContent = s.status.kind === 'error' || s.status.kind === 'wrongAnswer' ? '\u26A0' : '';
  refs.statusText.textContent = s.status.message ?? (won ? 'Correct!' : '');

  // Clue panel.
  refs.clue.textContent = s.activeClue ?? 'No clue yet. Press Tab for one.';
  refs.clue.classList.toggle('is-empty', s.activeClue === null);

  // Workbench container; WP-10 renders its contents.
  refs.workbench.hidden = s.workbench === undefined;
  if (s.workbench !== undefined) {
    renderWorkbench(refs.workbench, ctx);
  }
}

export function renderApp(root: HTMLElement, ctx: RenderCtx): void {
  let refs = registry.get(root);
  if (refs === undefined) {
    refs = build(root, ctx);
    registry.set(root, refs);
  }
  update(refs, ctx);
  renderOverlays(root, ctx);
}
