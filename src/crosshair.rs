use bevy::prelude::*;

pub struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_crosshair);
    }
}

fn spawn_crosshair(mut commands: Commands) {
    let color = Color::srgba(1.0, 1.0, 1.0, 0.85);

    commands.spawn((
        //full-screen, invisible container that centers its child
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            //16x16 box in the middle of the screen
            Node {
                width: px(16),
                height: px(16),
                ..default()
            },
            children![
                //horizontal line
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(7),
                        width: px(16),
                        height: px(2),
                        ..default()
                    },
                    BackgroundColor(color),
                ),
                //vertical line
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(7),
                        top: px(0),
                        width: px(2),
                        height: px(16),
                        ..default()
                    },
                    BackgroundColor(color),
                ),
            ],
        )],
    ));
}