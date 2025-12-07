use bevy::prelude::*;
use bevy_blender::*;

use crate::camera::PanOrbitCamera;

// Use pan orbit camera
mod camera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BlenderPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, camera::pan_orbit_camera)
        .run();
}

fn setup(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    spawn_blender_object(
        &mut commands,
        &asset_server,
        "robot.blend",
        "Cube.002",
        false,
        None,
    );

    // Light and camera
    commands.spawn((
        PointLight::default(),
        Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
    ));

    let translation = Vec3::new(5.0, 5.0, 5.0);
    let radius = translation.length();

    commands.spawn((
        Camera3d::default(),
        PanOrbitCamera { radius, ..Default::default() },
        Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y)
    ));
}
