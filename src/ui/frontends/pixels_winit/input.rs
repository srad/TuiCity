use winit::{
    event::{ElementState, MouseScrollDelta},
    keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey},
};

use crate::app::input::Action;

use super::font::{cell_h, cell_w};

/// Translate a winit 0.29 `KeyEvent` into an `Action`.
/// Returns `Action::None` for releases and unmapped keys.
pub fn translate_key_event(
    event: &winit::event::KeyEvent,
    mods: ModifiersState,
) -> Action {
    if event.state != ElementState::Pressed {
        return Action::None;
    }

    // Handle named / structural keys via the physical key (layout-independent)
    match event.physical_key {
        PhysicalKey::Code(KeyCode::Escape) => return Action::MenuBack,
        PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => {
            return Action::MenuSelect;
        }
        PhysicalKey::Code(KeyCode::ArrowUp) => return Action::MoveCursor(0, -1),
        PhysicalKey::Code(KeyCode::ArrowDown) => return Action::MoveCursor(0, 1),
        PhysicalKey::Code(KeyCode::ArrowLeft) => return Action::MoveCursor(-1, 0),
        PhysicalKey::Code(KeyCode::ArrowRight) => return Action::MoveCursor(1, 0),
        PhysicalKey::Code(KeyCode::Backspace | KeyCode::Delete) => {
            return Action::DeleteChar;
        }
        PhysicalKey::Code(KeyCode::F5) => return Action::SaveGame,
        PhysicalKey::Code(KeyCode::KeyW | KeyCode::KeyK) if !mods.shift_key() => {
            return Action::PanCamera(0, -3);
        }
        PhysicalKey::Code(KeyCode::KeyS | KeyCode::KeyJ) if !mods.shift_key() => {
            return Action::PanCamera(0, 3);
        }
        PhysicalKey::Code(KeyCode::KeyA | KeyCode::KeyH) if !mods.shift_key() => {
            return Action::PanCamera(-3, 0);
        }
        PhysicalKey::Code(KeyCode::KeyD | KeyCode::KeyL) if !mods.shift_key() => {
            return Action::PanCamera(3, 0);
        }
        PhysicalKey::Code(KeyCode::KeyQ) if mods.control_key() => return Action::Quit,
        _ => {}
    }

    // Character input from the key's produced text (respects keyboard layout)
    if let Some(text) = &event.text {
        if let Some(ch) = text.chars().next() {
            if !ch.is_control() {
                return Action::CharInput(ch);
            }
        }
    }

    // Named keys that produce no text but carry semantic meaning
    match &event.logical_key {
        Key::Named(NamedKey::Space) => return Action::CharInput(' '),
        _ => {}
    }

    Action::None
}

/// Convert physical pixel coordinates to cell coordinates.
///
/// Returns (col, row) where each unit is one cell_w × cell_h pixel block.
/// `Camera::col_scale` is set to 1 for this frontend so `screen_to_map`
/// does not divide the column by 2.
pub fn pixels_to_cell(px: f64, py: f64, scale: u32) -> (u16, u16) {
    let col = (px as u32 / cell_w(scale)).min(u16::MAX as u32) as u16;
    let row = (py as u32 / cell_h(scale)).min(u16::MAX as u32) as u16;
    (col, row)
}

/// Translate a mouse wheel delta into a pan action.
pub fn translate_scroll(delta: &MouseScrollDelta) -> Action {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => {
            let dx = if *x > 0.5 { 3 } else if *x < -0.5 { -3 } else { 0 };
            let dy = if *y > 0.5 { -3 } else if *y < -0.5 { 3 } else { 0 };
            if dx != 0 || dy != 0 { Action::PanCamera(dx, dy) } else { Action::None }
        }
        MouseScrollDelta::PixelDelta(pos) => {
            let dy = if pos.y > 10.0 { -3 } else if pos.y < -10.0 { 3 } else { 0 };
            let dx = if pos.x > 10.0 { 3 } else if pos.x < -10.0 { -3 } else { 0 };
            if dx != 0 || dy != 0 { Action::PanCamera(dx, dy) } else { Action::None }
        }
    }
}
