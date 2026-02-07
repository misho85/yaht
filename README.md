# YAHT - Yet Another Hacky Terminal (Yahtzee)

A multiplayer Yahtzee game played in the terminal, built with Rust.

```text
┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐
│*   *│  │  *  │  │*   *│  │*   *│  │*   *│
│  *  │  │  *  │  │     │  │*   *│  │  *  │
│*   *│  │  *  │  │*   *│  │*   *│  │*   *│
└─────┘  └─────┘  └─────┘  └─────┘  └─────┘
```

## Features

- **2-6 players** over TCP
- **Terminal UI** with ratatui - ASCII dice, live scoreboard, chat
- **Dice rolling animation** with random face cycling
- **Score flash highlights** when categories are scored
- **Room system** - create, join, spectate
- **Chat** during gameplay
- **Full Yahtzee rules** - 13 categories, upper bonus (35 at ≥63), Yahtzee bonus (+100)

## Architecture

```text
yaht/
├── crates/
│   ├── yaht-common/   # Shared types: dice, scoring, game logic, protocol
│   ├── yaht-server/   # TCP server with async room/lobby management
│   └── yaht-client/   # TUI client with ratatui
```

- **Networking**: Async TCP with `tokio` + `LengthDelimitedCodec` framing + JSON serialization
- **Protocol**: Request/response messages (`ClientMessage`/`ServerMessage` enums)
- **Game engine**: Turn state machine (WaitingForRoll → Rolling → MustScore → Done)
- **UI**: Screen state machine (Connect → Lobby → WaitingRoom → Game → Results)

## Quick Start

### Start the server

```sh
cargo run -p yaht-server
```

Default port: `9876`

### Start a client (in another terminal)

```sh
cargo run -p yaht-client
```

Repeat for each player (minimum 2 to start a game).

## How to Play

### Connect

1. Enter your player name
2. Tab to the server field (default `127.0.0.1:9876`)
3. Press Enter to connect

### Lobby

| Key     | Action         |
| ------- | -------------- |
| `c`     | Create room    |
| `Enter` | Join room      |
| `s`     | Spectate room  |
| `r`     | Refresh list   |
| `j`/`k` | Navigate rooms |
| `q`     | Quit           |

### Waiting Room

| Key     | Action                 |
| ------- | ---------------------- |
| `Enter` | Start game (host only) |
| `Esc`   | Leave room             |

### Game

| Key     | Action              |
| ------- | ------------------- |
| `r`     | Roll dice           |
| `1`-`5` | Toggle hold on die  |
| `j`/`k` | Navigate categories |
| `s`     | Score category      |
| `c`     | Toggle chat         |
| `q`     | Quit                |

In chat mode, type your message and press Enter to send. Esc exits chat.

### Scoring

Each player gets 13 rounds. Per turn: up to 3 rolls, hold any dice between rolls, then pick a category.

**Upper Section** (sum of matching dice):

| Category | Target |
| -------- | ------ |
| Ones     | 1s     |
| Twos     | 2s     |
| Threes   | 3s     |
| Fours    | 4s     |
| Fives    | 5s     |
| Sixes    | 6s     |

Upper bonus: **+35** if upper total ≥ 63

**Lower Section**:

| Category        | Score           |
| --------------- | --------------- |
| Three of a Kind | Sum of all dice |
| Four of a Kind  | Sum of all dice |
| Full House      | 25              |
| Small Straight  | 30              |
| Large Straight  | 40              |
| Yahtzee         | 50              |
| Chance          | Sum of all dice |

Yahtzee bonus: **+100** for each additional Yahtzee (after first scored as 50)

## Building

```sh
cargo build --release
```

Binaries will be in `target/release/`:

- `yaht-server`
- `yaht-client`

## Running Tests

```sh
cargo test
```

43 unit tests covering dice, scoring, game state machine, player/scorecard, and protocol serialization.
