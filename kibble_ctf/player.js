function update(controls, state) {
    state.x += controls.move_x * 10.;
    state.y += controls.move_y * 10.;
    return state;
}