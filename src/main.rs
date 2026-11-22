use std::f32::consts::FRAC_PI_2;

use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

const MOUSE_SENS: f32 = 0.002; //radians per mouse count
const MOVE_SPEED: f32 = 6.0; //meters per second
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01; //stop just short of straight up/down

//this entity is the player
#[derive(Component)]
struct Player;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (grab_cursor, mouse_look, move_player))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    //floor, a box with its top surface at y = 0
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(40.0, 0.2, 40.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));

    //cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    //sun
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    //player is just a camera at eye height
    commands.spawn((
        Player,
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.7, 6.0),
    ));
}

//click to capture mouse, esc to release
fn grab_cursor(
    mut cursor_options: Single<&mut CursorOptions>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
    }
}

//mouse movement -> yaw (turn left/right) and pitch (look up/down).
fn mouse_look(
    mouse_motion: Res<AccumulatedMouseMotion>,
    cursor_options: Single<&CursorOptions>,
    mut player: Single<&mut Transform, With<Player>>,
) {
    // Only look around while the cursor is captured
    if cursor_options.grab_mode == CursorGrabMode::None {
        return;
    }
    let delta = mouse_motion.delta;
    if delta == Vec2::ZERO {
        return;
    }

    let (yaw, pitch, _roll) = player.rotation.to_euler(EulerRot::YXZ);
    let yaw = yaw - delta.x * MOUSE_SENS;
    let pitch = (pitch - delta.y * MOUSE_SENS).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    player.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
}

//WASD moves along the floor relative to where you're facing
fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut player: Single<&mut Transform, With<Player>>,
) {
    //use only the yaw, prevents noclip
    let (yaw, _pitch, _roll) = player.rotation.to_euler(EulerRot::YXZ);
    let yaw_rotation = Quat::from_rotation_y(yaw);
    let forward = yaw_rotation * Vec3::NEG_Z;
    let right = yaw_rotation * Vec3::X;

    let mut direction = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        direction += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        direction -= right;
    }

    if direction != Vec3::ZERO {
        //normalize so diagonal movement isn't faster
        player.translation += direction.normalize() * MOVE_SPEED * time.delta_secs();
    }
}