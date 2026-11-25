import type { RenderCtx } from '../state';
import { isDialogCancelKey, keyToAction, type KeymapCtx } from './keymap';

/** Keys the app handles, so the browser default is suppressed (SPEC-ui §4.4). */
const PREVENT_DEFAULT_KEYS: ReadonlySet<string> = new Set([
  'Tab',
  'ArrowLeft',
  'ArrowRight',
  'ArrowUp',
  'ArrowDown',
  'Backspace',
]);

/** Attaches the single window keydown listener. Reads live state on every event. */
export function bindInput(ctx: RenderCtx): void {
  window.addEventListener('keydown', (event: KeyboardEvent) => {
    const keymapCtx: KeymapCtx = {
      phase: ctx.state.phase,
      helpOpen: ctx.state.helpOpen,
      dialogOpen: ctx.local.pending !== null,
    };

    if (isDialogCancelKey(event, keymapCtx)) {
      ctx.setLocal({ pending: null, sliderDraft: null });
      return;
    }

    const action = keyToAction(event, keymapCtx);
    if (action === null) {
      return;
    }

    if (PREVENT_DEFAULT_KEYS.has(event.key) || (event.ctrlKey && action.type === 'resetRound')) {
      event.preventDefault();
    }

    if (action.type === 'newPuzzle' || action.type === 'openCustomWord') {
      ctx.requestReplace(action);
    } else {
      ctx.dispatch(action);
    }
  });
}
