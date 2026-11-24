import type { ActionDto, PhaseDto } from '../types';

/** Structural subset of KeyboardEvent. Pure input for mapping and tests. */
export interface KeyLike {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  isComposing: boolean;
  repeat: boolean;
}

export interface KeymapCtx {
  phase: PhaseDto;
  helpOpen: boolean;
  /** True while a confirm dialog is open (local.pending !== null). */
  dialogOpen: boolean;
}

/** Actions that may auto-repeat while a key is held. One-shot actions never repeat. */
const REPEATABLE: ReadonlySet<ActionDto['type']> = new Set<ActionDto['type']>([
  'typeLetter',
  'backspace',
  'cursorLeft',
  'cursorRight',
  'customWordType',
  'customWordBackspace',
  'customPickDelta',
]);

function isLetter(key: string): boolean {
  return /^[a-zA-Z]$/.test(key);
}

function isChord(k: KeyLike): boolean {
  return k.ctrlKey || k.metaKey;
}

/** Ctrl+R only. Cmd+R and Ctrl+Shift+R are unlisted chords. */
function isCtrlR(k: KeyLike): boolean {
  return k.ctrlKey && !k.metaKey && !k.shiftKey && k.key.toLowerCase() === 'r';
}

function isHelpPhase(phase: PhaseDto): boolean {
  return phase === 'playing' || phase === 'help' || phase === 'won';
}

function isCustomPhase(phase: PhaseDto): boolean {
  return phase === 'customWordInput' || phase === 'customWordPick';
}

/** Letters are case-insensitive. Non-letters pass through unchanged. */
function normalize(key: string): string {
  return isLetter(key) ? key.toLowerCase() : key;
}

/** Maps a key event to a game action (SPEC-ui §3.6). Returns null when the key is unbound. */
export function keyToAction(k: KeyLike, ctx: KeymapCtx): ActionDto | null {
  const action = resolve(k, ctx);
  if (action === null) {
    return null;
  }
  if (k.repeat && !REPEATABLE.has(action.type)) {
    return null;
  }
  return action;
}

/**
 * True when Esc should close the confirm dialog (Cancel). The caller clears
 * `pending` and `sliderDraft`. `keyToAction` returns null in this case.
 */
export function isDialogCancelKey(k: KeyLike, ctx: KeymapCtx): boolean {
  return ctx.dialogOpen && k.key === 'Escape' && !isChord(k) && !k.isComposing && !k.repeat;
}

function resolve(k: KeyLike, ctx: KeymapCtx): ActionDto | null {
  // Confirm dialog is modal. Esc is handled by isDialogCancelKey; nothing else reaches the reducer.
  if (k.isComposing || ctx.dialogOpen) {
    return null;
  }

  if (k.key === 'Escape') {
    return isChord(k) ? null : escapeAction(ctx);
  }

  if (isChord(k)) {
    return isCtrlR(k) && isHelpPhase(ctx.phase) ? { type: 'resetRound' } : null;
  }

  switch (ctx.phase) {
    case 'playing':
    case 'help':
      return answerKey(k);
    case 'won':
      return wonKey(k);
    case 'customWordInput':
      return customInputKey(k);
    case 'customWordPick':
      return customPickKey(k);
  }
}

/** Global Esc precedence, topmost first: help overlay, then custom word input or pick. */
function escapeAction(ctx: KeymapCtx): ActionDto | null {
  if (ctx.helpOpen && isHelpPhase(ctx.phase)) {
    return { type: 'toggleHelp' };
  }
  if (isCustomPhase(ctx.phase)) {
    return { type: 'cancelCustomWord' };
  }
  return null;
}

function answerKey(k: KeyLike): ActionDto | null {
  if (isLetter(k.key)) {
    return { type: 'typeLetter', ch: k.key.toLowerCase() };
  }
  switch (k.key) {
    case 'ArrowLeft':
      return { type: 'cursorLeft' };
    case 'ArrowRight':
      return { type: 'cursorRight' };
    case 'Backspace':
      return { type: 'backspace' };
    case 'Tab':
      // Shift+Tab stays with the browser for reverse focus navigation.
      return k.shiftKey ? null : { type: 'requestClue' };
    case '?':
      return { type: 'toggleHelp' };
    default:
      return null;
  }
}

function wonKey(k: KeyLike): ActionDto | null {
  switch (normalize(k.key)) {
    case 'n':
      return { type: 'newPuzzle' };
    case 'i':
      return { type: 'openCustomWord' };
    case 'r':
      return { type: 'resetRound' };
    case '?':
      return { type: 'toggleHelp' };
    default:
      return null;
  }
}

function customInputKey(k: KeyLike): ActionDto | null {
  if (isLetter(k.key)) {
    return { type: 'customWordType', ch: k.key.toLowerCase() };
  }
  switch (k.key) {
    case 'Backspace':
      return { type: 'customWordBackspace' };
    case 'Enter':
      return { type: 'submitCustomWord' };
    default:
      return null;
  }
}

function customPickKey(k: KeyLike): ActionDto | null {
  switch (normalize(k.key)) {
    case 'ArrowUp':
    case 'k':
      return { type: 'customPickDelta', delta: -1 };
    case 'ArrowDown':
    case 'j':
      return { type: 'customPickDelta', delta: 1 };
    case 'Enter':
      return { type: 'confirmCustomPick' };
    case 'u':
      return { type: 'cancelCustomWord' };
    default:
      return null;
  }
}
