// TypeScript mirror of SPEC-ui §5.3 (UiState) and §5.4 (ActionDto).
// Wire format is camelCase (serde rename_all). Types only: no game logic.

export type PhaseDto = 'customWordInput' | 'customWordPick' | 'playing' | 'help' | 'won';

export type StatusKind = 'none' | 'error' | 'info' | 'wrongAnswer';

export type LayoutDto = 'grid' | 'free';

export type FocusDto = 'answer' | 'grid';

export interface ClueDto {
  used: number;
  budget: number;
}

export interface StatusDto {
  kind: StatusKind;
  message: string | null;
}

export interface RoundDto {
  length: number;
  pool: string[];
  answer: (string | null)[];
  cursor: number;
  /** Present only while highlightCorrectPositions is on. */
  correct?: boolean[];
  /** Present only when phase is 'won'. */
  solution?: string;
  /** Present only when phase is 'won' and the solved word has a definition. */
  definition?: string;
}

export interface PlacedDto {
  ch: string;
  x: number;
  y: number;
}

export interface AffixDto {
  starts: string[];
  ends: string[];
}

export interface WorkbenchDto {
  layout: LayoutDto;
  /** Grid layout only. */
  rows: number;
  /** Grid layout only. */
  cols: number;
  /** Grid layout only: rows × cols, row-major. */
  cells: (string | null)[];
  /** Free layout only (option A). */
  placed: PlacedDto[];
  /** Workbench's own pool; not RoundDto.pool. */
  pool: string[];
  cursor: number;
  held: string | null;
  focus: FocusDto;
  affixes: AffixDto;
}

export interface ChoiceDto {
  word: string;
  definition: string;
}

export interface CustomDto {
  input: string;
  choices: ChoiceDto[];
  pick: number;
}

export interface ThemeDto {
  emptySlot: string;
  filledSlot: string;
  pool: string;
  correctPosition: string;
}

export interface UiState {
  phase: PhaseDto;
  helpOpen: boolean;
  length: number;
  minSize: number;
  maxSize: number;
  clues: ClueDto;
  activeClue: string | null;
  status: StatusDto;
  round?: RoundDto;
  workbench?: WorkbenchDto;
  custom: CustomDto;
  theme: ThemeDto;
}

export type ActionDto =
  | { type: 'toggleHelp' }
  | { type: 'newPuzzle' }
  | { type: 'resetRound' }
  | { type: 'requestClue' }
  | { type: 'backspace' }
  | { type: 'cursorLeft' }
  | { type: 'cursorRight' }
  | { type: 'setCursor'; idx: number }
  | { type: 'typeLetter'; ch: string }
  | { type: 'selectSize'; n: number }
  | { type: 'openCustomWord' }
  | { type: 'customWordType'; ch: string }
  | { type: 'customWordBackspace' }
  | { type: 'submitCustomWord' }
  | { type: 'cancelCustomWord' }
  | { type: 'customPickDelta'; delta: number }
  | { type: 'customPickSelect'; idx: number }
  | { type: 'confirmCustomPick' }
  | { type: 'workbenchClickCell'; idx: number }
  | { type: 'workbenchClickPool'; idx: number }
  | { type: 'workbenchSetFocus'; focus: FocusDto }
  | { type: 'workbenchClear' };
