import type { ActionDto } from '../types';
import type { RenderCtx } from '../state';

export function renderOverlays(root: HTMLElement, ctx: RenderCtx): void {}

export function guardedDispatch(action: ActionDto, ctx: RenderCtx): void {}
