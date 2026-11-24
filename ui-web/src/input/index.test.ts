import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { RenderCtx } from '../state';
import type { ActionDto, UiState } from '../types';
import { bindInput } from './index';

type Listener = (event: KeyboardEvent) => void;

interface FakeEvent {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  isComposing: boolean;
  repeat: boolean;
  preventDefault: ReturnType<typeof vi.fn>;
}

function fakeKey(k: string, mods: Partial<FakeEvent> = {}): FakeEvent {
  return {
    key: k,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    isComposing: false,
    repeat: false,
    preventDefault: vi.fn(),
    ...mods,
  };
}

function baseState(phase: UiState['phase'], helpOpen = false): UiState {
  return {
    phase,
    helpOpen,
    length: 5,
    minSize: 4,
    maxSize: 10,
    clues: { used: 0, budget: 3 },
    activeClue: null,
    status: { kind: 'none', message: null },
    custom: { input: '', choices: [], pick: 0 },
    theme: {
      emptySlot: '#ff00ff',
      filledSlot: '#ffff00',
      pool: '#00ffff',
      correctPosition: '#00ff00',
    },
  };
}

function makeCtx(state: UiState, pending: ActionDto | null = null): RenderCtx {
  return {
    state,
    local: { pending, sliderDraft: null },
    dispatch: vi.fn(),
    requestReplace: vi.fn(),
    setLocal: vi.fn(),
  };
}

describe('bindInput', () => {
  let listeners: Listener[];

  beforeEach(() => {
    listeners = [];
    vi.stubGlobal('window', {
      addEventListener: (type: string, fn: Listener) => {
        if (type === 'keydown') {
          listeners.push(fn);
        }
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  function press(e: FakeEvent): void {
    for (const fn of listeners) {
      fn(e as unknown as KeyboardEvent);
    }
  }

  it('attaches exactly one keydown listener on window', () => {
    bindInput(makeCtx(baseState('playing')));
    expect(listeners).toHaveLength(1);
  });

  it('dispatches typeLetter without preventDefault', () => {
    const ctx = makeCtx(baseState('playing'));
    bindInput(ctx);
    const e = fakeKey('a');
    press(e);
    expect(ctx.dispatch).toHaveBeenCalledWith({ type: 'typeLetter', ch: 'a' });
    expect(e.preventDefault).not.toHaveBeenCalled();
  });

  it('prevents default and dispatches requestClue on Tab', () => {
    const ctx = makeCtx(baseState('playing'));
    bindInput(ctx);
    const e = fakeKey('Tab');
    press(e);
    expect(ctx.dispatch).toHaveBeenCalledWith({ type: 'requestClue' });
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
  });

  it('prevents default on handled Arrow keys in custom pick', () => {
    const ctx = makeCtx(baseState('customWordPick'));
    bindInput(ctx);
    for (const k of ['ArrowUp', 'ArrowDown']) {
      const e = fakeKey(k);
      press(e);
      expect(e.preventDefault).toHaveBeenCalledTimes(1);
    }
  });

  it('prevents default on Backspace and arrows in playing', () => {
    const ctx = makeCtx(baseState('playing'));
    bindInput(ctx);
    const back = fakeKey('Backspace');
    const left = fakeKey('ArrowLeft');
    press(back);
    press(left);
    expect(back.preventDefault).toHaveBeenCalledTimes(1);
    expect(left.preventDefault).toHaveBeenCalledTimes(1);
  });

  it('prevents default on Ctrl+R and dispatches resetRound', () => {
    const ctx = makeCtx(baseState('playing'));
    bindInput(ctx);
    const e = fakeKey('r', { ctrlKey: true });
    press(e);
    expect(ctx.dispatch).toHaveBeenCalledWith({ type: 'resetRound' });
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
  });

  it('does not prevent default or dispatch for unbound keys', () => {
    const ctx = makeCtx(baseState('playing'));
    bindInput(ctx);
    const e = fakeKey('1');
    press(e);
    expect(ctx.dispatch).not.toHaveBeenCalled();
    expect(ctx.requestReplace).not.toHaveBeenCalled();
    expect(e.preventDefault).not.toHaveBeenCalled();
  });

  it('routes newPuzzle and openCustomWord through requestReplace', () => {
    const ctx = makeCtx(baseState('won'));
    bindInput(ctx);
    press(fakeKey('n'));
    press(fakeKey('i'));
    expect(ctx.requestReplace).toHaveBeenNthCalledWith(1, { type: 'newPuzzle' });
    expect(ctx.requestReplace).toHaveBeenNthCalledWith(2, { type: 'openCustomWord' });
    expect(ctx.dispatch).not.toHaveBeenCalled();
  });

  it('reads live ctx.state on each event', () => {
    const ctx = makeCtx(baseState('playing'));
    bindInput(ctx);
    ctx.state = baseState('won');
    press(fakeKey('n'));
    expect(ctx.requestReplace).toHaveBeenCalledWith({ type: 'newPuzzle' });
  });

  it('Esc while dialog open clears pending and sliderDraft, dispatches nothing', () => {
    const ctx = makeCtx(baseState('playing'), { type: 'newPuzzle' });
    bindInput(ctx);
    const e = fakeKey('Escape');
    press(e);
    expect(ctx.setLocal).toHaveBeenCalledWith({ pending: null, sliderDraft: null });
    expect(ctx.dispatch).not.toHaveBeenCalled();
    expect(ctx.requestReplace).not.toHaveBeenCalled();
  });

  it('other keys while dialog open do nothing', () => {
    const ctx = makeCtx(baseState('playing', true), { type: 'newPuzzle' });
    bindInput(ctx);
    const tab = fakeKey('Tab');
    press(tab);
    press(fakeKey('a'));
    expect(ctx.dispatch).not.toHaveBeenCalled();
    expect(ctx.setLocal).not.toHaveBeenCalled();
    expect(tab.preventDefault).not.toHaveBeenCalled();
  });

  it('Esc with help open and no dialog dispatches toggleHelp', () => {
    const ctx = makeCtx(baseState('help', true));
    bindInput(ctx);
    press(fakeKey('Escape'));
    expect(ctx.dispatch).toHaveBeenCalledWith({ type: 'toggleHelp' });
    expect(ctx.setLocal).not.toHaveBeenCalled();
  });

  it('Esc in custom word pick dispatches cancelCustomWord', () => {
    const ctx = makeCtx(baseState('customWordPick'));
    bindInput(ctx);
    press(fakeKey('Escape'));
    expect(ctx.dispatch).toHaveBeenCalledWith({ type: 'cancelCustomWord' });
  });
});
