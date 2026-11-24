//! Round state: shuffle pool, cursor-based fill, win check.
//!
//! Public API: `Round`, `try_type_letter`, `backspace`, `move_cursor`, `reset_same_word`, `check_solution`.

use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Round {
    pub solution: String,
    pub pool: Vec<char>,
    pub answer: Vec<Option<char>>,
    /// Active answer slot (arrow keys).
    pub cursor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveResult {
    Incomplete,
    Correct,
    Wrong,
}

impl Round {
    pub fn new(word: &str, rng: &mut impl Rng) -> Self {
        let solution: String = word.to_uppercase();
        let len = solution.len();
        let mut pool: Vec<char> = solution.chars().collect();
        shuffle_pool_not_identity(&solution, &mut pool, rng);
        Self {
            solution,
            pool,
            answer: vec![None; len],
            cursor: 0,
        }
    }

    pub fn move_cursor(&mut self, delta: i32) -> bool {
        let new = self.cursor as i32 + delta;
        if new < 0 || new >= self.answer.len() as i32 {
            return false;
        }
        self.cursor = new as usize;
        true
    }

    pub fn try_type_letter(&mut self, ch: char) -> bool {
        if self.answer.is_empty() || self.answer[self.cursor].is_some() {
            return false;
        }
        let ch = ch.to_ascii_uppercase();
        let pool_idx = self
            .pool
            .iter()
            .position(|&p| p.eq_ignore_ascii_case(&ch));
        let Some(pool_idx) = pool_idx else {
            return false;
        };
        let letter = self.pool.remove(pool_idx);
        self.answer[self.cursor] = Some(letter);
        self.advance_cursor_after_type();
        self.assert_invariant();
        true
    }

    pub fn backspace(&mut self) -> bool {
        if self.answer[self.cursor].is_some() {
            let letter = self.answer[self.cursor].take().expect("filled slot");
            self.pool.push(letter);
            self.assert_invariant();
            return true;
        }
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.backspace()
    }

    pub fn reset_same_word(&mut self, rng: &mut impl Rng) {
        self.cursor = 0;
        for slot in &mut self.answer {
            *slot = None;
        }
        self.pool = self.solution.chars().collect();
        shuffle_pool_not_identity(&self.solution, &mut self.pool, rng);
        self.assert_invariant();
    }

    pub fn is_answer_full(&self) -> bool {
        self.answer.iter().all(|s| s.is_some())
    }

    /// True when slot `i` is filled with the solution letter at that index.
    pub fn slot_is_correct(&self, index: usize) -> bool {
        let Some(expected) = self.solution.chars().nth(index) else {
            return false;
        };
        match self.answer.get(index).and_then(|s| *s) {
            Some(ch) => ch.eq_ignore_ascii_case(&expected),
            None => false,
        }
    }

    pub fn check_solution(&self) -> SolveResult {
        if !self.is_answer_full() {
            return SolveResult::Incomplete;
        }
        let guess: String = self
            .answer
            .iter()
            .map(|s| s.expect("full answer"))
            .collect();
        if guess.eq_ignore_ascii_case(&self.solution) {
            SolveResult::Correct
        } else {
            SolveResult::Wrong
        }
    }

    pub fn assert_invariant(&self) {
        debug_assert!(multiset_matches(&self.solution, &self.pool, &self.answer));
    }

    /// After typing, move to the next empty slot to the **right**, not the leftmost empty.
    fn advance_cursor_after_type(&mut self) {
        for i in (self.cursor + 1)..self.answer.len() {
            if self.answer[i].is_none() {
                self.cursor = i;
                return;
            }
        }
    }

    /// Jump to the leftmost empty slot (e.g. after first-letter clue).
    pub fn snap_cursor_to_next_empty(&mut self) {
        if let Some(i) = self.answer.iter().position(|s| s.is_none()) {
            self.cursor = i;
        }
    }
}

fn shuffle_pool_not_identity(solution: &str, pool: &mut [char], rng: &mut impl Rng) {
    loop {
        pool.shuffle(rng);
        if distinct_letter_count(solution) <= 1 {
            break;
        }
        let identity: Vec<char> = solution.chars().collect();
        if pool != identity.as_slice() {
            break;
        }
    }
}

fn distinct_letter_count(word: &str) -> usize {
    let mut seen = std::collections::HashSet::new();
    for c in word.chars() {
        seen.insert(c.to_ascii_uppercase());
    }
    seen.len()
}

pub fn multiset_matches(solution: &str, pool: &[char], answer: &[Option<char>]) -> bool {
    let mut counts = std::collections::HashMap::new();
    for c in solution.chars() {
        *counts.entry(c.to_ascii_uppercase()).or_insert(0usize) += 1;
    }
    for c in pool {
        let key = c.to_ascii_uppercase();
        let Some(e) = counts.get_mut(&key) else {
            return false;
        };
        if *e == 0 {
            return false;
        }
        *e -= 1;
    }
    for slot in answer {
        if let Some(c) = slot {
            let key = c.to_ascii_uppercase();
            let Some(e) = counts.get_mut(&key) else {
                return false;
            };
            if *e == 0 {
                return false;
            }
            *e -= 1;
        }
    }
    counts.values().all(|&n| n == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn round_with_seed(word: &str, seed: u64) -> Round {
        let mut rng = StdRng::seed_from_u64(seed);
        Round::new(word, &mut rng)
    }

    fn assert_round_invariant(round: &Round) {
        assert!(multiset_matches(
            &round.solution,
            &round.pool,
            &round.answer
        ));
    }

    fn fill_solution_left_to_right(round: &mut Round) {
        for i in 0..round.answer.len() {
            let ch = round.solution.chars().nth(i).unwrap();
            round.cursor = i;
            assert!(round.try_type_letter(ch));
        }
    }

    #[test]
    fn new_empty_word() {
        let mut rng = StdRng::seed_from_u64(1);
        let round = Round::new("", &mut rng);
        assert!(round.pool.is_empty());
        assert!(round.answer.is_empty());
        assert_eq!(round.cursor, 0);
    }

    #[test]
    fn shuffle_preserves_multiset_and_length() {
        let round = round_with_seed("crate", 42);
        assert_eq!(round.pool.len(), 5);
        assert_eq!(round.solution, "CRATE");
        let mut sorted_pool = round.pool.clone();
        sorted_pool.sort_unstable();
        let mut sorted_solution: Vec<char> = round.solution.chars().collect();
        sorted_solution.sort_unstable();
        assert_eq!(sorted_pool, sorted_solution);
        assert_round_invariant(&round);
    }

    #[test]
    fn shuffle_not_identity_when_multiple_distinct_letters() {
        for seed in 0..50_u64 {
            let round = round_with_seed("crate", seed);
            let identity: Vec<char> = round.solution.chars().collect();
            assert_ne!(
                round.pool, identity,
                "pool should not match solution order (seed {seed})"
            );
        }
    }

    #[test]
    fn advance_cursor_prefers_right_not_leftmost_empty() {
        let mut round = round_with_seed("abcde", 7);
        round.cursor = 3;
        assert!(round.try_type_letter('d'));
        assert_eq!(round.answer[3], Some('D'));
        assert_eq!(round.cursor, 4);
    }

    #[test]
    fn type_at_cursor_and_move() {
        let mut round = round_with_seed("ab", 7);
        assert!(round.try_type_letter('b'));
        assert_eq!(round.answer[0], Some('B'));
        assert_eq!(round.cursor, 1);
        assert!(round.try_type_letter('a'));
        assert_eq!(round.cursor, 1);
        assert!(round.is_answer_full());
    }

    #[test]
    fn cannot_type_on_filled_slot() {
        let mut round = round_with_seed("ab", 1);
        assert!(round.try_type_letter('a'));
        round.cursor = 0;
        assert!(!round.try_type_letter('b'));
    }

    #[test]
    fn try_type_letter_case_insensitive() {
        let mut round = round_with_seed("cat", 3);
        fill_solution_left_to_right(&mut round);
        assert!(round.is_answer_full());
        assert_eq!(round.pool.len(), 0);
    }

    #[test]
    fn invalid_letter_is_noop() {
        let mut round = round_with_seed("cat", 3);
        let pool = round.pool.clone();
        assert!(!round.try_type_letter('z'));
        assert_eq!(round.pool, pool);
    }

    #[test]
    fn duplicate_letters_remove_one_instance() {
        let mut round = round_with_seed("letter", 9);
        let e_count_before = round.pool.iter().filter(|&&c| c == 'E').count();
        assert!(e_count_before >= 2);
        assert!(round.try_type_letter('e'));
        let e_count_after = round.pool.iter().filter(|&&c| c == 'E').count();
        assert_eq!(e_count_after, e_count_before - 1);
        assert_round_invariant(&round);
    }

    #[test]
    fn backspace_clears_cursor_or_moves_left() {
        let mut round = round_with_seed("ab", 11);
        assert!(round.try_type_letter('a'));
        assert!(round.backspace());
        assert_eq!(round.answer[0], None);
        assert!(round.try_type_letter('a'));
        round.cursor = 1;
        assert!(round.try_type_letter('b'));
        round.cursor = 1;
        assert!(round.backspace());
        assert_eq!(round.answer[1], None);
        assert_round_invariant(&round);
    }

    #[test]
    fn check_solution_states() {
        let mut round = round_with_seed("dog", 5);
        assert_eq!(round.check_solution(), SolveResult::Incomplete);

        fill_solution_left_to_right(&mut round);
        assert_eq!(round.check_solution(), SolveResult::Correct);
    }

    #[test]
    fn check_solution_wrong_when_full() {
        let mut round = round_with_seed("cat", 5);
        for (i, ch) in ['c', 't', 'a'].into_iter().enumerate() {
            round.cursor = i;
            round.try_type_letter(ch);
        }
        assert_eq!(round.check_solution(), SolveResult::Wrong);
    }

    #[test]
    fn slot_is_correct_only_when_letter_matches() {
        let mut round = round_with_seed("cat", 3);
        round.cursor = 0;
        round.try_type_letter('c');
        assert!(round.slot_is_correct(0));
        round.cursor = 1;
        round.try_type_letter('t'); // wrong for slot 1 (needs A)
        assert!(!round.slot_is_correct(1));
        round.backspace();
        round.try_type_letter('a');
        assert!(round.slot_is_correct(1));
    }

    #[test]
    fn reset_same_word_clears_answer_and_reshuffles() {
        let mut rng = StdRng::seed_from_u64(100);
        let mut round = Round::new("crate", &mut rng);
        round.try_type_letter('c');
        round.reset_same_word(&mut rng);
        assert_eq!(round.cursor, 0);
        assert!(round.answer.iter().all(|s| s.is_none()));
        assert_round_invariant(&round);
    }
}
