// Workbench-only styles, injected once. Keeps the pointer panel self-contained
// (CSP allows 'unsafe-inline' styles; no external CSS file is owned here).

const STYLE_ID = 'wb-styles';

const CSS = `
.wb-panel { display: flex; flex-direction: column; gap: 10px; padding: 12px; border: 1px solid #3a3a3a; border-radius: 8px; box-sizing: border-box; min-width: 0; }
.wb-panel[hidden] { display: none; }
.wb-panel.wb-focused { box-shadow: 0 0 0 2px #7aa2ff; }
.wb-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.wb-title { margin: 0; font-size: 1rem; }
.wb-label { margin: 0; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.04em; opacity: 0.8; min-width: 3.5rem; }
.wb-clear { font: inherit; padding: 4px 10px; cursor: pointer; }
.wb-body { display: flex; flex-wrap: wrap; gap: 12px; align-items: flex-start; }
.wb-grid { display: grid; gap: 4px; width: min(100%, 260px); aspect-ratio: 1 / 1; box-sizing: border-box; }
.wb-grid[hidden] { display: none; }
.wb-cell { display: flex; align-items: center; justify-content: center; min-width: 0; min-height: 0; border: 1px dashed #666; border-radius: 4px; cursor: pointer; user-select: none; text-transform: uppercase; font-weight: 600; }
.wb-cell.wb-filled { border-style: solid; color: var(--slot-filled, #ffff00); }
.wb-cell:hover, .wb-tile:hover { background: rgba(255, 255, 255, 0.08); }
.wb-side { display: flex; flex-direction: column; gap: 10px; flex: 1 1 180px; min-width: 0; }
.wb-row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.wb-hold { display: flex; align-items: center; justify-content: center; width: 36px; height: 36px; box-sizing: border-box; border: 1px dashed #666; border-radius: 4px; text-transform: uppercase; font-weight: 600; }
.wb-hold:not(:empty) { border-style: solid; color: var(--pool, #00ffff); }
.wb-pool { display: flex; flex-wrap: wrap; gap: 6px; }
.wb-tile { display: inline-flex; align-items: center; justify-content: center; min-width: 32px; height: 32px; padding: 0 6px; box-sizing: border-box; border: 1px solid #666; border-radius: 4px; cursor: pointer; user-select: none; text-transform: uppercase; font-weight: 600; color: var(--pool, #00ffff); }
.wb-chips { display: flex; flex-wrap: wrap; gap: 4px; }
.wb-chip { font-family: ui-monospace, monospace; padding: 2px 8px; border: 1px solid #666; border-radius: 999px; }
.wb-none { opacity: 0.6; font-size: 0.8rem; }
`;

export function ensureWorkbenchStyles(): void {
  if (document.getElementById(STYLE_ID)) return;
  const style = document.createElement('style');
  style.id = STYLE_ID;
  style.textContent = CSS;
  document.head.append(style);
}
