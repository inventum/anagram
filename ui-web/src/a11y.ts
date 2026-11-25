// WP-13 accessibility. main.ts calls applyA11y after every render.
// Idempotent: attributes are set to deterministic values each call. Focus moves
// only on dialog open/close transitions. Module scope has no DOM access (testable).

const DIALOG_SELECTOR = '.overlay__panel';
const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

interface DialogFocus {
  /** Topmost open dialog, or null. */
  dialog: HTMLElement | null;
  /** Control that opened the current dialog stack; focus returns here on close. */
  trigger: HTMLElement | null;
  /** Most recent focusable control outside any dialog (pointer or keyboard). */
  lastOutside: HTMLElement | null;
  listening: boolean;
}

const focusState: DialogFocus = { dialog: null, trigger: null, lastOutside: null, listening: false };

/**
 * Index to focus for a Tab press inside a dialog. Wraps at both ends.
 * Returns -1 when there is nothing to focus.
 */
export function nextTabIndex(count: number, active: number, shift: boolean): number {
  if (count <= 0) {
    return -1;
  }
  if (active < 0 || active >= count) {
    return shift ? count - 1 : 0;
  }
  if (shift) {
    return active === 0 ? count - 1 : active - 1;
  }
  return active === count - 1 ? 0 : active + 1;
}

/** Accessible name for an answer slot. Uses text so state is not conveyed by color alone. */
export function slotLabel(index: number, letter: string | null, correct: boolean): string {
  const base = letter === null ? `slot ${index + 1}, empty` : `slot ${index + 1}, letter ${letter}`;
  return correct ? `${base}, correct` : base;
}

function isShown(node: HTMLElement): boolean {
  return node.closest('[hidden]') === null;
}

function focusablesIn(dialog: HTMLElement): HTMLElement[] {
  return Array.from(dialog.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(isShown);
}

function openDialogs(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(DIALOG_SELECTOR)).filter(isShown);
}

function outsideControl(target: EventTarget | null): HTMLElement | null {
  if (!(target instanceof HTMLElement) || target.closest(DIALOG_SELECTOR) !== null) {
    return null;
  }
  return target.closest<HTMLElement>(FOCUSABLE_SELECTOR);
}

/** Keeps Tab inside the open dialog. Also stops Tab reaching the keymap (no clue request while modal). */
function trapTab(event: KeyboardEvent): void {
  const dialog = focusState.dialog;
  if (event.key !== 'Tab' || event.isComposing || dialog === null) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const items = focusablesIn(dialog);
  if (items.length === 0) {
    dialog.focus();
    return;
  }
  const active = items.findIndex((node) => node === document.activeElement);
  items[nextTabIndex(items.length, active, event.shiftKey)]?.focus();
}

function installListeners(): void {
  if (focusState.listening) {
    return;
  }
  focusState.listening = true;
  const remember = (target: EventTarget | null): void => {
    const control = outsideControl(target);
    if (control !== null) {
      focusState.lastOutside = control;
    }
  };
  // pointerdown: WebKit (Tauri on macOS) does not focus buttons on click.
  document.addEventListener('pointerdown', (event) => remember(event.target), true);
  document.addEventListener('focusin', (event) => remember(event.target), true);
  window.addEventListener('keydown', trapTab, true);
}

function applyStaticAttributes(root: HTMLElement): void {
  root.querySelectorAll<HTMLElement>('[role="status"]').forEach((node) => {
    node.setAttribute('aria-live', 'polite');
  });

  root.querySelectorAll<HTMLElement>('.overlay__status').forEach((node) => {
    node.setAttribute('aria-live', 'polite');
  });

  root.querySelectorAll<HTMLElement>(DIALOG_SELECTOR).forEach((dialog) => {
    dialog.setAttribute('role', 'dialog');
    dialog.setAttribute('aria-modal', 'true');
    dialog.setAttribute('tabindex', '-1');
    if (!dialog.hasAttribute('aria-labelledby')) {
      const title = dialog.querySelector<HTMLElement>('h1[id], h2[id], h3[id]');
      if (title !== null) {
        dialog.setAttribute('aria-labelledby', title.id);
      }
    }
  });

  root.querySelectorAll<HTMLInputElement>('input[type="range"]').forEach((slider) => {
    slider.setAttribute('aria-label', 'Puzzle length');
    slider.setAttribute('aria-valuemin', slider.min);
    slider.setAttribute('aria-valuemax', slider.max);
    slider.setAttribute('aria-valuenow', slider.value);
  });

  root.querySelectorAll<HTMLButtonElement>('button').forEach((button) => {
    if (button.hasAttribute('aria-label') || (button.textContent ?? '').trim() !== '') {
      return;
    }
    const title = button.getAttribute('title');
    if (title !== null && title !== '') {
      button.setAttribute('aria-label', title);
    }
  });

  root.querySelectorAll<HTMLElement>('.answer .slot').forEach((slot, index) => {
    const letter = (slot.querySelector('.slot__letter')?.textContent ?? '').trim();
    slot.setAttribute(
      'aria-label',
      slotLabel(index, letter === '' ? null : letter, slot.classList.contains('is-correct')),
    );
  });
}

function syncDialogFocus(root: HTMLElement): void {
  const open = openDialogs(root);
  const next = open.length > 0 ? open[open.length - 1] : null;
  const prev = focusState.dialog;
  if (next === prev) {
    return;
  }
  focusState.dialog = next;

  if (next !== null) {
    if (prev === null) {
      // Capture the trigger before focus moves into the dialog.
      const active = document.activeElement;
      const activeControl = active instanceof HTMLElement ? outsideControl(active) : null;
      focusState.trigger = activeControl ?? focusState.lastOutside;
    }
    if (!next.contains(document.activeElement)) {
      const first = focusablesIn(next)[0];
      (first ?? next).focus();
    }
    return;
  }

  // Dialog stack closed: restore focus if it was lost or was inside the closed dialog.
  const trigger = focusState.trigger;
  focusState.trigger = null;
  focusState.lastOutside = null;
  const active = document.activeElement;
  const focusLost = active === null || active === document.body || (prev !== null && prev.contains(active));
  if (focusLost && trigger !== null && trigger.isConnected && isShown(trigger)) {
    trigger.focus();
  }
}

export function applyA11y(root: HTMLElement): void {
  installListeners();
  applyStaticAttributes(root);
  syncDialogFocus(root);
}
