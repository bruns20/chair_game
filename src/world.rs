use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

//blender map :)
pub fn spawn_world(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        WorldAssetRoot(asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("maps/first.glb"),
        )),
        AsyncSceneCollider::default(),
    ));
}
