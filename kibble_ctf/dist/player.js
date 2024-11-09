export const update = (current_position, controls) => {
    console.log("Old position", current_position, "Controls", controls);
    const new_position = [
        current_position[0] + controls.move_direction[0],
        0,
        current_position[2] + controls.move_direction[1],
    ];
    console.log("New position:", new_position);
    return new_position;
};
