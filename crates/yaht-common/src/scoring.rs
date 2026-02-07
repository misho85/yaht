use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    // Upper section
    Ones,
    Twos,
    Threes,
    Fours,
    Fives,
    Sixes,
    // Lower section
    ThreeOfAKind,
    FourOfAKind,
    FullHouse,
    SmallStraight,
    LargeStraight,
    Yahtzee,
    Chance,
}

impl Category {
    pub const ALL: [Category; 13] = [
        Category::Ones,
        Category::Twos,
        Category::Threes,
        Category::Fours,
        Category::Fives,
        Category::Sixes,
        Category::ThreeOfAKind,
        Category::FourOfAKind,
        Category::FullHouse,
        Category::SmallStraight,
        Category::LargeStraight,
        Category::Yahtzee,
        Category::Chance,
    ];

    pub const UPPER: [Category; 6] = [
        Category::Ones,
        Category::Twos,
        Category::Threes,
        Category::Fours,
        Category::Fives,
        Category::Sixes,
    ];

    pub fn is_upper(&self) -> bool {
        matches!(
            self,
            Category::Ones
                | Category::Twos
                | Category::Threes
                | Category::Fours
                | Category::Fives
                | Category::Sixes
        )
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Category::Ones => "Ones",
            Category::Twos => "Twos",
            Category::Threes => "Threes",
            Category::Fours => "Fours",
            Category::Fives => "Fives",
            Category::Sixes => "Sixes",
            Category::ThreeOfAKind => "3 of a Kind",
            Category::FourOfAKind => "4 of a Kind",
            Category::FullHouse => "Full House",
            Category::SmallStraight => "Sm. Straight",
            Category::LargeStraight => "Lg. Straight",
            Category::Yahtzee => "YAHTZEE",
            Category::Chance => "Chance",
        }
    }
}

pub const UPPER_BONUS_THRESHOLD: u16 = 63;
pub const UPPER_BONUS_VALUE: u16 = 35;
pub const YAHTZEE_BONUS_VALUE: u16 = 100;

/// Compute the score for a given category and dice values.
pub fn compute_score(category: Category, dice: &[u8; 5]) -> u16 {
    match category {
        Category::Ones => count_value(dice, 1),
        Category::Twos => count_value(dice, 2),
        Category::Threes => count_value(dice, 3),
        Category::Fours => count_value(dice, 4),
        Category::Fives => count_value(dice, 5),
        Category::Sixes => count_value(dice, 6),
        Category::ThreeOfAKind => {
            if has_n_of_a_kind(dice, 3) {
                sum(dice)
            } else {
                0
            }
        }
        Category::FourOfAKind => {
            if has_n_of_a_kind(dice, 4) {
                sum(dice)
            } else {
                0
            }
        }
        Category::FullHouse => {
            if is_full_house(dice) {
                25
            } else {
                0
            }
        }
        Category::SmallStraight => {
            if has_small_straight(dice) {
                30
            } else {
                0
            }
        }
        Category::LargeStraight => {
            if has_large_straight(dice) {
                40
            } else {
                0
            }
        }
        Category::Yahtzee => {
            if is_yahtzee(dice) {
                50
            } else {
                0
            }
        }
        Category::Chance => sum(dice),
    }
}

fn count_value(dice: &[u8; 5], val: u8) -> u16 {
    dice.iter().filter(|&&d| d == val).count() as u16 * val as u16
}

fn sum(dice: &[u8; 5]) -> u16 {
    dice.iter().map(|&d| d as u16).sum()
}

fn value_counts(dice: &[u8; 5]) -> [u8; 7] {
    let mut counts = [0u8; 7]; // index 0 unused, 1..=6
    for &d in dice {
        counts[d as usize] += 1;
    }
    counts
}

fn has_n_of_a_kind(dice: &[u8; 5], n: u8) -> bool {
    value_counts(dice).iter().any(|&c| c >= n)
}

fn is_full_house(dice: &[u8; 5]) -> bool {
    let counts = value_counts(dice);
    let has_three = counts.iter().any(|&c| c == 3);
    let has_two = counts.iter().any(|&c| c == 2);
    has_three && has_two
}

fn has_small_straight(dice: &[u8; 5]) -> bool {
    let counts = value_counts(dice);
    (1..=3).any(|start| (start..start + 4).all(|i| counts[i] >= 1))
}

fn has_large_straight(dice: &[u8; 5]) -> bool {
    let counts = value_counts(dice);
    (1..=2).any(|start| (start..start + 5).all(|i| counts[i] >= 1))
}

fn is_yahtzee(dice: &[u8; 5]) -> bool {
    has_n_of_a_kind(dice, 5)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Upper section tests
    #[test]
    fn test_ones() {
        assert_eq!(compute_score(Category::Ones, &[1, 1, 3, 4, 5]), 2);
        assert_eq!(compute_score(Category::Ones, &[2, 3, 4, 5, 6]), 0);
        assert_eq!(compute_score(Category::Ones, &[1, 1, 1, 1, 1]), 5);
    }

    #[test]
    fn test_twos() {
        assert_eq!(compute_score(Category::Twos, &[2, 2, 3, 4, 5]), 4);
        assert_eq!(compute_score(Category::Twos, &[1, 3, 4, 5, 6]), 0);
    }

    #[test]
    fn test_threes() {
        assert_eq!(compute_score(Category::Threes, &[3, 3, 3, 4, 5]), 9);
    }

    #[test]
    fn test_fours() {
        assert_eq!(compute_score(Category::Fours, &[4, 4, 4, 4, 5]), 16);
    }

    #[test]
    fn test_fives() {
        assert_eq!(compute_score(Category::Fives, &[5, 5, 5, 5, 5]), 25);
    }

    #[test]
    fn test_sixes() {
        assert_eq!(compute_score(Category::Sixes, &[6, 6, 1, 2, 3]), 12);
    }

    // Lower section tests
    #[test]
    fn test_three_of_a_kind() {
        assert_eq!(compute_score(Category::ThreeOfAKind, &[3, 3, 3, 4, 5]), 18);
        assert_eq!(compute_score(Category::ThreeOfAKind, &[1, 2, 3, 4, 5]), 0);
        // Four of a kind also counts as three of a kind
        assert_eq!(compute_score(Category::ThreeOfAKind, &[3, 3, 3, 3, 5]), 17);
    }

    #[test]
    fn test_four_of_a_kind() {
        assert_eq!(compute_score(Category::FourOfAKind, &[3, 3, 3, 3, 5]), 17);
        assert_eq!(compute_score(Category::FourOfAKind, &[3, 3, 3, 4, 5]), 0);
        // Five of a kind also counts
        assert_eq!(compute_score(Category::FourOfAKind, &[6, 6, 6, 6, 6]), 30);
    }

    #[test]
    fn test_full_house() {
        assert_eq!(compute_score(Category::FullHouse, &[3, 3, 3, 5, 5]), 25);
        assert_eq!(compute_score(Category::FullHouse, &[1, 1, 2, 2, 3]), 0);
        // Five of a kind is NOT a full house (no pair distinct from triple)
        assert_eq!(compute_score(Category::FullHouse, &[3, 3, 3, 3, 3]), 0);
    }

    #[test]
    fn test_small_straight() {
        assert_eq!(compute_score(Category::SmallStraight, &[1, 2, 3, 4, 6]), 30);
        assert_eq!(compute_score(Category::SmallStraight, &[2, 3, 4, 5, 1]), 30);
        assert_eq!(compute_score(Category::SmallStraight, &[3, 4, 5, 6, 1]), 30);
        assert_eq!(compute_score(Category::SmallStraight, &[1, 2, 3, 5, 6]), 0);
        // Large straight contains a small straight
        assert_eq!(compute_score(Category::SmallStraight, &[1, 2, 3, 4, 5]), 30);
        // Duplicate in small straight
        assert_eq!(compute_score(Category::SmallStraight, &[1, 2, 3, 4, 4]), 30);
    }

    #[test]
    fn test_large_straight() {
        assert_eq!(compute_score(Category::LargeStraight, &[1, 2, 3, 4, 5]), 40);
        assert_eq!(compute_score(Category::LargeStraight, &[2, 3, 4, 5, 6]), 40);
        assert_eq!(compute_score(Category::LargeStraight, &[1, 2, 3, 4, 6]), 0);
    }

    #[test]
    fn test_yahtzee() {
        assert_eq!(compute_score(Category::Yahtzee, &[5, 5, 5, 5, 5]), 50);
        assert_eq!(compute_score(Category::Yahtzee, &[1, 1, 1, 1, 1]), 50);
        assert_eq!(compute_score(Category::Yahtzee, &[5, 5, 5, 5, 4]), 0);
    }

    #[test]
    fn test_chance() {
        assert_eq!(compute_score(Category::Chance, &[1, 2, 3, 4, 5]), 15);
        assert_eq!(compute_score(Category::Chance, &[6, 6, 6, 6, 6]), 30);
    }

    #[test]
    fn test_category_is_upper() {
        assert!(Category::Ones.is_upper());
        assert!(Category::Sixes.is_upper());
        assert!(!Category::ThreeOfAKind.is_upper());
        assert!(!Category::Yahtzee.is_upper());
    }
}
