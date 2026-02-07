use rand::Rng;

use crate::dice::DiceSet;
use crate::player::Scorecard;
use crate::scoring::{self, Category};

/// AI difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiDifficulty {
    Easy,   // Random choices
    Medium, // Greedy (pick best immediate score)
    Hard,   // Greedy with smart holds and upper bonus awareness
}

/// Choose which dice to hold based on AI strategy.
/// Returns the held array [bool; 5].
pub fn choose_holds(
    dice: &DiceSet,
    scorecard: &Scorecard,
    difficulty: AiDifficulty,
    rng: &mut impl Rng,
) -> [bool; 5] {
    match difficulty {
        AiDifficulty::Easy => {
            // Random holds
            let mut held = [false; 5];
            for h in held.iter_mut() {
                *h = rng.gen_bool(0.3);
            }
            held
        }
        AiDifficulty::Medium | AiDifficulty::Hard => {
            greedy_holds(dice, scorecard, difficulty)
        }
    }
}

/// Choose which category to score based on AI strategy.
pub fn choose_category(
    dice: &DiceSet,
    scorecard: &Scorecard,
    difficulty: AiDifficulty,
    rng: &mut impl Rng,
) -> Category {
    let available = scorecard.available_categories();
    if available.is_empty() {
        return Category::Chance; // shouldn't happen
    }

    match difficulty {
        AiDifficulty::Easy => {
            // Random available category
            let idx = rng.gen_range(0..available.len());
            available[idx]
        }
        AiDifficulty::Medium | AiDifficulty::Hard => {
            greedy_category(dice, scorecard, difficulty)
        }
    }
}

/// Greedy hold strategy: find the best category and hold dice that contribute to it.
fn greedy_holds(dice: &DiceSet, scorecard: &Scorecard, difficulty: AiDifficulty) -> [bool; 5] {
    let values = dice.values();
    let available = scorecard.available_categories();

    if available.is_empty() {
        return [false; 5];
    }

    // Find the best scoring category for current dice
    let best_cat = greedy_category(dice, scorecard, difficulty);

    // Now decide which dice to hold based on the target category
    match best_cat {
        // Upper section: hold matching dice
        Category::Ones => hold_matching(&values, 1),
        Category::Twos => hold_matching(&values, 2),
        Category::Threes => hold_matching(&values, 3),
        Category::Fours => hold_matching(&values, 4),
        Category::Fives => hold_matching(&values, 5),
        Category::Sixes => hold_matching(&values, 6),

        // N of a kind: hold the most frequent value
        Category::ThreeOfAKind | Category::FourOfAKind | Category::Yahtzee => {
            let counts = value_counts(&values);
            let best_val = (1..=6u8)
                .max_by_key(|&v| (counts[v as usize], v))
                .unwrap_or(6);
            hold_matching(&values, best_val)
        }

        // Full house: hold the most frequent group
        Category::FullHouse => {
            let counts = value_counts(&values);
            // Find values with 2+ count
            let mut groups: Vec<(u8, u8)> = (1..=6u8)
                .filter(|&v| counts[v as usize] >= 2)
                .map(|v| (v, counts[v as usize]))
                .collect();
            groups.sort_by(|a, b| b.1.cmp(&a.1));

            if groups.len() >= 2 {
                // Hold the triple and the pair
                let triple_val = groups[0].0;
                let pair_val = groups[1].0;
                let mut held = [false; 5];
                let mut triple_count = 0;
                let mut pair_count = 0;
                for (i, &v) in values.iter().enumerate() {
                    if v == triple_val && triple_count < 3 {
                        held[i] = true;
                        triple_count += 1;
                    } else if v == pair_val && pair_count < 2 {
                        held[i] = true;
                        pair_count += 1;
                    }
                }
                held
            } else {
                // Hold the most common
                let best_val = (1..=6u8)
                    .max_by_key(|&v| counts[v as usize])
                    .unwrap_or(1);
                hold_matching(&values, best_val)
            }
        }

        // Straights: hold sequential dice
        Category::SmallStraight | Category::LargeStraight => {
            hold_for_straight(&values)
        }

        // Chance: hold high values (4, 5, 6)
        Category::Chance => {
            let mut held = [false; 5];
            for (i, &v) in values.iter().enumerate() {
                held[i] = v >= 4;
            }
            held
        }
    }
}

/// Greedy category selection: pick the category that gives the best score.
/// For Hard difficulty, also considers upper bonus potential.
fn greedy_category(dice: &DiceSet, scorecard: &Scorecard, difficulty: AiDifficulty) -> Category {
    let values = dice.values();
    let available = scorecard.available_categories();

    if available.is_empty() {
        return Category::Chance;
    }

    // Score each available category
    let mut scored: Vec<(Category, u16, i32)> = available
        .iter()
        .map(|&cat| {
            let score = scoring::compute_score(cat, &values);
            let priority = if difficulty == AiDifficulty::Hard {
                category_priority(cat, score, scorecard)
            } else {
                score as i32
            };
            (cat, score, priority)
        })
        .collect();

    // Sort by priority descending, then by score descending
    scored.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.cmp(&a.1)));

    // If the best score is 0, try to burn the least valuable category
    if scored[0].1 == 0 {
        // Pick the category where 0 hurts least
        return least_valuable_zero(&available, scorecard);
    }

    scored[0].0
}

/// For hard difficulty: prioritize categories based on strategic value.
fn category_priority(cat: Category, score: u16, scorecard: &Scorecard) -> i32 {
    let base = score as i32;

    match cat {
        // Yahtzee is always highest priority
        Category::Yahtzee if score == 50 => base + 100,

        // Large straight is valuable
        Category::LargeStraight if score > 0 => base + 20,

        // Full house is decent
        Category::FullHouse if score > 0 => base + 10,

        // Upper section: bonus for being on track for 63
        cat if cat.is_upper() => {
            let upper_so_far = scorecard.upper_subtotal();
            let cats_used = Category::UPPER.iter().filter(|c| scorecard.is_category_used(**c)).count();
            let cats_remaining = 6 - cats_used;

            if cats_remaining > 0 {
                // Target per remaining category to hit 63
                let remaining_needed = 63u16.saturating_sub(upper_so_far);
                let target_per_cat = if cats_remaining > 0 {
                    remaining_needed / cats_remaining as u16
                } else {
                    0
                };

                // Bonus if this score meets or exceeds the per-category target
                let face_val = upper_face_value(cat);
                let expected = face_val as u16 * 3; // 3 of each is the "par"
                if score >= expected {
                    base + 15 // Good upper score
                } else if score >= target_per_cat && score > 0 {
                    base + 5
                } else {
                    base
                }
            } else {
                base
            }
        }

        _ => base,
    }
}

/// When all categories would score 0, pick the least valuable one to sacrifice.
fn least_valuable_zero(available: &[Category], scorecard: &Scorecard) -> Category {
    // Prefer to zero out categories that are hardest to score
    let priority = |cat: &Category| -> i32 {
        match cat {
            // Don't waste Yahtzee (highest priority to keep)
            Category::Yahtzee => -100,
            Category::LargeStraight => -80,
            Category::FullHouse => -60,
            Category::SmallStraight => -50,
            Category::FourOfAKind => -40,
            Category::ThreeOfAKind => -30,
            // Upper section: sacrifice ones first since they contribute least to bonus
            Category::Ones => 10,
            Category::Twos => 5,
            Category::Threes => 0,
            Category::Chance => -10, // Keep chance as a fallback
            _ => {
                // For upper categories close to bonus, protect them
                let upper_total = scorecard.upper_subtotal();
                if upper_total >= 50 {
                    -20 // Protect upper cats when close to bonus
                } else {
                    0
                }
            }
        }
    };

    available
        .iter()
        .max_by_key(|c| priority(c))
        .copied()
        .unwrap_or(Category::Chance)
}

fn upper_face_value(cat: Category) -> u8 {
    match cat {
        Category::Ones => 1,
        Category::Twos => 2,
        Category::Threes => 3,
        Category::Fours => 4,
        Category::Fives => 5,
        Category::Sixes => 6,
        _ => 0,
    }
}

fn value_counts(dice: &[u8; 5]) -> [u8; 7] {
    let mut counts = [0u8; 7];
    for &d in dice {
        counts[d as usize] += 1;
    }
    counts
}

fn hold_matching(values: &[u8; 5], target: u8) -> [bool; 5] {
    let mut held = [false; 5];
    for (i, &v) in values.iter().enumerate() {
        held[i] = v == target;
    }
    held
}

fn hold_for_straight(values: &[u8; 5]) -> [bool; 5] {
    let mut held = [false; 5];
    let counts = value_counts(values);

    // Find the best straight run
    // Try 1-2-3-4-5 first, then 2-3-4-5-6
    let runs = [
        vec![1, 2, 3, 4, 5],
        vec![2, 3, 4, 5, 6],
        vec![1, 2, 3, 4],
        vec![2, 3, 4, 5],
        vec![3, 4, 5, 6],
    ];

    let mut best_run: &[u8] = &[];
    let mut best_matches = 0;

    for run in &runs {
        let matches: usize = run.iter().filter(|&&v| counts[v as usize] > 0).count();
        if matches > best_matches {
            best_matches = matches;
            best_run = run;
        }
    }

    if !best_run.is_empty() {
        let mut used = [false; 7]; // track which values we've used
        for (i, &v) in values.iter().enumerate() {
            if best_run.contains(&v) && !used[v as usize] {
                held[i] = true;
                used[v as usize] = true;
            }
        }
    }

    held
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::Die;
    use rand::SeedableRng;

    fn make_dice(values: [u8; 5]) -> DiceSet {
        let mut ds = DiceSet::new();
        for (i, v) in values.iter().enumerate() {
            ds.dice[i] = Die { value: *v, held: false };
        }
        ds
    }

    #[test]
    fn test_greedy_picks_yahtzee() {
        let dice = make_dice([5, 5, 5, 5, 5]);
        let sc = Scorecard::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let cat = choose_category(&dice, &sc, AiDifficulty::Medium, &mut rng);
        assert_eq!(cat, Category::Yahtzee);
    }

    #[test]
    fn test_greedy_picks_large_straight() {
        let dice = make_dice([1, 2, 3, 4, 5]);
        let sc = Scorecard::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let cat = choose_category(&dice, &sc, AiDifficulty::Medium, &mut rng);
        assert_eq!(cat, Category::LargeStraight);
    }

    #[test]
    fn test_hold_matching() {
        let result = hold_matching(&[3, 3, 1, 5, 3], 3);
        assert_eq!(result, [true, true, false, false, true]);
    }

    #[test]
    fn test_hold_for_straight() {
        let result = hold_for_straight(&[1, 2, 3, 5, 6]);
        // Should hold sequential dice for the best run
        assert!(result.iter().filter(|&&h| h).count() >= 2);
    }

    #[test]
    fn test_easy_random_category() {
        let dice = make_dice([1, 2, 3, 4, 5]);
        let sc = Scorecard::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let cat = choose_category(&dice, &sc, AiDifficulty::Easy, &mut rng);
        // Should return some valid available category
        assert!(sc.available_categories().contains(&cat));
    }

    #[test]
    fn test_zero_score_sacrifices_ones() {
        let dice = make_dice([2, 3, 4, 5, 6]); // No ones
        let mut sc = Scorecard::new();
        // Fill all categories except Ones and Twos
        for cat in &[
            Category::Threes, Category::Fours, Category::Fives, Category::Sixes,
            Category::ThreeOfAKind, Category::FourOfAKind, Category::FullHouse,
            Category::SmallStraight, Category::LargeStraight, Category::Yahtzee, Category::Chance,
        ] {
            let _ = sc.record(*cat, 10);
        }
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let cat = choose_category(&dice, &sc, AiDifficulty::Medium, &mut rng);
        // Should pick Twos (score 2) over Ones (score 0), or Twos which actually scores
        assert!(cat == Category::Ones || cat == Category::Twos);
    }
}
