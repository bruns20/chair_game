mod camera;
mod debug_hud;
mod player;
mod world;

use bevy::prelude::*;
use bevy_fps_controller::controller::*;
use bevy_rapier3d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<()>::default())
        .add_plugins(FpsControllerPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (camera::manage_cursor, debug_hud::update_debug_hud))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    world::spawn_world(&mut commands, asset_server);
    player::spawn_player(&mut commands);
    debug_hud::spawn_debug_hud(&mut commands);
}
