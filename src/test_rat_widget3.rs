use rat_widget::slider::SliderState;
fn main() {
    let mut state = SliderState::default();
    state.set_value(50.0);
    let v: f64 = state.value();
}
