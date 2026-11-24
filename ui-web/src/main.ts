import './styles/layout.css';
import './styles/theme.css';
import './styles/focus.css';
import { getState, dispatch as sendAction } from './ipc';
import { applyTheme } from './theme-apply';
import { renderApp } from './render';
import { guardedDispatch } from './overlays';
import { bindInput } from './input';
import { applyA11y } from './a11y';
import type { RenderCtx } from './state';

const root = document.getElementById('app');
if (root === null) {
  throw new Error('Missing #app root element');
}
const appRoot: HTMLElement = root;

function rerender(ctx: RenderCtx): void {
  renderApp(appRoot, ctx);
  applyA11y(appRoot);
}

async function start(): Promise<void> {
  const initial = await getState();
  const ctx: RenderCtx = {
    state: initial,
    local: { pending: null, sliderDraft: null },
    dispatch(a) {
      sendAction(a).then(
        (next) => {
          ctx.state = next;
          rerender(ctx);
        },
        (err: unknown) => console.error(err),
      );
    },
    requestReplace(a) {
      guardedDispatch(a, ctx);
    },
    setLocal(patch) {
      Object.assign(ctx.local, patch);
      rerender(ctx);
    },
  };

  applyTheme(initial.theme);
  renderApp(appRoot, ctx);
  bindInput(ctx);
  applyA11y(appRoot);
}

start().catch((err: unknown) => {
  console.error(err);
  appRoot.textContent = 'Could not load game state.';
});
