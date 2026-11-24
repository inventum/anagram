import type { ActionDto, UiState } from './types';

export interface LocalState {
  pending: ActionDto | null;
  sliderDraft: number | null;
}

export interface RenderCtx {
  state: UiState;
  local: LocalState;
  dispatch(a: ActionDto): void;
  requestReplace(a: ActionDto): void;
  setLocal(patch: Partial<LocalState>): void;
}
