use crate::{right_hand_zup_to_right_hand_yup, BevyBlenderError};
use bevy::prelude::*;
use bevy_asset::{AssetServer, Handle};
use bevy_ecs::{
    bundle::Bundle,
    system::{Commands, ResMut},
};
use bevy_pbr::prelude::StandardMaterial;
use bevy_render::{
    mesh::Mesh,
    prelude::{ComputedVisibility, Visibility},
};
use bevy_transform::prelude::{GlobalTransform, Transform};
use blend::{Blend, Instance};

/// A component bundle for Blender Object entities
#[derive(Bundle)]
pub struct BlenderObjectBundle {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for BlenderObjectBundle {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

impl BlenderObjectBundle {
    /// Creates a new BlenderObjectBundle from a .blend file path and an object within it
    pub fn new(
        asset_server: &ResMut<AssetServer>,
        blender_file: &str,
        object_name: &str,
    ) -> anyhow::Result<Self> {
        let blend = Blend::from_path(
            std::env::current_dir()
                .unwrap()
                .join(std::path::PathBuf::from("assets").join(blender_file)),
        );

        Self::new_from_blend(asset_server, &blend, blender_file, object_name)
    }

    /// Creates a new BlenderObjectBundle from a Blend object
    pub fn new_from_blend(
        asset_server: &ResMut<AssetServer>,
        blend: &Blend,
        blender_file: &str,
        object_name: &str,
    ) -> anyhow::Result<Self> {
        let obj = get_object_by_name(blend, format!("OB{}", object_name).as_str())
            .expect(format!("Object {} does not exist in {}", object_name, blender_file).as_str());

        // Get the first material, if it is not a nodes based material
        // TODO: load all materials instead of just the first
        let mut materials = obj.get("data").get_iter("mat");
        let material: Handle<StandardMaterial> = match materials.next() {
            None => Handle::default(),
            Some(material) => {
                if (material.get_char("use_nodes") as u8) == 0 {
                    asset_server.load(
                        format!("{}#{}", blender_file, material.get("id").get_string("name"))
                            .as_str(),
                    )
                } else {
                    Handle::default()
                }
            }
        };

        // Get transform
        let world_matrix = get_world_matrix(&obj);
        let corrected_matrix = right_hand_zup_to_right_hand_yup(&world_matrix);
        let transform = Transform::from_matrix(corrected_matrix);

        return Ok(Self {
            mesh: asset_server.load(
                format!(
                    "{}#{}",
                    blender_file,
                    obj.get("data").get("id").get_string("name")
                )
                .as_str(),
            ),
            material,
            transform,
            ..Default::default()
        });
    }
}

fn get_object_by_name<'a>(blend: &'a Blend, name: &str) -> Option<Instance<'a>> {
    for obj in blend.get_by_code(*b"OB") {
        if obj.get("id").get_string("name") == name {
            return Some(obj);
        }
    }

    None
}

fn get_children<'a>(blend: &'a Blend, name: &str) -> Vec<Instance<'a>> {
    let mut children: Vec<Instance<'a>> = Vec::new();

    for obj in blend.get_by_code(*b"OB") {
        if obj.is_valid("parent") && obj.get("parent").get("id").get_string("name") == name {
            children.push(obj);
        }
    }

    children
}

fn get_world_matrix(object: &Instance) -> Mat4 {
    // world matrix comes in as a flattend row major 4x4 matrix
    let w = object.get_f32_vec("obmat");

    Mat4::from_cols(
        Vec4::from_slice(&w[0..4]),   // x axis
        Vec4::from_slice(&w[4..8]),   // y axis
        Vec4::from_slice(&w[8..12]),  // z axis
        Vec4::from_slice(&w[12..16]), // w axis
    )
}

/// Creates a BlenderObjectBundle for the object "root_object_name" and spaws it. This object will maintain its Blender transform and
/// will have its material applied (if it is not nodes-based).
/// If "spawn_children" is true, then all of the object's children will also be created as BlenderObjectBundles and spawed as children
/// Of the root object.
pub fn spawn_blender_object(
    commands: &mut Commands,
    asset_server: &ResMut<AssetServer>,
    blender_file: &str,
    root_object_name: &str,
    spawn_children: bool,
    parent_transform: Option<Transform>,
) {
    // Read blend file, we will pass this along to recurisive calls
    let blend = Blend::from_path(
        std::env::current_dir()
            .unwrap()
            .join(std::path::PathBuf::from("assets").join(blender_file.clone())),
    );

    // Get object
    let obj = get_object_by_name(&blend, format!("OB{}", root_object_name).as_str()).expect(
        format!(
            "Object {} does not exist in {}",
            root_object_name, blender_file
        )
        .as_str(),
    );

    // Get mesh
    let mesh = asset_server.load(
        format!(
            "{}#{}",
            blender_file,
            obj.get("data").get("id").get_string("name")
        )
        .as_str(),
    );

    // Get the first material, if it is not a nodes based material
    // TODO: load all materials instead of just the first
    let mut materials = obj.get("data").get_iter("mat");
    let material: Handle<StandardMaterial> = match materials.next() {
        None => Handle::default(),
        Some(material) => {
            if (material.get_char("use_nodes") as u8) == 0 {
                asset_server.load(
                    format!("{}#{}", blender_file, material.get("id").get_string("name")).as_str(),
                )
            } else {
                Handle::default()
            }
        }
    };

    // Get the object's transform
    let world_matrix = get_world_matrix(&obj);
    let transform = match parent_transform {
        Some(t) => t,
        None => {
            let corrected_matrix = right_hand_zup_to_right_hand_yup(&world_matrix);
            Transform::from_matrix(corrected_matrix)
        }
    };

    // Create BlenderObjectBundle
    let bundle = BlenderObjectBundle {
        mesh,
        material,
        transform,
        ..Default::default()
    };

    // Spawn Bundle
    commands.spawn_bundle(bundle).with_children(|parent| {
        if !spawn_children {
            return;
        }
        for child in get_children(&blend, format!("OB{}", root_object_name).as_str()) {
            spawn_children_objects(
                parent,
                asset_server,
                &blend,
                blender_file,
                child,
                world_matrix,
            );
        }
    });
}

fn spawn_children_objects(
    builder: &mut ChildBuilder,
    asset_server: &ResMut<AssetServer>,
    blend: &Blend,
    blender_file: &str,
    obj: Instance,
    parent_matrix: Mat4,
) {
    // Get mesh
    let mesh = asset_server.load(
        format!(
            "{}#{}",
            blender_file,
            obj.get("data").get("id").get_string("name")
        )
        .as_str(),
    );

    // Get the first material, if it is not a nodes based material
    // TODO: load all materials instead of just the first
    let mut materials = obj.get("data").get_iter("mat");
    let material: Handle<StandardMaterial> = match materials.next() {
        None => Handle::default(),
        Some(material) => {
            if (material.get_char("use_nodes") as u8) == 0 {
                asset_server.load(
                    format!("{}#{}", blender_file, material.get("id").get_string("name")).as_str(),
                )
            } else {
                Handle::default()
            }
        }
    };

    // Get the global transform matrix
    let world_matrix = get_world_matrix(&obj);
    // Calculate local matrix from global matrix and parent matrix
    // L = P' * W
    let local_matrix = parent_matrix.inverse().mul_mat4(&world_matrix);
    let corrected_local_matrix = right_hand_zup_to_right_hand_yup(&local_matrix);
    let transform = Transform::from_matrix(corrected_local_matrix);

    // Create BlenderObjectBundle
    let bundle = BlenderObjectBundle {
        mesh,
        material,
        transform,
        ..Default::default()
    };

    // Spawn Bundle
    builder.spawn_bundle(bundle).with_children(|parent| {
        for child in get_children(&blend, obj.get("id").get_string("name").as_str()) {
            spawn_children_objects(
                parent,
                asset_server,
                &blend,
                blender_file,
                child,
                world_matrix,
            );
        }
    });
}
