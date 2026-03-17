use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};

#[derive(Debug, Clone)]
pub enum Action {
    None,
    Quit,
    MoveCursor(i32, i32),
    PanCamera(i32, i32),
    SaveGame,
    MouseClick { col: u16, row: u16 },
    MouseDrag { col: u16, row: u16 },
    MouseUp { col: u16, row: u16 },
    MouseMove { col: u16, row: u16 },
    MenuSelect,
    MenuBack,
    CharInput(char),
    DeleteChar,
}

pub fn translate_event(event: Event) -> Action {
    match event {
        Event::Key(key) => translate_key(key),
        Event::Mouse(mouse) => match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => Action::MouseClick {
                col: mouse.column,
                row: mouse.row,
            },
            MouseEventKind::Drag(MouseButton::Left) => Action::MouseDrag {
                col: mouse.column,
                row: mouse.row,
            },
            MouseEventKind::Up(MouseButton::Left) => Action::MouseUp {
                col: mouse.column,
                row: mouse.row,
            },
            MouseEventKind::Moved => Action::MouseMove {
                col: mouse.column,
                row: mouse.row,
            },
            MouseEventKind::ScrollUp => Action::PanCamera(0, -3),
            MouseEventKind::ScrollDown => Action::PanCamera(0, 3),
            _ => Action::None,
        },
        _ => Action::None,
    }
}

fn translate_key(key: crossterm::event::KeyEvent) -> Action {
    // Only handle Press (and Repeat for held keys); ignore Release to avoid double-firing
    if key.kind == KeyEventKind::Release {
        return Action::None;
    }

    // Ctrl combos
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('s') => Action::SaveGame,
            _ => Action::None,
        };
    }

    match key.code {
        // Cursor movement (arrow keys)
        KeyCode::Up => Action::MoveCursor(0, -1),
        KeyCode::Down => Action::MoveCursor(0, 1),
        KeyCode::Left => Action::MoveCursor(-1, 0),
        KeyCode::Right => Action::MoveCursor(1, 0),

        // Menu
        KeyCode::Enter => Action::MenuSelect,
        KeyCode::Esc => Action::MenuBack,
        KeyCode::Backspace => Action::DeleteChar,

        // All printable characters — context decides meaning
        KeyCode::Char(c) => Action::CharInput(c),

        _ => Action::None,
    }
}
