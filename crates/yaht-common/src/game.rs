use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dice::{DiceSet, MAX_ROLLS};
use crate::player::{Player, Scorecard};
use crate::scoring::{self, Category};

// -- Turn State Machine --

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TurnPhase {
    WaitingForRoll,
    Rolling { rolls_used: u8 },
    MustScore,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnState {
    pub player_id: Uuid,
    pub phase: TurnPhase,
    pub dice: DiceSet,
    pub rolls_used: u8,
}

impl TurnState {
    pub fn new(player_id: Uuid) -> Self {
        Self {
            player_id,
            phase: TurnPhase::WaitingForRoll,
            dice: DiceSet::new(),
            rolls_used: 0,
        }
    }

    pub fn can_roll(&self) -> bool {
        self.rolls_used < MAX_ROLLS
            && matches!(
                self.phase,
                TurnPhase::WaitingForRoll | TurnPhase::Rolling { .. }
            )
    }

    pub fn can_hold(&self) -> bool {
        matches!(self.phase, TurnPhase::Rolling { .. })
    }

    pub fn can_score(&self) -> bool {
        matches!(self.phase, TurnPhase::Rolling { .. } | TurnPhase::MustScore)
    }

    pub fn roll(&mut self, rng: &mut impl Rng) -> Result<(), GameError> {
        if !self.can_roll() {
            return Err(GameError::CannotRoll);
        }
        if self.rolls_used == 0 {
            self.dice.release_all();
        }
        self.dice.roll_unheld(rng);
        self.rolls_used += 1;
        self.phase = if self.rolls_used >= MAX_ROLLS {
            TurnPhase::MustScore
        } else {
            TurnPhase::Rolling {
                rolls_used: self.rolls_used,
            }
        };
        Ok(())
    }

    pub fn hold(&mut self, held: [bool; 5]) -> Result<(), GameError> {
        if !self.can_hold() {
            return Err(GameError::CannotHold);
        }
        self.dice.set_held(held);
        Ok(())
    }
}

// -- Game State Machine --

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GamePhase {
    Lobby,
    Playing,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub phase: GamePhase,
    pub players: Vec<Player>,
    pub current_player_index: usize,
    pub turn: Option<TurnState>,
    pub round: u8,
    pub total_rounds: u8,
}

impl GameState {
    pub fn new(players: Vec<Player>) -> Self {
        Self {
            phase: GamePhase::Lobby,
            players,
            current_player_index: 0,
            turn: None,
            round: 0,
            total_rounds: 13,
        }
    }

    pub fn start(&mut self) -> Result<(), GameError> {
        if self.players.len() < 2 {
            return Err(GameError::NotEnoughPlayers);
        }
        if self.players.len() > 6 {
            return Err(GameError::TooManyPlayers);
        }
        self.phase = GamePhase::Playing;
        self.round = 1;
        self.current_player_index = 0;
        self.turn = Some(TurnState::new(self.current_player().id));
        Ok(())
    }

    /// Start a game with 1+ players (allows solo play against AI).
    pub fn start_solo(&mut self) -> Result<(), GameError> {
        if self.players.is_empty() {
            return Err(GameError::NotEnoughPlayers);
        }
        if self.players.len() > 6 {
            return Err(GameError::TooManyPlayers);
        }
        self.phase = GamePhase::Playing;
        self.round = 1;
        self.current_player_index = 0;
        self.turn = Some(TurnState::new(self.current_player().id));
        Ok(())
    }

    pub fn current_player(&self) -> &Player {
        &self.players[self.current_player_index]
    }

    pub fn current_player_mut(&mut self) -> &mut Player {
        &mut self.players[self.current_player_index]
    }

    pub fn is_current_player(&self, player_id: Uuid) -> bool {
        self.current_player().id == player_id
    }

    pub fn roll_dice(&mut self, player_id: Uuid, rng: &mut impl Rng) -> Result<(), GameError> {
        if self.phase != GamePhase::Playing {
            return Err(GameError::GameNotInProgress);
        }
        if !self.is_current_player(player_id) {
            return Err(GameError::NotYourTurn);
        }
        let turn = self.turn.as_mut().ok_or(GameError::NoActiveTurn)?;
        turn.roll(rng)
    }

    pub fn hold_dice(
        &mut self,
        player_id: Uuid,
        held: [bool; 5],
    ) -> Result<(), GameError> {
        if self.phase != GamePhase::Playing {
            return Err(GameError::GameNotInProgress);
        }
        if !self.is_current_player(player_id) {
            return Err(GameError::NotYourTurn);
        }
        let turn = self.turn.as_mut().ok_or(GameError::NoActiveTurn)?;
        turn.hold(held)
    }

    pub fn score_category(
        &mut self,
        player_id: Uuid,
        category: Category,
    ) -> Result<u16, GameError> {
        if self.phase != GamePhase::Playing {
            return Err(GameError::GameNotInProgress);
        }
        if !self.is_current_player(player_id) {
            return Err(GameError::NotYourTurn);
        }
        let turn = self.turn.as_ref().ok_or(GameError::NoActiveTurn)?;
        if !turn.can_score() {
            return Err(GameError::CannotScore);
        }

        let dice_values = turn.dice.values();
        let is_yahtzee = scoring::compute_score(Category::Yahtzee, &dice_values) == 50;

        // Yahtzee bonus: if dice are a Yahtzee AND the player already scored
        // Yahtzee with 50, they get a 100-point bonus.
        let joker_active = is_yahtzee
            && self.current_player().scorecard.scores.get(&Category::Yahtzee) == Some(&50);

        if joker_active {
            self.current_player_mut().scorecard.add_yahtzee_bonus();
        }

        // Use Joker scoring when applicable (Full House/Straights score full value with Yahtzee)
        let score = scoring::compute_score_joker(category, &dice_values, joker_active);
        self.current_player_mut()
            .scorecard
            .record(category, score)
            .map_err(|_| GameError::CategoryAlreadyScored)?;

        self.advance_turn();
        Ok(score)
    }

    fn advance_turn(&mut self) {
        self.current_player_index += 1;
        if self.current_player_index >= self.players.len() {
            self.current_player_index = 0;
            self.round += 1;
        }
        if self.round > self.total_rounds {
            self.phase = GamePhase::Finished;
            self.turn = None;
        } else {
            self.turn = Some(TurnState::new(self.current_player().id));
        }
    }

    pub fn winner(&self) -> Option<&Player> {
        if self.phase != GamePhase::Finished {
            return None;
        }
        self.players
            .iter()
            .max_by_key(|p| p.scorecard.grand_total())
    }

    pub fn snapshot(&self) -> GameStateSnapshot {
        GameStateSnapshot {
            phase: self.phase.clone(),
            players: self
                .players
                .iter()
                .map(|p| PlayerSnapshot {
                    id: p.id,
                    name: p.name.clone(),
                    scorecard: p.scorecard.clone(),
                    connected: p.connected,
                })
                .collect(),
            current_player_index: self.current_player_index,
            dice: self.turn.as_ref().map(|t| t.dice),
            turn_phase: self.turn.as_ref().map(|t| t.phase.clone()),
            rolls_used: self.turn.as_ref().map(|t| t.rolls_used).unwrap_or(0),
            round: self.round,
            total_rounds: self.total_rounds,
        }
    }
}

// -- Snapshot (sent over the network) --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateSnapshot {
    pub phase: GamePhase,
    pub players: Vec<PlayerSnapshot>,
    pub current_player_index: usize,
    pub dice: Option<DiceSet>,
    pub turn_phase: Option<TurnPhase>,
    pub rolls_used: u8,
    pub round: u8,
    pub total_rounds: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub id: Uuid,
    pub name: String,
    pub scorecard: Scorecard,
    pub connected: bool,
}

// -- Errors --

#[derive(Debug, Clone, thiserror::Error)]
pub enum GameError {
    #[error("cannot roll now")]
    CannotRoll,
    #[error("cannot hold dice now")]
    CannotHold,
    #[error("cannot score now")]
    CannotScore,
    #[error("no active turn")]
    NoActiveTurn,
    #[error("category already scored")]
    CategoryAlreadyScored,
    #[error("not enough players (need 2-6)")]
    NotEnoughPlayers,
    #[error("too many players (max 6)")]
    TooManyPlayers,
    #[error("not your turn")]
    NotYourTurn,
    #[error("game not in progress")]
    GameNotInProgress,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn make_players(n: usize) -> Vec<Player> {
        (0..n)
            .map(|i| Player::new(Uuid::new_v4(), format!("Player{}", i + 1)))
            .collect()
    }

    #[test]
    fn test_game_start_requires_min_players() {
        let mut game = GameState::new(vec![Player::new(Uuid::new_v4(), "Solo".into())]);
        assert!(matches!(game.start(), Err(GameError::NotEnoughPlayers)));
    }

    #[test]
    fn test_game_start_max_players() {
        let mut game = GameState::new(make_players(7));
        assert!(matches!(game.start(), Err(GameError::TooManyPlayers)));
    }

    #[test]
    fn test_game_start_success() {
        let mut game = GameState::new(make_players(2));
        assert!(game.start().is_ok());
        assert_eq!(game.phase, GamePhase::Playing);
        assert_eq!(game.round, 1);
        assert!(game.turn.is_some());
    }

    #[test]
    fn test_turn_roll_and_score() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let players = make_players(2);
        let p1_id = players[0].id;
        let mut game = GameState::new(players);
        game.start().unwrap();

        // Player 1 rolls
        game.roll_dice(p1_id, &mut rng).unwrap();
        assert!(game.turn.as_ref().unwrap().can_score());

        // Player 1 scores Chance
        let score = game.score_category(p1_id, Category::Chance).unwrap();
        assert!(score > 0);

        // Now it should be player 2's turn
        assert_eq!(game.current_player_index, 1);
    }

    #[test]
    fn test_wrong_player_cannot_act() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let players = make_players(2);
        let p2_id = players[1].id;
        let mut game = GameState::new(players);
        game.start().unwrap();

        assert!(matches!(
            game.roll_dice(p2_id, &mut rng),
            Err(GameError::NotYourTurn)
        ));
    }

    #[test]
    fn test_cannot_score_before_rolling() {
        let players = make_players(2);
        let p1_id = players[0].id;
        let mut game = GameState::new(players);
        game.start().unwrap();

        assert!(matches!(
            game.score_category(p1_id, Category::Chance),
            Err(GameError::CannotScore)
        ));
    }

    #[test]
    fn test_three_rolls_then_must_score() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let players = make_players(2);
        let p1_id = players[0].id;
        let mut game = GameState::new(players);
        game.start().unwrap();

        game.roll_dice(p1_id, &mut rng).unwrap();
        game.roll_dice(p1_id, &mut rng).unwrap();
        game.roll_dice(p1_id, &mut rng).unwrap();

        // Fourth roll should fail
        assert!(matches!(
            game.roll_dice(p1_id, &mut rng),
            Err(GameError::CannotRoll)
        ));

        // Must score
        assert_eq!(
            game.turn.as_ref().unwrap().phase,
            TurnPhase::MustScore
        );
    }

    #[test]
    fn test_hold_before_roll_fails() {
        let players = make_players(2);
        let p1_id = players[0].id;
        let mut game = GameState::new(players);
        game.start().unwrap();

        assert!(matches!(
            game.hold_dice(p1_id, [true, false, false, false, false]),
            Err(GameError::CannotHold)
        ));
    }

    #[test]
    fn test_full_game_two_players() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(123);
        let players = make_players(2);
        let ids: Vec<Uuid> = players.iter().map(|p| p.id).collect();
        let mut game = GameState::new(players);
        game.start().unwrap();

        let categories = Category::ALL;
        for round_idx in 0..13 {
            for player_idx in 0..2 {
                let pid = ids[player_idx];
                assert!(game.is_current_player(pid));

                // Roll once
                game.roll_dice(pid, &mut rng).unwrap();

                // Score a category (use the round_idx to pick one)
                let cat = categories[round_idx];
                game.score_category(pid, cat).unwrap();
            }
        }

        assert_eq!(game.phase, GamePhase::Finished);
        assert!(game.winner().is_some());
    }

    #[test]
    fn test_snapshot_round_trip() {
        let players = make_players(3);
        let game = GameState::new(players);
        let snap = game.snapshot();

        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: GameStateSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.players.len(), 3);
        assert_eq!(deserialized.phase, GamePhase::Lobby);
    }

    #[test]
    fn test_solo_start_with_one_player() {
        let mut game = GameState::new(make_players(1));
        assert!(game.start_solo().is_ok());
        assert_eq!(game.phase, GamePhase::Playing);
    }

    #[test]
    fn test_solo_start_with_multiple_players() {
        let mut game = GameState::new(make_players(3));
        assert!(game.start_solo().is_ok());
        assert_eq!(game.phase, GamePhase::Playing);
        assert_eq!(game.players.len(), 3);
    }

    #[test]
    fn test_hold_dice_during_turn() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let players = make_players(2);
        let p1_id = players[0].id;
        let mut game = GameState::new(players);
        game.start().unwrap();

        // Roll first
        game.roll_dice(p1_id, &mut rng).unwrap();

        // Hold dice 0 and 2
        game.hold_dice(p1_id, [true, false, true, false, false]).unwrap();
        let turn = game.turn.as_ref().unwrap();
        assert!(turn.dice.dice[0].held);
        assert!(!turn.dice.dice[1].held);
        assert!(turn.dice.dice[2].held);

        // Roll again - held dice should keep their values
        let held_val_0 = turn.dice.dice[0].value;
        let held_val_2 = turn.dice.dice[2].value;
        game.roll_dice(p1_id, &mut rng).unwrap();
        let turn = game.turn.as_ref().unwrap();
        assert_eq!(turn.dice.dice[0].value, held_val_0);
        assert_eq!(turn.dice.dice[2].value, held_val_2);
    }

    #[test]
    fn test_category_already_scored() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let players = make_players(2);
        let p1_id = players[0].id;
        let p2_id = players[1].id;
        let mut game = GameState::new(players);
        game.start().unwrap();

        // P1 rolls and scores Chance
        game.roll_dice(p1_id, &mut rng).unwrap();
        game.score_category(p1_id, Category::Chance).unwrap();

        // P2 rolls and scores Ones
        game.roll_dice(p2_id, &mut rng).unwrap();
        game.score_category(p2_id, Category::Ones).unwrap();

        // P1 tries to score Chance again - should fail
        game.roll_dice(p1_id, &mut rng).unwrap();
        assert!(matches!(
            game.score_category(p1_id, Category::Chance),
            Err(GameError::CategoryAlreadyScored)
        ));
    }

    #[test]
    fn test_yahtzee_bonus_scoring() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let players = make_players(2);
        let p1_id = players[0].id;
        let p2_id = players[1].id;
        let mut game = GameState::new(players);
        game.start().unwrap();

        // Manually set dice to all 5s for yahtzee
        game.roll_dice(p1_id, &mut rng).unwrap();
        let turn = game.turn.as_mut().unwrap();
        for die in &mut turn.dice.dice {
            die.value = 5;
        }

        // Score Yahtzee
        let score = game.score_category(p1_id, Category::Yahtzee).unwrap();
        assert_eq!(score, 50);

        // P2's turn
        game.roll_dice(p2_id, &mut rng).unwrap();
        game.score_category(p2_id, Category::Chance).unwrap();

        // P1 gets another Yahtzee
        game.roll_dice(p1_id, &mut rng).unwrap();
        let turn = game.turn.as_mut().unwrap();
        for die in &mut turn.dice.dice {
            die.value = 5;
        }

        // Score Fives (Yahtzee bonus should be triggered)
        game.score_category(p1_id, Category::Fives).unwrap();
        assert_eq!(game.players[0].scorecard.yahtzee_bonus_count, 1);
    }

    #[test]
    fn test_full_game_six_players() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(777);
        let players = make_players(6);
        let ids: Vec<Uuid> = players.iter().map(|p| p.id).collect();
        let mut game = GameState::new(players);
        game.start().unwrap();

        let categories = Category::ALL;
        for round_idx in 0..13 {
            for player_idx in 0..6 {
                let pid = ids[player_idx];
                assert!(game.is_current_player(pid));
                game.roll_dice(pid, &mut rng).unwrap();
                let cat = categories[round_idx];
                game.score_category(pid, cat).unwrap();
            }
        }

        assert_eq!(game.phase, GamePhase::Finished);
        assert!(game.winner().is_some());
        // All players should have complete scorecards
        for player in &game.players {
            assert!(player.scorecard.is_complete());
        }
    }

    #[test]
    fn test_game_not_started_actions_fail() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let players = make_players(2);
        let p1_id = players[0].id;
        let game = GameState::new(players);

        // Game is in Lobby phase, should fail
        let mut game_clone = game.clone();
        assert!(matches!(
            game_clone.roll_dice(p1_id, &mut rng),
            Err(GameError::GameNotInProgress)
        ));
    }
}
