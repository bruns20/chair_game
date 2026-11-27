use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_fps_controller::controller::*;
use bevy_rapier3d::prelude::*;

const SPAWN_POINT: Vec3 = Vec3::new(0.0, 1.625, 6.0);

//player: two entities.
//the "logical" player is the invisible physics body the controller drives,
//the "render" player is the camera that follows it.
pub fn spawn_player(commands: &mut Commands) {
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
        //ambient fill so the shadowed side of the map isn't black
        AmbientLight {
            color: Color::srgb(0.5, 0.55, 0.65),
            brightness: 30.0,
            affects_lightmapped_meshes: true,
        },
        RenderPlayer { logical_entity },
    ));
}
