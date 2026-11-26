use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_fps_controller::controller::*;

//click to capture the mouse (and enable the controller), esc to release
pub fn manage_cursor(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor: Single<&mut CursorOptions>,
    mut controllers: Query<&mut FpsController>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
        for mut controller in &mut controllers {
            controller.enable_input = true;
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
        for mut controller in &mut controllers {
            controller.enable_input = false;
        }
    }
}
