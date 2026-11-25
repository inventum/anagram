import type { ThemeDto } from './types';

// Config colors arrive as CSS color strings (backend maps names; frontend uses
// them as-is, SPEC-ui §6). Inline values on :root override theme.css defaults.
// An empty string removes the override, so the theme.css default applies.
const THEME_PROPS: ReadonlyArray<readonly [keyof ThemeDto, string]> = [
  ['emptySlot', '--slot-empty'],
  ['filledSlot', '--slot-filled'],
  ['pool', '--pool'],
  ['correctPosition', '--correct'],
];

export function applyTheme(theme: ThemeDto): void {
  const style = document.documentElement.style;
  for (const [key, prop] of THEME_PROPS) {
    const value = theme[key];
    if (value === '') {
      style.removeProperty(prop);
    } else {
      style.setProperty(prop, value);
    }
  }
}
