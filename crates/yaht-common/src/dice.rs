use rand::Rng;
use serde::{Deserialize, Serialize};

pub const NUM_DICE: usize = 5;
pub const MAX_ROLLS: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Die {
    pub value: u8,
    pub held: bool,
}

impl Die {
    pub fn new() -> Self {
        Self {
            value: 1,
            held: false,
        }
    }

    pub fn roll(&mut self, rng: &mut impl Rng) {
        if !self.held {
            self.value = rng.gen_range(1..=6);
        }
    }
}

impl Default for Die {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiceSet {
    pub dice: [Die; NUM_DICE],
}

impl DiceSet {
    pub fn new() -> Self {
        Self {
            dice: [Die::new(); NUM_DICE],
        }
    }

    pub fn roll_unheld(&mut self, rng: &mut impl Rng) {
        for die in &mut self.dice {
            die.roll(rng);
        }
    }

    pub fn set_held(&mut self, held: [bool; 5]) {
        for (die, &h) in self.dice.iter_mut().zip(held.iter()) {
            die.held = h;
        }
    }

    pub fn release_all(&mut self) {
        for die in &mut self.dice {
            die.held = false;
        }
    }

    pub fn values(&self) -> [u8; 5] {
        [
            self.dice[0].value,
            self.dice[1].value,
            self.dice[2].value,
            self.dice[3].value,
            self.dice[4].value,
        ]
    }

    pub fn sorted_values(&self) -> [u8; 5] {
        let mut v = self.values();
        v.sort();
        v
    }
}

impl Default for DiceSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_die_default_value() {
        let die = Die::new();
        assert_eq!(die.value, 1);
        assert!(!die.held);
    }

    #[test]
    fn test_die_roll_changes_value() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut die = Die::new();
        die.roll(&mut rng);
        assert!((1..=6).contains(&die.value));
    }

    #[test]
    fn test_die_held_does_not_roll() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut die = Die { value: 3, held: true };
        die.roll(&mut rng);
        assert_eq!(die.value, 3);
    }

    #[test]
    fn test_dice_set_roll_all() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut dice = DiceSet::new();
        dice.roll_unheld(&mut rng);
        for die in &dice.dice {
            assert!((1..=6).contains(&die.value));
        }
    }

    #[test]
    fn test_dice_set_hold_and_roll() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut dice = DiceSet::new();
        dice.roll_unheld(&mut rng);
        let first_values = dice.values();

        dice.set_held([true, true, false, false, false]);
        dice.roll_unheld(&mut rng);

        assert_eq!(dice.dice[0].value, first_values[0]);
        assert_eq!(dice.dice[1].value, first_values[1]);
    }

    #[test]
    fn test_dice_set_release_all() {
        let mut dice = DiceSet::new();
        dice.set_held([true, true, true, true, true]);
        dice.release_all();
        for die in &dice.dice {
            assert!(!die.held);
        }
    }

    #[test]
    fn test_sorted_values() {
        let mut dice = DiceSet::new();
        dice.dice[0].value = 5;
        dice.dice[1].value = 3;
        dice.dice[2].value = 1;
        dice.dice[3].value = 4;
        dice.dice[4].value = 2;
        assert_eq!(dice.sorted_values(), [1, 2, 3, 4, 5]);
    }
}
