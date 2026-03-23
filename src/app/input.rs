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
    MouseMiddleDown { col: u16, row: u16 },
    MouseMiddleDrag { col: u16, row: u16 },
    MouseMiddleUp,
    MouseMove { col: u16, row: u16 },
    MenuSelect,
    MenuBack,
    MenuActivate,
    CharInput(char),
    DeleteChar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiEvent {
    Activity,
    Resize { cols: u16, rows: u16 },
}

pub fn terminal_ui_event(event: &Event) -> UiEvent {
    match event {
        Event::Resize(cols, rows) => UiEvent::Resize {
            cols: *cols,
            rows: *rows,
        },
        _ => UiEvent::Activity,
    }
}

pub fn translate_terminal_event(event: Event) -> Action {
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
            MouseEventKind::Down(MouseButton::Middle) => Action::MouseMiddleDown {
                col: mouse.column,
                row: mouse.row,
            },
            MouseEventKind::Drag(MouseButton::Middle) => Action::MouseMiddleDrag {
                col: mouse.column,
                row: mouse.row,
            },
            MouseEventKind::Up(MouseButton::Middle) => Action::MouseMiddleUp,
            MouseEventKind::Moved => Action::MouseMove {
                col: mouse.column,
                row: mouse.row,
            },
            MouseEventKind::ScrollUp => Action::PanCamera(0, -3),
            MouseEventKind::ScrollDown => Action::PanCamera(0, 3),
            MouseEventKind::ScrollLeft => Action::PanCamera(-3, 0),
            MouseEventKind::ScrollRight => Action::PanCamera(3, 0),
            _ => Action::None,
        },
        _ => Action::None,
    }
}

#[allow(dead_code)]
pub fn translate_miniquad_key(key: miniquad::KeyCode, keymods: miniquad::KeyMods) -> Action {
    use miniquad::KeyCode;

    if key == KeyCode::F1 {
        return Action::MenuActivate;
    }

    if keymods.ctrl {
        return match key {
            KeyCode::C | KeyCode::Q => Action::Quit,
            KeyCode::S => Action::SaveGame,
            _ => Action::None,
        };
    }

    match key {
        KeyCode::Up => Action::MoveCursor(0, -1),
        KeyCode::Down => Action::MoveCursor(0, 1),
        KeyCode::Left => Action::MoveCursor(-1, 0),
        KeyCode::Right => Action::MoveCursor(1, 0),
        KeyCode::Enter | KeyCode::KpEnter => Action::MenuSelect,
        KeyCode::Escape => Action::MenuBack,
        KeyCode::Backspace => Action::DeleteChar,
        KeyCode::Tab => Action::CharInput('\t'),
        _ => Action::None,
    }
}

#[allow(dead_code)]
pub fn translate_miniquad_char(character: char, keymods: miniquad::KeyMods) -> Action {
    if keymods.ctrl || character.is_control() {
        Action::None
    } else {
        Action::CharInput(character)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};

    #[test]
    fn translate_middle_drag_event() {
        let event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Middle),
            column: 12,
            row: 7,
            modifiers: KeyModifiers::empty(),
        });
        assert!(matches!(
            translate_terminal_event(event),
            Action::MouseMiddleDrag { col: 12, row: 7 }
        ));
    }

    #[test]
    fn translate_horizontal_scroll_event() {
        let event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollRight,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        });
        assert!(matches!(
            translate_terminal_event(event),
            Action::PanCamera(3, 0)
        ));
    }
}

fn translate_key(key: crossterm::event::KeyEvent) -> Action {
    // Only handle Press (and Repeat for held keys); ignore Release to avoid double-firing
    if key.kind == KeyEventKind::Release {
        return Action::None;
    }

    // F-keys (no modifiers needed)
    if key.code == KeyCode::F(1) {
        return Action::MenuActivate;
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

        // Tab cycles overlays
        KeyCode::Tab => Action::CharInput('\t'),

        // All printable characters — context decides meaning
        KeyCode::Char(c) => Action::CharInput(c),

        _ => Action::None,
    }
}
