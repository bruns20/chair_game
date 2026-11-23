use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_fps_controller::controller::*;
use bevy_rapier3d::prelude::*;

const SPAWN_POINT: Vec3 = Vec3::new(0.0, 1.625, 6.0);

//scripted test runs for screenshots, only with `--features betty` (see src/betty.rs)
#[cfg(feature = "betty")]
mod betty;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<()>::default())
        .add_plugins(FpsControllerPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (manage_cursor, arrow_key_look, update_debug_hud));
    #[cfg(feature = "betty")]
    app.add_plugins(betty::BettyPlugin);
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    //floor, a box with its top surface at y = 0, plus a physics collider
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(40.0, 0.2, 40.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        RigidBody::Fixed,
        Collider::cuboid(20.0, 0.1, 20.0),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));

    //cube — now something you can walk into
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        RigidBody::Fixed,
        Collider::cuboid(0.5, 0.5, 0.5),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    //sun
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    //player: two entities.
    //the "logical" player is the invisible physics body the controller drives,
    //the "render" player is the camera that follows it.
    let logical_entity = commands
        .spawn((
            Collider::cylinder(1.5, 0.5),
            Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            Restitution {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            ActiveEvents::COLLISION_EVENTS,
            Velocity::zero(),
            RigidBody::Dynamic,
            Sleeping::disabled(),
            LockedAxes::ROTATION_LOCKED,
            AdditionalMassProperties::Mass(1.0),
            //the controller does its own Source-style gravity
            GravityScale(0.0),
            Ccd { enabled: true }, //prevents clipping through thin geometry at speed
            Transform::from_translation(SPAWN_POINT),
            LogicalPlayer,
            FpsControllerInput {
                pitch: -TAU / 12.0,
                ..default()
            },
            FpsController {
                //radians per mouse count — lower = slower look
                //NOTE: 100x reduction because testing over RDP inflates mouse counts
                sensitivity: 0.00000125,
                ..default()
            },
        ))
        .insert(CameraConfig {
            height_offset: -0.5,
        })
        .id();

    commands.spawn((
        Camera3d::default(),
        RenderPlayer { logical_entity },
    ));

    //debug HUD: shows look angles + raw mouse motion
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

#[derive(Component)]
struct DebugHud;

//shows the look angles and raw mouse counts so we can see what's actually happening
fn update_debug_hud(
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

//temporary debug aid for RDP testing: RDP discards mouse direction, so
//arrow keys drive the look directly. REMOVE BEFORE PUSHING.
fn arrow_key_look(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut input: Single<&mut FpsControllerInput>,
) {
    const SPEED: f32 = 1.5; //radians per second
    let dt = time.delta_secs();
    if keys.pressed(KeyCode::ArrowUp) {
        input.pitch += SPEED * dt;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        input.pitch -= SPEED * dt;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        input.yaw += SPEED * dt;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        input.yaw -= SPEED * dt;
    }
    input.pitch = input.pitch.clamp(-FRAC_PI_2 + 0.001, FRAC_PI_2 - 0.001);
}

//click to capture the mouse (and enable the controller), esc to release
fn manage_cursor(
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
