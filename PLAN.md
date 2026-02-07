# yaht — Terminal-Based Multiplayer Yahtzee in Rust

## Overview

**yaht** is a multiplayer Yahtzee game played entirely in the terminal, built with Rust. Players connect to a shared game server over TCP and interact through a rich terminal UI. The game supports 2–4 players, enforces official Yahtzee rules (including Yahtzee bonus and joker rules), and provides a polished, responsive TUI experience.

---

## Architecture

### High-Level Diagram

```
┌──────────────┐     TCP      ┌──────────────────────┐     TCP      ┌──────────────┐
│   Client A   │◄────────────►│                      │◄────────────►│   Client B   │
│  (ratatui)   │              │    Game Server        │              │  (ratatui)   │
└──────────────┘              │                      │              └──────────────┘
                              │  - Auth. game state  │
┌──────────────┐              │  - Turn management   │              ┌──────────────┐
│   Client C   │◄────────────►│  - Rule validation   │◄────────────►│   Client D   │
│  (ratatui)   │              │  - State broadcast   │              │  (ratatui)   │
└──────────────┘              └──────────────────────┘              └──────────────┘
```

### Model: Client–Server with Authoritative Server

- **Server** holds the single source of truth for all game state.
- **Clients** are thin — they render state and send player actions.
- Server validates every action before applying it, preventing cheating.
- All state changes are broadcast to every connected client.

### Tech Stack

| Layer         | Crate / Tool        | Purpose                              |
|---------------|---------------------|--------------------------------------|
| Async runtime | `tokio`             | Async TCP, tasks, channels           |
| TUI rendering | `ratatui`           | Terminal widgets and layout           |
| TUI backend   | `crossterm`         | Cross-platform terminal I/O          |
| Serialization | `serde` + `bincode` | Fast binary message encoding         |
| RNG           | `rand`              | Dice rolls (server-side only)        |
| CLI args      | `clap`              | Command-line argument parsing        |

---

## Project Structure

```
yaht/
├── Cargo.toml
├── PLAN.md
├── src/
│   ├── main.rs              # Entry point — CLI dispatch (server / client)
│   ├── protocol.rs          # Shared message types (ClientMsg, ServerMsg)
│   ├── game/
│   │   ├── mod.rs
│   │   ├── dice.rs           # Dice roll logic
│   │   ├── scorecard.rs      # Scoring categories, validation, calculation
│   │   ├── rules.rs          # Turn flow, joker rules, game-over detection
│   │   └── state.rs          # GameState, PlayerState structs
│   ├── server/
│   │   ├── mod.rs
│   │   ├── lobby.rs          # Pre-game lobby, player join/ready
│   │   ├── session.rs        # Per-client TCP task
│   │   └── engine.rs         # Main game loop, action processing
│   └── client/
│       ├── mod.rs
│       ├── net.rs            # TCP connection, send/recv messages
│       ├── app.rs            # Application state, event handling
│       └── ui/
│           ├── mod.rs
│           ├── board.rs      # Scorecard table widget
│           ├── dice.rs       # Dice display (ASCII art)
│           ├── status.rs     # Turn info, messages, player list
│           └── input.rs      # Key bindings, input mode handling
└── tests/
    ├── scoring_tests.rs
    ├── rules_tests.rs
    └── protocol_tests.rs
```

### Single Binary, Two Modes

```sh
yaht server --port 7777 --players 3     # Start a game server
yaht join   --host 192.168.1.5:7777 --name "Alice"   # Join as a player
```

Both `server` and `client` compile into one binary. `clap` subcommands dispatch to the appropriate mode.

---

## Game Rules (Full Reference)

### Turn Flow

Each player gets **13 turns** (one per scoring category). On each turn:

1. **Roll 1** — All 5 dice are rolled automatically.
2. **Hold/Release** — Player selects which dice to keep.
3. **Roll 2** — Remaining dice are re-rolled.
4. **Hold/Release** — Player selects again.
5. **Roll 3** — Final re-roll of remaining dice.
6. **Score** — Player must assign the result to exactly one unused category.

A player may choose to score after any roll (they don't have to use all 3 rolls).

### Scoring Categories

#### Upper Section

| Category | Rule                        | Score             |
|----------|-----------------------------|-------------------|
| Ones     | Count dice showing 1        | Sum of 1s         |
| Twos     | Count dice showing 2        | Sum of 2s         |
| Threes   | Count dice showing 3        | Sum of 3s         |
| Fours    | Count dice showing 4        | Sum of 4s         |
| Fives    | Count dice showing 5        | Sum of 5s         |
| Sixes    | Count dice showing 6        | Sum of 6s         |

**Upper Bonus**: If upper section total >= 63, award **+35 points**.

#### Lower Section

| Category       | Rule                                        | Score          |
|----------------|---------------------------------------------|----------------|
| Three of a Kind| At least 3 dice with same value             | Sum of all dice|
| Four of a Kind | At least 4 dice with same value             | Sum of all dice|
| Full House     | 3 of one value + 2 of another               | 25 (fixed)     |
| Small Straight | 4 consecutive values (e.g. 1-2-3-4)         | 30 (fixed)     |
| Large Straight | 5 consecutive values (e.g. 2-3-4-5-6)       | 40 (fixed)     |
| Yahtzee        | All 5 dice same value                       | 50 (fixed)     |
| Chance         | Any combination                             | Sum of all dice|

### Yahtzee Bonus

If a player scores a Yahtzee (50 pts) and later rolls another Yahtzee:
- **+100 bonus** per additional Yahtzee (tracked separately, can stack).

### Joker Rules (Forced Joker — Official)

When a player rolls a Yahtzee and the Yahtzee box is already filled (with 50):
1. Score the corresponding upper section box if available (e.g., five 4s → Fours = 20).
2. If that upper box is used, score any open lower section box. The Yahtzee acts as a **joker**: Full House scores 25, Small Straight scores 30, Large Straight scores 40, regardless of actual dice.
3. If no lower section boxes are open, score 0 in any open upper section box.

If the Yahtzee box was scored as 0 (crossed out), no bonus is awarded and no joker rules apply.

### Game End

The game ends when all players have filled all 13 categories. Final score = upper total + upper bonus + lower total + Yahtzee bonuses. Highest score wins.

---

## Network Protocol

### Message Types

All messages are serialized with `serde` + `bincode` over TCP with length-prefixed framing (`u32` length header).

```
Wire format:  [4 bytes: payload length (big-endian u32)] [payload bytes]
```

#### Client → Server (`ClientMsg`)

| Message          | Fields                    | Description                         |
|------------------|---------------------------|-------------------------------------|
| `Join`           | `name: String`            | Request to join the game            |
| `Ready`          | —                         | Signal ready to start               |
| `RollDice`       | `hold: [bool; 5]`        | Roll, keeping held dice             |
| `ScoreCategory`  | `category: Category`     | Assign current dice to a category   |
| `Chat`           | `text: String`            | Send a chat message                 |
| `Leave`          | —                         | Disconnect gracefully               |

#### Server → Client (`ServerMsg`)

| Message            | Fields                           | Description                          |
|--------------------|----------------------------------|--------------------------------------|
| `Welcome`          | `player_id: u8, players: Vec`    | Confirm join, list current players   |
| `PlayerJoined`     | `id: u8, name: String`           | New player joined the lobby          |
| `PlayerLeft`       | `id: u8`                         | Player disconnected                  |
| `GameStarted`      | `player_order: Vec<u8>`          | Game begins, turn order decided      |
| `TurnStarted`      | `player_id: u8, dice: [u8; 5]`  | A player's turn begins (auto roll 1) |
| `DiceRolled`       | `dice: [u8; 5], rolls_left: u8` | Result of a dice roll                |
| `ScoreRecorded`    | `player_id, category, score`     | A category was scored                |
| `GameOver`         | `final_scores: Vec<(u8, u16)>`  | Game ended with final standings      |
| `ChatMessage`      | `from: u8, text: String`        | Relayed chat message                 |
| `Error`            | `msg: String`                    | Invalid action, not your turn, etc.  |

---

## Server Design

### Lobby Phase

```
[Players connect] → server accepts up to N players
                  → broadcasts PlayerJoined to all
[All players Ready] → server starts game
                    → randomizes turn order
                    → broadcasts GameStarted
                    → begins first turn
```

### Game Loop (engine.rs)

```
for round in 0..13 {
    for player_id in turn_order {
        1. Roll all 5 dice → send TurnStarted
        2. Wait for player action:
           - RollDice { hold } → re-roll, send DiceRolled (max 2 more rolls)
           - ScoreCategory { cat } → validate, record, send ScoreRecorded
        3. Move to next player
    }
}
→ Calculate final scores → send GameOver
```

### Concurrency Model

```
                          ┌─────────────────────────┐
  client A task ──mpsc──► │                         │ ──broadcast──► client A task
  client B task ──mpsc──► │   Game Engine Task       │ ──broadcast──► client B task
  client C task ──mpsc──► │   (single-threaded logic)│ ──broadcast──► client C task
                          └─────────────────────────┘
```

- **One tokio task per client** handles TCP read/write.
- Client tasks forward `ClientMsg` into a shared **mpsc channel**.
- A single **game engine task** reads from the mpsc, processes game logic, and sends updates via a **broadcast channel**.
- This avoids locks — all mutable game state lives in the engine task.

### Disconnect Handling

- If a player disconnects mid-game, their remaining turns are auto-scored as 0.
- Server broadcasts `PlayerLeft` so other clients can show the disconnect.
- Game continues with remaining players.

---

## Client Design

### Event Loop

```
loop {
    select! {
        // Terminal input (crossterm events)
        key_event = input_reader => handle_input(key_event),

        // Network messages from server
        server_msg = net_receiver => handle_server_msg(server_msg),

        // Tick for animations (dice roll animation, blinking cursor)
        _ = tick_interval => { /* redraw */ }
    }
    draw(&mut terminal, &app_state)?;
}
```

### TUI Layout

```
┌─────────────────────────────────────────────────────────────────┐
│  yaht — Yahtzee                              Round 5/13        │
├───────────────────────────────────┬─────────────────────────────┤
│                                   │  SCORECARD                  │
│   ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐ │                             │
│   │ ⚂ │ │ ⚄ │ │ ⚀ │ │ ⚂ │ │ ⚅ │ │  Category     You  Opp    │
│   └───┘ └───┘ └───┘ └───┘ └───┘ │  ─────────────────────────  │
│    [1]   [2]  *[3]*  [4]   [5]  │  Ones          3    2       │
│                                   │  Twos          —    6       │
│   Roll 2 of 3                     │  Threes       [9]   —       │
│   Hold: toggle 1-5, Enter: roll   │  ...                        │
│                                   │  Yahtzee       —    50      │
│   Players:                        │  Chance        —    —       │
│   ► Alice (you) ← current turn   │                             │
│     Bob                           │  Upper Bonus   —    35      │
│     Charlie                       │  TOTAL        102  187      │
├───────────────────────────────────┴─────────────────────────────┤
│  [Chat] Alice: nice roll!                                       │
│  > _                                                            │
└─────────────────────────────────────────────────────────────────┘
```

### Key Bindings

| Key         | Action                                         |
|-------------|-------------------------------------------------|
| `1`–`5`     | Toggle hold on dice 1–5                        |
| `Enter`     | Roll dice / confirm category selection          |
| `Tab`       | Switch focus between dice area and scorecard    |
| `↑` / `↓`  | Navigate scorecard categories                  |
| `Enter`     | Score in highlighted category (when in scorecard)|
| `/`         | Open chat input                                 |
| `Esc`       | Cancel / close chat                            |
| `q`         | Quit game                                      |

### Dice ASCII Art

```
┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐
│     │  │ ●   │  │ ●   │  │ ● ● │  │ ● ● │  │ ● ● │
│  ●  │  │     │  │  ●  │  │     │  │  ●  │  │ ● ● │
│     │  │   ● │  │   ● │  │ ● ● │  │ ● ● │  │ ● ● │
└─────┘  └─────┘  └─────┘  └─────┘  └─────┘  └─────┘
  [1]      [2]      [3]      [4]      [5]      [6]
```

Held dice are shown with a highlighted/inverted border. A short roll animation (rapid cycling) plays when dice are rolled.

---

## Implementation Phases

### Phase 1 — Core Game Logic (no network, no TUI)

**Goal**: Get the rules right, fully tested.

- [ ] `dice.rs` — Roll function, hold logic
- [ ] `scorecard.rs` — All 13 category scoring functions
- [ ] `rules.rs` — Turn validation, Yahtzee bonus/joker rules, game-over check
- [ ] `state.rs` — `GameState`, `PlayerState` structs
- [ ] Unit tests for every scoring category, edge cases, joker rules

### Phase 2 — Protocol & Networking

**Goal**: Server accepts connections, manages lobby, processes a full game.

- [ ] `protocol.rs` — Define `ClientMsg` / `ServerMsg` with serde + bincode
- [ ] Length-prefixed TCP framing (async read/write helpers)
- [ ] `server/session.rs` — Per-client connection task
- [ ] `server/lobby.rs` — Join, ready-up, start game
- [ ] `server/engine.rs` — Game loop, action validation, state broadcast
- [ ] Integration test: simulate a full game with mock clients

### Phase 3 — Terminal UI

**Goal**: Rich, interactive client TUI.

- [ ] `client/app.rs` — Client state machine (Lobby → Playing → GameOver)
- [ ] `client/net.rs` — Async TCP connection, message send/recv
- [ ] `client/ui/dice.rs` — Dice rendering with ASCII art, hold indicators
- [ ] `client/ui/board.rs` — Scorecard table with all players' scores
- [ ] `client/ui/status.rs` — Turn info, player list, round counter
- [ ] `client/ui/input.rs` — Key bindings, mode switching
- [ ] Dice roll animation

### Phase 4 — Polish & UX

**Goal**: Feels good to play.

- [ ] Chat system (in-game messages)
- [ ] Sound/bell on your turn
- [ ] Color theme (category groups, active player highlight)
- [ ] Score preview — show potential score for each category before committing
- [ ] Graceful disconnect/reconnect handling
- [ ] Error messages shown inline (not your turn, invalid category, etc.)

### Phase 5 — Extras (Optional)

- [ ] AI player (basic strategy: maximize expected score)
- [ ] Solo mode (play against AI without networking)
- [ ] Game replay / history log
- [ ] Spectator mode
- [ ] Custom rules (e.g., number of rolls, extra categories)

---

## Key Design Decisions

| Decision                  | Choice                 | Rationale                                      |
|---------------------------|------------------------|------------------------------------------------|
| Client–Server vs P2P      | Client–Server          | Simpler, authoritative state, prevents cheating|
| Serialization             | serde + bincode        | Fast, compact, Rust-native                     |
| TCP vs UDP                | TCP                    | Turn-based game needs reliability, not speed   |
| Single binary vs separate | Single binary          | Simpler distribution, `clap` subcommands       |
| Server-side RNG           | Yes                    | Clients can't manipulate dice rolls            |
| Async runtime             | tokio                  | Industry standard, excellent TCP support       |
| TUI framework             | ratatui + crossterm    | Best maintained, cross-platform, rich widgets  |
| Game state sync           | Full state broadcast   | Simple, correct for low-frequency turn-based   |

---

## Running (Target UX)

```sh
# Terminal 1 — Start server
cargo run -- server --port 7777 --players 2

# Terminal 2 — Player 1
cargo run -- join --host 127.0.0.1:7777 --name Alice

# Terminal 3 — Player 2
cargo run -- join --host 127.0.0.1:7777 --name Bob

# Both players type 'r' to ready up → game starts
```

---

## Dependencies (Cargo.toml)

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
ratatui = "0.29"
crossterm = "0.28"
serde = { version = "1", features = ["derive"] }
bincode = "1"
rand = "0.8"
clap = { version = "4", features = ["derive"] }
```
