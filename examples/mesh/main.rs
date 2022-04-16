use bevy::prelude::*;
use bevy_blender::*;

// Use pan orbit camera
mod camera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(BlenderPlugin)
        .add_startup_system(setup)
        .add_system(camera::pan_orbit_camera)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn_bundle(PbrBundle {
        mesh: asset_server.load(blender_mesh!("demo.blend", "Cylinder")),
        material: materials.add(Color::rgb(0.9, 0.4, 0.3).into()),
        transform: Transform::from_translation(Vec3::new(-4.0, 0.0, 0.0)),
        ..Default::default()
    });

    commands.spawn_bundle(PbrBundle {
        mesh: asset_server.load(blender_mesh!("demo.blend", "Cube")),
        material: materials.add(Color::rgb(0.9, 0.4, 0.3).into()),
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ..Default::default()
    });

    commands.spawn_bundle(PbrBundle {
        mesh: asset_server.load(blender_mesh!("demo.blend", "Suzanne")),
        material: materials.add(Color::rgb(0.9, 0.4, 0.3).into()),
        transform: Transform::from_translation(Vec3::new(4.0, 0.0, 0.0)),
        ..Default::default()
    });

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });

    let translation = Vec3::new(5.0, 5.0, 5.0);
    let radius = translation.length();

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert(camera::PanOrbitCamera {
            radius,
            ..Default::default()
        });
}
