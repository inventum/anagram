import { invoke } from '@tauri-apps/api/core';
import type { FixtureName } from './fixtures';
import type { ActionDto, UiState } from './types';

export function isTauri(): boolean {
  return '__TAURI_INTERNALS__' in window;
}

// Dev-only: outside Tauri, ?fixture=<name> returns a canned UiState.
// fixtures.ts is dynamically imported so production bundles omit answer data.
async function fixtureFromUrl(): Promise<UiState | null> {
  if (!import.meta.env.DEV || isTauri()) {
    return null;
  }
  const name = new URLSearchParams(window.location.search).get('fixture');
  if (name === null) {
    return null;
  }
  const { FIXTURES } = await import('./fixtures');
  if (!Object.prototype.hasOwnProperty.call(FIXTURES, name)) {
    throw new Error(`Unknown fixture: ${name}`);
  }
  return FIXTURES[name as FixtureName];
}

async function resolveFixture(): Promise<UiState | null> {
  const fixture = await fixtureFromUrl();
  if (fixture === null && !isTauri()) {
    throw new Error('Not running in Tauri and no ?fixture= given');
  }
  return fixture;
}

// D-15: all IPC calls are queued FIFO so focus-then-click pairs apply in order.
let queue: Promise<unknown> = Promise.resolve();

function enqueue<T>(task: () => Promise<T>): Promise<T> {
  const run = queue.then(task);
  queue = run.catch(() => undefined);
  return run;
}

export function getState(): Promise<UiState> {
  return enqueue(async () => {
    const fixture = await resolveFixture();
    return fixture ?? invoke<UiState>('get_state');
  });
}

export function dispatch(action: ActionDto): Promise<UiState> {
  return enqueue(async () => {
    const fixture = await resolveFixture();
    return fixture ?? invoke<UiState>('dispatch', { action });
  });
}
