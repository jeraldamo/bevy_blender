use bevy_blender::*;
use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(BlenderPlugin)
        .add_startup_system(setup.system())
        // ...
        .run();
}

fn setup(mut commands: Commands, asset_server: ResMut<AssetServer>, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands
        .spawn_bundle(PbrBundle {
            mesh: asset_server.load("/home/jerald/Cube.blend#Cube"),
            material: materials.add(Color::rgb(0.9, 0.4, 0.3).into()),
            ..Default::default()
        });
        // ...
}
