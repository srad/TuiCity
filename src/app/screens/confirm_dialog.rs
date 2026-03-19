pub(crate) fn cycle_selection(selected: &mut usize, button_count: usize, delta: i32) {
    if button_count == 0 {
        *selected = 0;
        return;
    }

    let current = (*selected).min(button_count.saturating_sub(1)) as i32;
    let next = (current + delta).rem_euclid(button_count as i32) as usize;
    *selected = next;
}
