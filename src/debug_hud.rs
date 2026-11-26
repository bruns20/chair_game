use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_fps_controller::controller::*;

#[derive(Component)]
pub struct DebugHud;

//debug HUD: shows look angles + raw mouse motion
pub fn spawn_debug_hud(commands: &mut Commands) {
    commands.spawn((
        Text::new("debug"),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        DebugHud,
    ));
}

//shows the look angles and raw mouse counts so we can see what's actually happening
pub fn update_debug_hud(
    mut mouse_events: MessageReader<MouseMotion>,
    input: Single<&FpsControllerInput>,
    mut text: Single<&mut Text, With<DebugHud>>,
) {
    let (mut dx, mut dy) = (0.0_f32, 0.0_f32);
    for event in mouse_events.read() {
        dx += event.delta.x;
        dy += event.delta.y;
    }
    text.0 = format!(
        "yaw: {:7.1}   pitch: {:7.1}\nmouse: {:+9.0} , {:+9.0}\narrow keys: look (RDP test)",
        input.yaw.to_degrees(),
        input.pitch.to_degrees(),
        dx,
        dy
    );
}
