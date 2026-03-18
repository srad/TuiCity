use rat_widget::text_input::TextInputState;
fn main() {
    let mut state = TextInputState::default();
    state.set_text("hello");
    let s: &str = state.text();
}
