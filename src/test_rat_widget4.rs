use rat_widget::text_input::{TextInputState, TextInput};
use rat_event::HandleEvent;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

fn main() {
    let mut state = TextInputState::default();
    let ev = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
    let outcome = rat_widget::text_input::handle_events(&mut state, true, &ev);
}
