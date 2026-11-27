use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use bevy_fps_controller::controller::{LogicalPlayer, RenderPlayer};
use bevy_rapier3d::prelude::*;

/// How far to the right of the view the shot comes out, in world units.
const GUN_OFFSET: f32 = 0.5;
/// How far down the aim ray the shot converges on the cursor, in world units.
const CONVERGENCE: f32 = 10.0;
/// How far a shot travels before giving up, in world units.
const MAX_TOI: f32 = 100.0;
/// How long a tracer stays visible, in seconds.
const TRACER_LIFETIME: f32 = 0.08;
/// Where the tracer starts, in front of the camera.
const TRACER_START: f32 = 0.5;
/// The tracer's radius, in world units.
const TRACER_RADIUS: f32 = 0.05;

/// A tracer that despawns once its lifetime runs out.
#[derive(Component)]
pub struct Lifetime(f32);

//left-click fires while the cursor is locked. This must run before manage_cursor
//(see main.rs), so the click that locks the cursor still sees the old state and
//is consumed: the first shot only goes off once the cursor is already locked.
pub fn fire_tracer(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Single<&CursorOptions>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<RenderPlayer>>,
    player: Single<Entity, With<LogicalPlayer>>,
    rapier_context: ReadRapierContext,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if cursor.grab_mode != CursorGrabMode::Locked || !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    //the cursor is locked, so the aim is the middle of the screen
    let Ok(aim) = camera.0.viewport_to_world(camera.1, window.size() / 2.0) else {
        return;
    };
    //the shot comes out of the gun on the right side of the screen and
    //converges on whatever the cursor points at
    let right = camera.1.rotation() * Vec3::X;
    let origin = Vec3::from(aim.origin) + right * GUN_OFFSET;
    let aim_point: Vec3 = (aim.origin + aim.direction * CONVERGENCE).into();
    let dir = (aim_point - origin).normalize();

    //exclude the player's own collider, or every shot hits the body at range zero
    let filter = QueryFilter::new().exclude_rigid_body(*player);
    let Ok(context) = rapier_context.single() else {
        return;
    };
        let toi = context
        .cast_ray(origin, dir, MAX_TOI, true, filter)   
        .map(|(_, toi)| toi)
        .unwrap_or(MAX_TOI);

    let start = origin + dir * TRACER_START;            
    let end = origin + dir * toi;                      
    let length = (end - start).length();
    if length < f32::EPSILON {
        return;
    }

    let mesh = meshes.add(Cylinder {
        radius: TRACER_RADIUS,
        half_height: length / 2.0,
    });
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.0),
        emissive: Color::srgb(1.0, 0.85, 0.0).to_linear(),
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform {
            translation: (start + end) / 2.0,
            //the cylinder's axis is Y; point it along the shot
            rotation: Quat::from_rotation_arc(Vec3::Y, dir),
            ..default()
        },
        Lifetime(TRACER_LIFETIME),
    ));
}

//despawn tracers once their lifetime runs out
pub fn expire_tracers(
    time: Res<Time>,
    mut tracers: Query<(Entity, &mut Lifetime)>,
    mut commands: Commands,
) {
    for (entity, mut lifetime) in &mut tracers {
        lifetime.0 -= time.delta_secs();
        if lifetime.0 <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{
        input::{
            mouse::MouseButtonInput,
            ButtonState, InputPlugin,
        },
        time::TimeUpdateStrategy,
        transform::TransformPlugin,
        window::{PrimaryWindow, WindowResolution},
    };
    use bevy::camera::RenderTargetInfo;

    use super::*;

    const SIZE: (u32, u32) = (1280, 720);

    //camera at the origin looking down -Z; the player's collider sits around the
    //camera, and a wall stands 5 m ahead.
    fn test_app() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            InputPlugin,
            AssetPlugin::default(),
            RapierPhysicsPlugin::<()>::default(),
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_systems(Update, (fire_tracer, expire_tracers));

        let world = app.world_mut();
        let window = world.spawn((
            Window {
                resolution: WindowResolution::new(SIZE.0, SIZE.1),
                ..default()
            },
            PrimaryWindow,
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ))
        .id();
        let player = world.spawn((
            LogicalPlayer,
            RigidBody::Dynamic,
            GravityScale(0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            Transform::from_xyz(0.0, 0.0, -0.5),
        ))
        .id();
        let camera = world.spawn((
            Camera3d::default(),
            RenderPlayer { logical_entity: player },
        ))
        .id();
        world.spawn((
            RigidBody::Fixed,
            Collider::cuboid(10.0, 10.0, 0.5),
            Transform::from_xyz(0.0, 0.0, -5.0),
        ));

        //MinimalPlugins has no render pipeline, so give the camera its matrices by hand
        let mut cam = world.entity_mut(camera);
        let mut camera_comp = cam.get_mut::<Camera>().unwrap();
        camera_comp.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(SIZE.0, SIZE.1),
            scale_factor: 1.0,
        });
        let mut projection = Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::PI / 3.6,
            ..default()
        });
        projection.update(SIZE.0 as f32, SIZE.1 as f32);
        camera_comp.computed.clip_from_view = projection.get_clip_from_view();

        (app, window, camera)
    }

    //two warm-up frames so physics builds the broadphase
    fn warm_up(app: &mut App) {
        app.update();
        app.update();
    }

    fn click(app: &mut App, window: Entity) {
        let button = |state| MouseButtonInput {
            button: MouseButton::Left,
            state,
            window,
        };
        app.world_mut()
            .resource_mut::<Messages<MouseButtonInput>>()
            .write(button(ButtonState::Pressed));
        app.update();
        app.world_mut()
            .resource_mut::<Messages<MouseButtonInput>>()
            .write(button(ButtonState::Released));
        app.update();
    }

    fn tracers(world: &mut World) -> Vec<Entity> {
        world
            .query_filtered::<Entity, With<Lifetime>>()
            .iter(world)
            .collect()
    }

    #[test]
    fn fires_at_the_wall_and_not_the_player() {
        let (mut app, window, _camera) = test_app();
        warm_up(&mut app);
        click(&mut app, window);

        let mut world = app.world_mut();
        let tracers = tracers(&mut world);
        assert_eq!(tracers.len(), 1);
        let transform = world.entity(tracers[0]).get::<Transform>().unwrap();
        //the ray starts on the near plane (-0.1 m) and stops at the wall's front
        //face at -4.5 m; the tracer runs from -0.6 to -4.5, midpoint -2.55.
        //If the player weren't excluded, the ray would hit the player's collider
        //at range zero (the camera sits inside it) and the midpoint would be -0.35.
        assert!((transform.translation.z - -2.55).abs() < 1e-3, "{transform:?}");
        //the cylinder points along the shot
        let axis = transform.rotation * Vec3::Y;
        assert!(axis.abs_diff_eq(Vec3::new(0.0, 0.0, -1.0), 1e-3), "{axis:?}");
    }

    #[test]
    fn does_not_fire_while_unlocked() {
        let (mut app, window, _camera) = test_app();
        let world = app.world_mut();
        let mut entity = world.entity_mut(window);
        let mut cursor = entity.get_mut::<CursorOptions>().unwrap();
        cursor.grab_mode = CursorGrabMode::None;
        warm_up(&mut app);
        click(&mut app, window);

        assert!(tracers(&mut app.world_mut()).is_empty());
    }

    #[test]
    fn a_miss_flies_to_max_toi() {
        let (mut app, window, camera) = test_app();
        //look away from the wall, down +X
        app.world_mut()
            .entity_mut(camera)
            .insert(Transform::from_rotation(Quat::from_rotation_y(-std::f32::consts::PI / 2.0)));
        warm_up(&mut app);
        click(&mut app, window);

        let mut world = app.world_mut();
        let tracers = tracers(&mut world);
        assert_eq!(tracers.len(), 1);
        let transform = world.entity(tracers[0]).get::<Transform>().unwrap();
        //the ray starts on the near plane (0.1 m ahead) and flies to MAX_TOI
        assert!((transform.translation.x - (0.1 + (TRACER_START + MAX_TOI) / 2.0)).abs() < 1e-3,
            "{transform:?}");
    }

    #[test]
    fn tracer_despawns_after_its_lifetime() {
        let (mut app, window, _camera) = test_app();
        warm_up(&mut app);
        click(&mut app, window);
        assert_eq!(tracers(&mut app.world_mut()).len(), 1);

        //80 ms is 4.8 frames at 60 fps
        for _ in 0..6 {
            app.update();
        }
        assert!(tracers(&mut app.world_mut()).is_empty());
    }
}
