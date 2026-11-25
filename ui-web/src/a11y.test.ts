import { describe, expect, it } from 'vitest';
import { nextTabIndex, slotLabel } from './a11y';

describe('nextTabIndex (dialog focus trap)', () => {
  it('returns -1 when the dialog has nothing focusable', () => {
    expect(nextTabIndex(0, -1, false)).toBe(-1);
  });

  it('enters at the first item on Tab from outside', () => {
    expect(nextTabIndex(3, -1, false)).toBe(0);
  });

  it('enters at the last item on Shift+Tab from outside', () => {
    expect(nextTabIndex(3, -1, true)).toBe(2);
  });

  it('steps forward and backward inside the dialog', () => {
    expect(nextTabIndex(3, 0, false)).toBe(1);
    expect(nextTabIndex(3, 2, true)).toBe(1);
  });

  it('wraps from last to first on Tab', () => {
    expect(nextTabIndex(3, 2, false)).toBe(0);
  });

  it('wraps from first to last on Shift+Tab', () => {
    expect(nextTabIndex(3, 0, true)).toBe(2);
  });

  it('keeps a single item focused', () => {
    expect(nextTabIndex(1, 0, false)).toBe(0);
    expect(nextTabIndex(1, 0, true)).toBe(0);
  });
});

describe('slotLabel', () => {
  it('names a filled slot with its letter', () => {
    expect(slotLabel(0, 'c', false)).toBe('slot 1, letter c');
  });

  it('names an empty slot', () => {
    expect(slotLabel(1, null, false)).toBe('slot 2, empty');
  });

  it('adds correct state as text', () => {
    expect(slotLabel(2, 'r', true)).toBe('slot 3, letter r, correct');
  });
});
