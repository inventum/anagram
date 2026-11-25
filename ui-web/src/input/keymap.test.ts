import { describe, expect, it } from 'vitest';
import type { PhaseDto } from '../types';
import { isDialogCancelKey, keyToAction, type KeyLike, type KeymapCtx } from './keymap';

function key(k: string, mods: Partial<Omit<KeyLike, 'key'>> = {}): KeyLike {
  return {
    key: k,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    isComposing: false,
    repeat: false,
    ...mods,
  };
}

function ctx(phase: PhaseDto, extra: Partial<KeymapCtx> = {}): KeymapCtx {
  return { phase, helpOpen: false, dialogOpen: false, ...extra };
}

const LETTERS = 'abcdefghijklmnopqrstuvwxyz'.split('');
const DIGITS = '0123456789'.split('');

describe('keyToAction: playing / help (answer focus, SPEC-ui §3.6)', () => {
  for (const phase of ['playing', 'help'] as const) {
    describe(phase, () => {
      const c = ctx(phase, { helpOpen: phase === 'help' });

      it('maps a-z to typeLetter with lowercase ch', () => {
        for (const l of LETTERS) {
          expect(keyToAction(key(l), c)).toEqual({ type: 'typeLetter', ch: l });
        }
      });

      it('maps uppercase letters (Shift) case-insensitively', () => {
        expect(keyToAction(key('A', { shiftKey: true }), c)).toEqual({ type: 'typeLetter', ch: 'a' });
        expect(keyToAction(key('Z', { shiftKey: true }), c)).toEqual({ type: 'typeLetter', ch: 'z' });
      });

      it('keeps typing and arrows unchanged; pointer setCursor is never a key action', () => {
        // Same payload a pool tile click sends: { type: 'typeLetter', ch: lowercase }.
        expect(keyToAction(key('E', { shiftKey: true }), c)).toEqual({ type: 'typeLetter', ch: 'e' });
        for (const l of LETTERS) {
          expect(keyToAction(key(l), c)?.type).toBe('typeLetter');
        }
        expect(keyToAction(key('ArrowLeft'), c)).toEqual({ type: 'cursorLeft' });
        expect(keyToAction(key('ArrowRight'), c)).toEqual({ type: 'cursorRight' });
        for (const k of ['ArrowLeft', 'ArrowRight', 'Backspace', 'Tab', 'a']) {
          expect(keyToAction(key(k), c)?.type).not.toBe('setCursor');
        }
      });

      it('maps ArrowLeft and ArrowRight to cursor moves', () => {
        expect(keyToAction(key('ArrowLeft'), c)).toEqual({ type: 'cursorLeft' });
        expect(keyToAction(key('ArrowRight'), c)).toEqual({ type: 'cursorRight' });
      });

      it('maps Backspace to backspace', () => {
        expect(keyToAction(key('Backspace'), c)).toEqual({ type: 'backspace' });
      });

      it('maps Tab to requestClue', () => {
        expect(keyToAction(key('Tab'), c)).toEqual({ type: 'requestClue' });
      });

      it('leaves Shift+Tab to the browser', () => {
        expect(keyToAction(key('Tab', { shiftKey: true }), c)).toBeNull();
      });

      it('maps ? to toggleHelp', () => {
        expect(keyToAction(key('?', { shiftKey: true }), c)).toEqual({ type: 'toggleHelp' });
      });

      it('maps Ctrl+R to resetRound', () => {
        expect(keyToAction(key('r', { ctrlKey: true }), c)).toEqual({ type: 'resetRound' });
        expect(keyToAction(key('R', { ctrlKey: true }), c)).toEqual({ type: 'resetRound' });
      });

      it('ignores Cmd+R and Ctrl+Shift+R', () => {
        expect(keyToAction(key('r', { metaKey: true }), c)).toBeNull();
        expect(keyToAction(key('R', { ctrlKey: true, shiftKey: true }), c)).toBeNull();
      });

      it('ignores unlisted Ctrl and Meta chords', () => {
        expect(keyToAction(key('a', { ctrlKey: true }), c)).toBeNull();
        expect(keyToAction(key('a', { metaKey: true }), c)).toBeNull();
        expect(keyToAction(key('Backspace', { metaKey: true }), c)).toBeNull();
      });

      it('ignores digits and unlisted keys', () => {
        for (const d of DIGITS) {
          expect(keyToAction(key(d), c)).toBeNull();
        }
        expect(keyToAction(key(' '), c)).toBeNull();
        expect(keyToAction(key('Enter'), c)).toBeNull();
      });

      it('ignores IME composition', () => {
        expect(keyToAction(key('a', { isComposing: true }), c)).toBeNull();
      });
    });
  }

  it('keeps answer keys active while help overlay is open', () => {
    expect(keyToAction(key('q'), ctx('playing', { helpOpen: true }))).toEqual({
      type: 'typeLetter',
      ch: 'q',
    });
  });

  it('suppresses repeat for one-shot actions but not for typing or cursor', () => {
    const c = ctx('playing');
    expect(keyToAction(key('Tab', { repeat: true }), c)).toBeNull();
    expect(keyToAction(key('?', { repeat: true }), c)).toBeNull();
    expect(keyToAction(key('r', { ctrlKey: true, repeat: true }), c)).toBeNull();
    expect(keyToAction(key('a', { repeat: true }), c)).toEqual({ type: 'typeLetter', ch: 'a' });
    expect(keyToAction(key('Backspace', { repeat: true }), c)).toEqual({ type: 'backspace' });
    expect(keyToAction(key('ArrowLeft', { repeat: true }), c)).toEqual({ type: 'cursorLeft' });
  });
});

describe('keyToAction: won (answer focus, SPEC-ui §3.6)', () => {
  const c = ctx('won');

  it('maps n and N to newPuzzle', () => {
    expect(keyToAction(key('n'), c)).toEqual({ type: 'newPuzzle' });
    expect(keyToAction(key('N', { shiftKey: true }), c)).toEqual({ type: 'newPuzzle' });
  });

  it('maps i to openCustomWord', () => {
    expect(keyToAction(key('i'), c)).toEqual({ type: 'openCustomWord' });
  });

  it('maps r and Ctrl+R to resetRound', () => {
    expect(keyToAction(key('r'), c)).toEqual({ type: 'resetRound' });
    expect(keyToAction(key('r', { ctrlKey: true }), c)).toEqual({ type: 'resetRound' });
  });

  it('maps ? to toggleHelp', () => {
    expect(keyToAction(key('?', { shiftKey: true }), c)).toEqual({ type: 'toggleHelp' });
  });

  it('does nothing for other letters, digits, arrows, Backspace, and Tab', () => {
    for (const l of LETTERS.filter((l) => l !== 'n' && l !== 'i' && l !== 'r')) {
      expect(keyToAction(key(l), c)).toBeNull();
    }
    for (const d of DIGITS) {
      expect(keyToAction(key(d), c)).toBeNull();
    }
    expect(keyToAction(key('ArrowLeft'), c)).toBeNull();
    expect(keyToAction(key('Backspace'), c)).toBeNull();
    expect(keyToAction(key('Tab'), c)).toBeNull();
  });

  it('ignores unlisted Ctrl chords such as Ctrl+N', () => {
    expect(keyToAction(key('n', { ctrlKey: true }), c)).toBeNull();
  });
});

describe('keyToAction: custom word input (SPEC-ui §3.6)', () => {
  const c = ctx('customWordInput');

  it('maps a-z to customWordType with lowercase ch', () => {
    for (const l of LETTERS) {
      expect(keyToAction(key(l), c)).toEqual({ type: 'customWordType', ch: l });
    }
    expect(keyToAction(key('Q', { shiftKey: true }), c)).toEqual({ type: 'customWordType', ch: 'q' });
  });

  it('maps Backspace to customWordBackspace', () => {
    expect(keyToAction(key('Backspace'), c)).toEqual({ type: 'customWordBackspace' });
  });

  it('maps Enter to submitCustomWord', () => {
    expect(keyToAction(key('Enter'), c)).toEqual({ type: 'submitCustomWord' });
  });

  it('maps Esc to cancelCustomWord', () => {
    expect(keyToAction(key('Escape'), c)).toEqual({ type: 'cancelCustomWord' });
  });

  it('ignores Ctrl+letter, digits, and arrows (GUI deviation from TUI)', () => {
    expect(keyToAction(key('a', { ctrlKey: true }), c)).toBeNull();
    expect(keyToAction(key('1'), c)).toBeNull();
    expect(keyToAction(key('ArrowUp'), c)).toBeNull();
  });
});

describe('keyToAction: custom word pick (SPEC-ui §3.6)', () => {
  const c = ctx('customWordPick');

  it('maps ArrowUp and k to customPickDelta -1', () => {
    expect(keyToAction(key('ArrowUp'), c)).toEqual({ type: 'customPickDelta', delta: -1 });
    expect(keyToAction(key('k'), c)).toEqual({ type: 'customPickDelta', delta: -1 });
  });

  it('maps ArrowDown and j to customPickDelta +1', () => {
    expect(keyToAction(key('ArrowDown'), c)).toEqual({ type: 'customPickDelta', delta: 1 });
    expect(keyToAction(key('j'), c)).toEqual({ type: 'customPickDelta', delta: 1 });
  });

  it('maps Enter to confirmCustomPick', () => {
    expect(keyToAction(key('Enter'), c)).toEqual({ type: 'confirmCustomPick' });
  });

  it('maps u and Esc to cancelCustomWord', () => {
    expect(keyToAction(key('u'), c)).toEqual({ type: 'cancelCustomWord' });
    expect(keyToAction(key('U', { shiftKey: true }), c)).toEqual({ type: 'cancelCustomWord' });
    expect(keyToAction(key('Escape'), c)).toEqual({ type: 'cancelCustomWord' });
  });

  it('does nothing for other letters, Backspace, and Ctrl+R', () => {
    expect(keyToAction(key('a'), c)).toBeNull();
    expect(keyToAction(key('Backspace'), c)).toBeNull();
    expect(keyToAction(key('r', { ctrlKey: true }), c)).toBeNull();
  });

  it('repeats Arrow and j/k deltas, but not Enter or u', () => {
    expect(keyToAction(key('ArrowDown', { repeat: true }), c)).toEqual({
      type: 'customPickDelta',
      delta: 1,
    });
    expect(keyToAction(key('Enter', { repeat: true }), c)).toBeNull();
    expect(keyToAction(key('u', { repeat: true }), c)).toBeNull();
  });
});

describe('keyToAction: global Esc precedence (SPEC-ui §3.1, §3.6)', () => {
  it('help open in playing → toggleHelp', () => {
    expect(keyToAction(key('Escape'), ctx('playing', { helpOpen: true }))).toEqual({
      type: 'toggleHelp',
    });
  });

  it('help open in help phase → toggleHelp', () => {
    expect(keyToAction(key('Escape'), ctx('help', { helpOpen: true }))).toEqual({
      type: 'toggleHelp',
    });
  });

  it('help open in won → toggleHelp', () => {
    expect(keyToAction(key('Escape'), ctx('won', { helpOpen: true }))).toEqual({
      type: 'toggleHelp',
    });
  });

  it('nothing open in playing or won → no-op (never quits)', () => {
    expect(keyToAction(key('Escape'), ctx('playing'))).toBeNull();
    expect(keyToAction(key('Escape'), ctx('won'))).toBeNull();
  });

  it('Ctrl+Esc and Cmd+Esc are unlisted chords → no-op', () => {
    expect(keyToAction(key('Escape', { ctrlKey: true }), ctx('playing', { helpOpen: true }))).toBeNull();
    expect(keyToAction(key('Escape', { metaKey: true }), ctx('customWordInput'))).toBeNull();
  });

  it('dialog open → keyToAction returns null for Esc; isDialogCancelKey is true', () => {
    const c = ctx('playing', { dialogOpen: true });
    expect(keyToAction(key('Escape'), c)).toBeNull();
    expect(isDialogCancelKey(key('Escape'), c)).toBe(true);
  });

  it('dialog beats help: Esc cancels the dialog, not help', () => {
    const c = ctx('playing', { dialogOpen: true, helpOpen: true });
    expect(keyToAction(key('Escape'), c)).toBeNull();
    expect(isDialogCancelKey(key('Escape'), c)).toBe(true);
  });

  it('dialog open → every other key returns null and is not a cancel', () => {
    const c = ctx('playing', { dialogOpen: true, helpOpen: true });
    const others = [
      key('a'),
      key('Tab'),
      key('ArrowLeft'),
      key('Backspace'),
      key('?', { shiftKey: true }),
      key('r', { ctrlKey: true }),
      key('Enter'),
    ];
    for (const k of others) {
      expect(keyToAction(k, c)).toBeNull();
      expect(isDialogCancelKey(k, c)).toBe(false);
    }
  });

  it('dialog cancel ignores IME composition and repeats', () => {
    const c = ctx('playing', { dialogOpen: true });
    expect(isDialogCancelKey(key('Escape', { isComposing: true }), c)).toBe(false);
    expect(isDialogCancelKey(key('Escape', { repeat: true }), c)).toBe(false);
  });

  it('no dialog → isDialogCancelKey is false', () => {
    expect(isDialogCancelKey(key('Escape'), ctx('playing'))).toBe(false);
  });

  it('custom input and pick → cancelCustomWord', () => {
    expect(keyToAction(key('Escape'), ctx('customWordInput'))).toEqual({ type: 'cancelCustomWord' });
    expect(keyToAction(key('Escape'), ctx('customWordPick'))).toEqual({ type: 'cancelCustomWord' });
  });

  it('IME composition suppresses Esc everywhere', () => {
    expect(keyToAction(key('Escape', { isComposing: true }), ctx('customWordInput'))).toBeNull();
  });
});
