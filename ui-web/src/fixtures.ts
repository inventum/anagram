import type { RoundDto, ThemeDto, UiState, WorkbenchDto } from './types';

// Hand-written UiState snapshots for ?fixture=<name> (dev only).
// `playing` and `playingWorkbench` must contain no solution.

const theme: ThemeDto = {
  emptySlot: '#ff00ff',
  filledSlot: '#ffff00',
  pool: '#00ffff',
  correctPosition: '#00ff00',
};

const noCustom = { input: '', choices: [], pick: 0 };

const playingRound: RoundDto = {
  length: 5,
  pool: ['r', 'e', 't', 'c', 'a'],
  answer: [null, null, null, null, null],
  cursor: 0,
  correct: null,
  solution: null,
};

const playing: UiState = {
  phase: 'playing',
  helpOpen: false,
  length: 5,
  minSize: 4,
  maxSize: 10,
  clues: { used: 0, budget: 3 },
  activeClue: null,
  status: { kind: 'none', message: null },
  round: playingRound,
  workbench: null,
  custom: noCustom,
  theme,
};

const playingWorkbenchDto: WorkbenchDto = {
  layout: 'grid',
  rows: 5,
  cols: 5,
  cells: Array.from({ length: 25 }, (_, i) => (i === 0 ? 'c' : null)),
  placed: [],
  pool: ['r', 'e', 't', 'a'],
  cursor: 0,
  held: null,
  focus: 'answer',
  affixes: { starts: ['cr', 'ca'], ends: ['-e', '-te'] },
};

export const FIXTURES = {
  playing,

  help: { ...playing, phase: 'help', helpOpen: true },

  won: {
    ...playing,
    phase: 'won',
    clues: { used: 2, budget: 3 },
    activeClue: 'Starts with "c"',
    status: { kind: 'info', message: 'Correct!' },
    round: {
      length: 5,
      pool: [],
      answer: ['c', 'r', 'a', 't', 'e'],
      cursor: 4,
      correct: [true, true, true, true, true],
      solution: 'crate',
    },
  },

  customInput: {
    ...playing,
    phase: 'customWordInput',
    clues: { used: 0, budget: 0 },
    status: { kind: 'info', message: 'Type the scrambled letters, then press Enter.' },
    round: null,
    custom: { input: 'cra', choices: [], pick: 0 },
  },

  customPick: {
    ...playing,
    phase: 'customWordPick',
    clues: { used: 0, budget: 0 },
    status: { kind: 'info', message: '3 possible words. Pick one and press Enter.' },
    round: null,
    custom: {
      input: 'crate',
      choices: [
        { word: 'crate', definition: 'a wooden box used for packing or shipping' },
        { word: 'trace', definition: 'a mark or track left by something that has passed' },
        { word: 'cater', definition: 'to provide food and services' },
      ],
      pick: 0,
    },
  },

  playingWorkbench: {
    ...playing,
    clues: { used: 1, budget: 3 },
    activeClue: 'Starts with "c"',
    workbench: playingWorkbenchDto,
  },
} satisfies Record<string, UiState>;

export type FixtureName = keyof typeof FIXTURES;
