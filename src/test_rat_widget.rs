use rat_widget::text_input::TextInputState;
use rat_widget::slider::SliderState;
use rat_widget::button::ButtonState;

pub struct State {
    city_name: TextInputState,
    seed_input: TextInputState,
    water_slider: SliderState,
    trees_slider: SliderState,
    regen_btn: ButtonState,
    start_btn: ButtonState,
    back_btn: ButtonState,
}
impl State {
    pub fn new() -> Self {
        Self {
            city_name: TextInputState::default(),
            seed_input: TextInputState::default(),
            water_slider: SliderState::default(),
            trees_slider: SliderState::default(),
            regen_btn: ButtonState::default(),
            start_btn: ButtonState::default(),
            back_btn: ButtonState::default(),
        }
    }
}
