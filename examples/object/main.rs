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
    // Create and spawn a Blender object using a BlenderObjectBundle
    // This will only spawn the one object
    // This example is included for completeness, but it is recommended to use spawn_blender_object instead
    let mut suzanne = BlenderObjectBundle::new(&asset_server, "demo.blend", "Suzanne").unwrap();
    suzanne.transform = Transform::from_translation(Vec3::new(-4.0, 0.0, 0.0));
    commands.spawn(suzanne);

    // Spawn Blender object with children
    // The parent object's transform is taken from Blender
    spawn_blender_object(
        &mut commands,
        &asset_server,
        "demo.blend",
        "TransformCube",
        true,
        None,
    );

    // Spawn Blender object with children
    // The parent object's transform is provided
    spawn_blender_object(
        &mut commands,
        &asset_server,
        "demo.blend",
        "Suzanne",
        true,
        Some(Transform::from_matrix(
            Mat4::from_scale_rotation_translation(
                Vec3::new(0.5, 0.5, 0.5),
                Quat::IDENTITY,
                Vec3::new(4.0, 0.0, 0.0),
            ),
        )),
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
        Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y),
    ));

}
