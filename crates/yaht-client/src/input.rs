use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::Screen;

#[derive(Debug, Clone)]
pub enum Action {
    // Global
    Quit,
    ShowHelp,

    // Text input
    TypeChar(char),
    Backspace,
    Submit,

    // Navigation
    NavigateUp,
    NavigateDown,

    // Connect screen
    SwitchField,

    // Lobby
    RefreshRooms,
    CreateRoom,
    JoinSelected,
    SpectateSelected,
    StartGame,
    LeaveRoom,

    // Game
    RollDice,
    ToggleHold(usize),
    ConfirmScore,
    ToggleChatFocus,
    SendChat,

    // Results
    BackToLobby,
}

pub fn map_key(key: KeyEvent, screen: &Screen, chat_focused: bool) -> Option<Action> {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }

    // Chat input mode
    if chat_focused {
        return match key.code {
            KeyCode::Enter => Some(Action::SendChat),
            KeyCode::Esc => Some(Action::ToggleChatFocus),
            KeyCode::Char(c) => Some(Action::TypeChar(c)),
            KeyCode::Backspace => Some(Action::Backspace),
            _ => None,
        };
    }

    match screen {
        Screen::Connect(_) => match key.code {
            KeyCode::Enter => Some(Action::Submit),
            KeyCode::Tab => Some(Action::SwitchField),
            KeyCode::Char(c) => Some(Action::TypeChar(c)),
            KeyCode::Backspace => Some(Action::Backspace),
            KeyCode::Esc => Some(Action::Quit),
            _ => None,
        },

        Screen::Lobby(s) if s.is_in_room() => match key.code {
            KeyCode::Enter => Some(Action::StartGame),
            KeyCode::Esc => Some(Action::LeaveRoom),
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('?') => Some(Action::ShowHelp),
            _ => None,
        },

        Screen::Lobby(_) => match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('r') => Some(Action::RefreshRooms),
            KeyCode::Char('c') => Some(Action::CreateRoom),
            KeyCode::Char('s') => Some(Action::SpectateSelected),
            KeyCode::Enter => Some(Action::JoinSelected),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::NavigateUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::NavigateDown),
            KeyCode::Char('?') => Some(Action::ShowHelp),
            KeyCode::Esc => Some(Action::Quit),
            _ => None,
        },

        Screen::Game(_) => match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('r') | KeyCode::Char('R') => Some(Action::RollDice),
            KeyCode::Char('1') => Some(Action::ToggleHold(0)),
            KeyCode::Char('2') => Some(Action::ToggleHold(1)),
            KeyCode::Char('3') => Some(Action::ToggleHold(2)),
            KeyCode::Char('4') => Some(Action::ToggleHold(3)),
            KeyCode::Char('5') => Some(Action::ToggleHold(4)),
            KeyCode::Char('s') | KeyCode::Char('S') => Some(Action::ConfirmScore),
            KeyCode::Char('c') | KeyCode::Char('C') => Some(Action::ToggleChatFocus),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::NavigateUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::NavigateDown),
            KeyCode::Enter => Some(Action::ConfirmScore),
            KeyCode::Char('?') => Some(Action::ShowHelp),
            _ => None,
        },

        Screen::Results(_) => match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Enter => Some(Action::BackToLobby),
            KeyCode::Esc => Some(Action::Quit),
            _ => None,
        },
    }
}
