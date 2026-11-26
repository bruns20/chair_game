use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_fps_controller::controller::*;
use bevy_rapier3d::prelude::*;

const SPAWN_POINT: Vec3 = Vec3::new(0.0, 1.625, 6.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<()>::default())
        .add_plugins(FpsControllerPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (manage_cursor, update_debug_hud))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {

    //blender map :)
    commands.spawn((
        WorldAssetRoot(asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("maps/first.glb"),
        )),
        AsyncSceneCollider::default(),
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
                sensitivity: 0.001,
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
