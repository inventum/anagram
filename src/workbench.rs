//! Letter workbench (cue grid) — pure state; no dictionary.

use std::collections::HashSet;

use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetterWorkbench {
    pub rows: u8,
    pub cols: u8,
    pub cells: Vec<Option<char>>,
    pub pool: Vec<char>,
    pub cursor: usize,
    pub held: Option<char>,
    /// Solution letters (uppercase, identity order). Used by the identity guard in `reset_grid`.
    solution: Vec<char>,
}

impl LetterWorkbench {
    pub fn from_solution(word: &str, rows: u8, cols: u8, rng: &mut impl Rng) -> Self {
        let solution: String = word.to_ascii_uppercase();
        let len = solution.len();
        let cell_count = (rows as usize) * (cols as usize);
        let identity: Vec<char> = solution.chars().collect();
        let mut pool = identity.clone();
        shuffle_not_identity(&mut pool, &identity, rng);
        let wb = Self {
            rows,
            cols,
            cells: vec![None; cell_count],
            pool,
            cursor: 0,
            held: None,
            solution: identity,
        };
        wb.debug_assert_invariant(len);
        wb
    }

    fn n_letters(&self) -> usize {
        let in_cells = self.cells.iter().filter(|c| c.is_some()).count();
        let held = usize::from(self.held.is_some());
        in_cells + self.pool.len() + held
    }

    fn debug_assert_invariant(&self, n: usize) {
        debug_assert!(self.cursor < self.cells.len());
        let in_cells = self.cells.iter().filter(|c| c.is_some()).count();
        let held = usize::from(self.held.is_some());
        debug_assert_eq!(in_cells + self.pool.len() + held, n);
        for c in self
            .cells
            .iter()
            .flatten()
            .chain(self.pool.iter())
            .chain(self.held.iter())
        {
            debug_assert!(c.is_ascii_alphabetic());
            debug_assert!(c.is_ascii_uppercase());
        }
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) -> bool {
        let cols = self.cols as i32;
        let rows = self.rows as i32;
        let col = (self.cursor as i32) % cols;
        let row = (self.cursor as i32) / cols;
        let nc = col + dx;
        let nr = row + dy;
        if nc < 0 || nc >= cols || nr < 0 || nr >= rows {
            return false;
        }
        self.cursor = (nr * cols + nc) as usize;
        true
    }

    pub fn pick_place_swap(&mut self) -> bool {
        let n = self.n_letters();
        let cursor = self.cursor;
        let cell = self.cells[cursor];
        let changed = match (self.held, cell) {
            (None, None) => {
                if self.pool.is_empty() {
                    return false;
                }
                let letter = self.pool.remove(0);
                self.cells[cursor] = Some(letter);
                true
            }
            (None, Some(letter)) => {
                self.held = Some(letter);
                self.cells[cursor] = None;
                true
            }
            (Some(held), None) => {
                self.cells[cursor] = Some(held);
                self.held = None;
                true
            }
            (Some(held), Some(cell_letter)) => {
                self.cells[cursor] = Some(held);
                self.held = Some(cell_letter);
                true
            }
        };
        self.debug_assert_invariant(n);
        changed
    }

    pub fn clear_cell(&mut self) -> bool {
        let n = self.n_letters();
        let cursor = self.cursor;
        let Some(letter) = self.cells[cursor].take() else {
            return false;
        };
        self.pool.push(letter);
        self.debug_assert_invariant(n);
        true
    }

    /// Pointer click on a cell. Returns `false` if out of range or a no-op (hand empty, cell empty).
    pub fn click_cell(&mut self, idx: usize) -> bool {
        if idx >= self.cells.len() {
            return false;
        }
        self.cursor = idx;
        let n = self.n_letters();
        match (self.held, self.cells[idx]) {
            (None, None) => return false,
            (None, Some(letter)) => {
                self.held = Some(letter);
                self.cells[idx] = None;
            }
            (Some(held), None) => {
                self.cells[idx] = Some(held);
                self.held = None;
            }
            (Some(held), Some(cell_letter)) => {
                self.cells[idx] = Some(held);
                self.held = Some(cell_letter);
            }
        }
        self.debug_assert_invariant(n);
        true
    }

    /// Pointer click on a workbench pool tile: pick it up. No-op while holding (D-1).
    pub fn click_pool(&mut self, idx: usize) -> bool {
        if self.held.is_some() || idx >= self.pool.len() {
            return false;
        }
        let n = self.n_letters();
        self.held = Some(self.pool.remove(idx));
        self.debug_assert_invariant(n);
        true
    }

    /// Type into the cursor cell from the grid pool (empty cell only).
    pub fn try_type_letter(&mut self, ch: char) -> bool {
        if !ch.is_ascii_alphabetic() {
            return false;
        }
        if self.cells[self.cursor].is_some() {
            return false;
        }
        let ch = ch.to_ascii_uppercase();
        let Some(pool_idx) = self.pool.iter().position(|&p| p == ch) else {
            return false;
        };
        let n = self.n_letters();
        let letter = self.pool.remove(pool_idx);
        self.cells[self.cursor] = Some(letter);
        self.debug_assert_invariant(n);
        true
    }

    pub fn reset_grid(&mut self, rng: &mut impl Rng) {
        let n = self.n_letters();
        let mut letters: Vec<char> = self.pool.clone();
        for cell in &self.cells {
            if let Some(ch) = cell {
                letters.push(*ch);
            }
        }
        if let Some(h) = self.held.take() {
            letters.push(h);
        }
        for cell in &mut self.cells {
            *cell = None;
        }
        shuffle_not_identity(&mut letters, &self.solution, rng);
        self.pool = letters;
        self.debug_assert_invariant(n);
    }
}

/// Shuffle avoiding the identity order when the word has more than one distinct letter.
/// Same rule as `game::shuffle_pool_not_identity` (SPEC.md §5.2); `game.rs` is unchanged.
fn shuffle_not_identity(letters: &mut [char], identity: &[char], rng: &mut impl Rng) {
    let distinct = identity.iter().collect::<HashSet<_>>().len();
    loop {
        letters.shuffle(rng);
        if distinct <= 1 || letters.iter().ne(identity.iter()) {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn wb(word: &str, rows: u8, cols: u8, seed: u64) -> LetterWorkbench {
        let mut rng = StdRng::seed_from_u64(seed);
        LetterWorkbench::from_solution(word, rows, cols, &mut rng)
    }

    fn assert_invariant(wb: &LetterWorkbench, n: usize) {
        let in_cells = wb.cells.iter().filter(|c| c.is_some()).count();
        let held = usize::from(wb.held.is_some());
        assert_eq!(in_cells + wb.pool.len() + held, n);
        assert!(wb.cursor < wb.cells.len());
        for c in wb
            .cells
            .iter()
            .flatten()
            .chain(wb.pool.iter())
            .chain(wb.held.iter())
        {
            assert!(c.is_ascii_uppercase());
        }
    }

    #[test]
    fn from_solution_uppercase_multiset_and_independent_shuffle() {
        let mut rng_a = StdRng::seed_from_u64(11);
        let mut rng_b = StdRng::seed_from_u64(11);
        let wb_a = LetterWorkbench::from_solution("Crate", 5, 5, &mut rng_a);
        let wb_b = LetterWorkbench::from_solution("crate", 5, 5, &mut rng_b);
        assert_eq!(wb_a.pool.len(), 5);
        assert_invariant(&wb_a, 5);
        assert_invariant(&wb_b, 5);
        let mut sorted = wb_a.pool.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec!['A', 'C', 'E', 'R', 'T']);
        let identity: Vec<char> = "CRATE".chars().collect();
        assert_ne!(wb_a.pool, identity);
    }

    #[test]
    fn type_letter_fills_cursor_from_pool() {
        let mut wb = wb("cat", 4, 4, 2);
        let target = wb.pool[wb.pool.len() - 1];
        assert!(wb.try_type_letter(target));
        assert_eq!(wb.cells[0], Some(target));
        assert!(!wb.pool.contains(&target));
        assert_invariant(&wb, 3);
        assert!(!wb.try_type_letter('z'));
        assert!(!wb.try_type_letter(target));
    }

    #[test]
    fn space_empty_hand_empty_cell_takes_leftmost_pool() {
        let mut rng = StdRng::seed_from_u64(3);
        let fresh = LetterWorkbench::from_solution("listen", 5, 5, &mut rng);
        let expected_first = fresh.pool[0];
        let mut wb = wb("listen", 5, 5, 3);
        assert!(wb.pick_place_swap());
        assert_eq!(wb.cells[0], Some(expected_first));
        assert_eq!(wb.pool.len(), 5);
        assert_invariant(&wb, 6);
        assert!(wb.held.is_none());
    }

    #[test]
    fn space_empty_empty_empty_pool_is_noop() {
        let mut wb = wb("ab", 5, 5, 1);
        wb.pool.clear();
        wb.cells[0] = Some('A');
        wb.cells[1] = Some('B');
        wb.cursor = 2;
        assert!(!wb.pick_place_swap());
        assert_invariant(&wb, 2);
    }

    #[test]
    fn space_pick_cell_into_hand() {
        let mut wb = wb("cat", 4, 4, 5);
        assert!(wb.pick_place_swap());
        let placed = wb.cells[0].unwrap();
        assert!(wb.pick_place_swap());
        assert_eq!(wb.held, Some(placed));
        assert_eq!(wb.cells[0], None);
        assert_invariant(&wb, 3);
    }

    #[test]
    fn space_place_hand_on_empty_cell() {
        let mut wb = wb("dog", 4, 4, 6);
        assert!(wb.pick_place_swap());
        assert!(wb.pick_place_swap());
        wb.move_cursor(1, 0);
        assert!(wb.pick_place_swap());
        assert!(wb.held.is_none());
        assert!(wb.cells[1].is_some());
        assert_invariant(&wb, 3);
    }

    #[test]
    fn space_swap_hand_and_cell() {
        let mut wb = wb("ab", 4, 4, 7);
        assert!(wb.pick_place_swap());
        let first = wb.cells[0].unwrap();
        wb.move_cursor(1, 0);
        assert!(wb.pick_place_swap());
        assert!(wb.pick_place_swap());
        let hand = wb.held.unwrap();
        wb.move_cursor(-1, 0);
        let cell0 = wb.cells[0].unwrap();
        assert!(wb.pick_place_swap());
        assert_eq!(wb.cells[0], Some(hand));
        assert_eq!(wb.held, Some(cell0));
        assert_ne!(wb.cells[0], Some(first));
        assert_invariant(&wb, 2);
    }

    #[test]
    fn clear_cell_appends_to_pool_end() {
        let mut wb = wb("hi", 5, 5, 8);
        assert!(wb.pick_place_swap());
        let placed = wb.cells[0].unwrap();
        let before = wb.pool.len();
        assert!(wb.clear_cell());
        assert_eq!(wb.cells[0], None);
        assert_eq!(wb.pool.len(), before + 1);
        assert_eq!(wb.pool.last(), Some(&placed));
        assert_invariant(&wb, 2);
    }

    #[test]
    fn clear_empty_cell_is_noop() {
        let mut wb = wb("hi", 5, 5, 9);
        wb.cursor = 0;
        assert!(!wb.clear_cell());
        assert_invariant(&wb, 2);
    }

    #[test]
    fn reset_grid_clears_held_and_cells_reshuffles_pool() {
        let mut wb = wb("crate", 5, 5, 10);
        assert!(wb.pick_place_swap());
        wb.move_cursor(1, 0);
        assert!(wb.pick_place_swap());
        assert!(wb.pick_place_swap());
        let mut rng = StdRng::seed_from_u64(99);
        wb.reset_grid(&mut rng);
        assert!(wb.cells.iter().all(|c| c.is_none()));
        assert_eq!(wb.held, None);
        assert_eq!(wb.pool.len(), 5);
        assert_invariant(&wb, 5);
        let mut sorted = wb.pool.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec!['A', 'C', 'E', 'R', 'T']);
    }

    #[test]
    fn move_cursor_bounds_5x5() {
        let mut wb = wb("a", 5, 5, 12);
        wb.cursor = 0;
        assert!(!wb.move_cursor(-1, 0));
        assert!(!wb.move_cursor(0, -1));
        assert_eq!(wb.cursor, 0);
        assert!(wb.move_cursor(1, 0));
        assert_eq!(wb.cursor, 1);
        wb.cursor = 24;
        assert!(!wb.move_cursor(1, 0));
        assert!(!wb.move_cursor(0, 1));
        assert_eq!(wb.cursor, 24);
        assert!(wb.move_cursor(-1, 0));
        assert_eq!(wb.cursor, 23);
    }

    #[test]
    fn move_cursor_bounds_4x4() {
        let mut wb = wb("bee", 4, 4, 13);
        wb.cursor = 15;
        assert!(!wb.move_cursor(0, 1));
        assert!(wb.move_cursor(0, -1));
        assert_eq!(wb.cursor, 11);
        wb.cursor = 0;
        assert!(!wb.move_cursor(-1, 0));
        assert!(wb.move_cursor(0, 1));
        assert_eq!(wb.cursor, 4);
    }

    #[test]
    fn invariant_after_space_clear_reset_sequence() {
        let mut wb = wb("letters", 5, 5, 20);
        assert!(wb.pick_place_swap());
        wb.move_cursor(1, 0);
        assert!(wb.pick_place_swap());
        wb.cursor = 0;
        assert!(wb.clear_cell());
        let mut rng = StdRng::seed_from_u64(21);
        wb.reset_grid(&mut rng);
        assert_invariant(&wb, 7);
    }

    #[test]
    fn click_cell_empty_hand_empty_cell_is_noop() {
        let mut wb = wb("cat", 5, 5, 30);
        assert!(!wb.click_cell(7));
        assert_eq!(wb.cells[7], None);
        assert_eq!(wb.held, None);
        assert_eq!(wb.cursor, 7);
        assert_invariant(&wb, 3);
    }

    #[test]
    fn click_cell_empty_hand_filled_cell_picks_up() {
        let mut wb = wb("cat", 5, 5, 31);
        assert!(wb.pick_place_swap());
        let placed = wb.cells[0].unwrap();
        wb.cursor = 4;
        assert!(wb.click_cell(0));
        assert_eq!(wb.cells[0], None);
        assert_eq!(wb.held, Some(placed));
        assert_eq!(wb.cursor, 0);
        assert_invariant(&wb, 3);
    }

    #[test]
    fn click_cell_hand_filled_cell_empty_places() {
        let mut wb = wb("cat", 5, 5, 32);
        assert!(wb.click_pool(0));
        let held = wb.held.unwrap();
        assert!(wb.click_cell(3));
        assert_eq!(wb.cells[3], Some(held));
        assert_eq!(wb.held, None);
        assert_invariant(&wb, 3);
    }

    #[test]
    fn click_cell_hand_filled_cell_filled_swaps() {
        let mut wb = wb("cat", 5, 5, 33);
        assert!(wb.click_pool(0));
        let first = wb.held.unwrap();
        assert!(wb.click_cell(0));
        assert!(wb.click_pool(0));
        let second = wb.held.unwrap();
        assert!(wb.click_cell(0));
        assert_eq!(wb.cells[0], Some(second));
        assert_eq!(wb.held, Some(first));
        assert_invariant(&wb, 3);
    }

    #[test]
    fn click_cell_out_of_range_is_false() {
        let mut wb = wb("cat", 4, 4, 34);
        assert!(!wb.click_cell(16));
        assert_eq!(wb.cursor, 0);
        assert_invariant(&wb, 3);
    }

    #[test]
    fn click_pool_pick_then_noop_while_holding() {
        let mut wb = wb("cat", 5, 5, 35);
        let first = wb.pool[1];
        assert!(wb.click_pool(1));
        assert_eq!(wb.held, Some(first));
        assert_eq!(wb.pool.len(), 2);
        assert!(!wb.click_pool(0));
        assert_eq!(wb.pool.len(), 2);
        assert_invariant(&wb, 3);
    }

    #[test]
    fn click_pool_out_of_range_is_false() {
        let mut wb = wb("cat", 5, 5, 36);
        assert!(!wb.click_pool(3));
        assert_eq!(wb.held, None);
        assert_invariant(&wb, 3);
    }

    #[test]
    fn invariant_after_click_sequence() {
        let mut wb = wb("letters", 5, 5, 37);
        for step in 0..40usize {
            let idx = (step * 7) % 25;
            if step % 3 == 0 {
                let _ = wb.click_pool(idx % wb.pool.len().max(1));
            } else {
                let _ = wb.click_cell(idx);
            }
            assert_invariant(&wb, 7);
        }
    }

    #[test]
    fn identity_guard_from_solution_crate_50_seeds() {
        for seed in 0..50u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let wb = LetterWorkbench::from_solution("crate", 5, 5, &mut rng);
            let identity: Vec<char> = "CRATE".chars().collect();
            assert_ne!(wb.pool, identity, "seed {seed}");
            assert_invariant(&wb, 5);
        }
    }

    #[test]
    fn identity_guard_reset_grid_crate_50_seeds() {
        for seed in 0..50u64 {
            let mut wb = wb("crate", 5, 5, 38);
            let mut rng = StdRng::seed_from_u64(seed);
            wb.reset_grid(&mut rng);
            let identity: Vec<char> = "CRATE".chars().collect();
            assert_ne!(wb.pool, identity, "seed {seed}");
            assert_invariant(&wb, 5);
        }
    }
}