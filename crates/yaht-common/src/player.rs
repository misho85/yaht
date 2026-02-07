use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::scoring::{Category, UPPER_BONUS_THRESHOLD, UPPER_BONUS_VALUE, YAHTZEE_BONUS_VALUE};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scorecard {
    pub scores: HashMap<Category, u16>,
    pub yahtzee_bonus_count: u8,
}

impl Scorecard {
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
            yahtzee_bonus_count: 0,
        }
    }

    pub fn is_category_used(&self, category: Category) -> bool {
        self.scores.contains_key(&category)
    }

    pub fn record(&mut self, category: Category, score: u16) -> Result<(), ScorecardError> {
        if self.is_category_used(category) {
            return Err(ScorecardError::CategoryAlreadyUsed);
        }
        self.scores.insert(category, score);
        Ok(())
    }

    pub fn add_yahtzee_bonus(&mut self) {
        self.yahtzee_bonus_count += 1;
    }

    pub fn upper_subtotal(&self) -> u16 {
        Category::UPPER
            .iter()
            .filter_map(|c| self.scores.get(c))
            .sum()
    }

    pub fn upper_bonus(&self) -> u16 {
        if self.upper_subtotal() >= UPPER_BONUS_THRESHOLD {
            UPPER_BONUS_VALUE
        } else {
            0
        }
    }

    pub fn lower_total(&self) -> u16 {
        Category::ALL
            .iter()
            .filter(|c| !c.is_upper())
            .filter_map(|c| self.scores.get(c))
            .sum()
    }

    pub fn yahtzee_bonus_total(&self) -> u16 {
        self.yahtzee_bonus_count as u16 * YAHTZEE_BONUS_VALUE
    }

    pub fn grand_total(&self) -> u16 {
        self.upper_subtotal() + self.upper_bonus() + self.lower_total() + self.yahtzee_bonus_total()
    }

    pub fn is_complete(&self) -> bool {
        self.scores.len() == 13
    }

    pub fn available_categories(&self) -> Vec<Category> {
        Category::ALL
            .iter()
            .filter(|c| !self.is_category_used(**c))
            .copied()
            .collect()
    }
}

impl Default for Scorecard {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ScorecardError {
    #[error("category already used")]
    CategoryAlreadyUsed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: Uuid,
    pub name: String,
    pub scorecard: Scorecard,
    pub connected: bool,
}

impl Player {
    pub fn new(id: Uuid, name: String) -> Self {
        Self {
            id,
            name,
            scorecard: Scorecard::new(),
            connected: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_scorecard() {
        let sc = Scorecard::new();
        assert_eq!(sc.grand_total(), 0);
        assert_eq!(sc.upper_subtotal(), 0);
        assert_eq!(sc.upper_bonus(), 0);
        assert!(!sc.is_complete());
        assert_eq!(sc.available_categories().len(), 13);
    }

    #[test]
    fn test_record_and_total() {
        let mut sc = Scorecard::new();
        sc.record(Category::Ones, 3).unwrap();
        sc.record(Category::Twos, 6).unwrap();
        assert_eq!(sc.upper_subtotal(), 9);
        assert_eq!(sc.grand_total(), 9);
    }

    #[test]
    fn test_duplicate_record() {
        let mut sc = Scorecard::new();
        sc.record(Category::Ones, 3).unwrap();
        assert!(sc.record(Category::Ones, 4).is_err());
    }

    #[test]
    fn test_upper_bonus_not_reached() {
        let mut sc = Scorecard::new();
        sc.record(Category::Ones, 3).unwrap();
        sc.record(Category::Twos, 6).unwrap();
        sc.record(Category::Threes, 9).unwrap();
        sc.record(Category::Fours, 12).unwrap();
        sc.record(Category::Fives, 15).unwrap();
        sc.record(Category::Sixes, 12).unwrap(); // total = 57, below 63
        assert_eq!(sc.upper_bonus(), 0);
    }

    #[test]
    fn test_upper_bonus_reached() {
        let mut sc = Scorecard::new();
        sc.record(Category::Ones, 3).unwrap();
        sc.record(Category::Twos, 6).unwrap();
        sc.record(Category::Threes, 9).unwrap();
        sc.record(Category::Fours, 12).unwrap();
        sc.record(Category::Fives, 15).unwrap();
        sc.record(Category::Sixes, 18).unwrap(); // total = 63, exactly threshold
        assert_eq!(sc.upper_bonus(), 35);
        assert_eq!(
            sc.grand_total(),
            63 + 35 // upper + bonus, no lower scores
        );
    }

    #[test]
    fn test_yahtzee_bonus() {
        let mut sc = Scorecard::new();
        sc.record(Category::Yahtzee, 50).unwrap();
        sc.add_yahtzee_bonus();
        sc.add_yahtzee_bonus();
        assert_eq!(sc.yahtzee_bonus_total(), 200);
    }

    #[test]
    fn test_complete_scorecard() {
        let mut sc = Scorecard::new();
        for cat in Category::ALL {
            sc.record(cat, 10).unwrap();
        }
        assert!(sc.is_complete());
        assert_eq!(sc.available_categories().len(), 0);
    }

    #[test]
    fn test_available_categories() {
        let mut sc = Scorecard::new();
        sc.record(Category::Ones, 3).unwrap();
        sc.record(Category::Yahtzee, 50).unwrap();
        let available = sc.available_categories();
        assert_eq!(available.len(), 11);
        assert!(!available.contains(&Category::Ones));
        assert!(!available.contains(&Category::Yahtzee));
    }
}
