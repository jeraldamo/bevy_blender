use crate::{right_hand_zup_to_right_hand_yup, BevyBlenderError};
use bevy::{
    asset::{AssetServer, Handle},
    camera::visibility::Visibility,
    ecs::{bundle::Bundle, system::{Commands, ResMut}},
    log::error,
    math::{Mat4, Vec4},
    mesh::Mesh3d,
    pbr::{MeshMaterial3d, StandardMaterial},
    transform::components::{GlobalTransform, Transform}
};
use blend::{Blend, Instance};

/// A component bundle for Blender Object entities modeled after bevy_pbr::MaterialMeshBundle
#[derive(Bundle)]
pub struct BlenderObjectBundle {
    /// Standard mesh
    pub mesh: Mesh3d,
    /// Standard PBR material to be applied
    pub material: MeshMaterial3d<StandardMaterial>,
    /// User indication of whether an entity is visibible
    pub visibility: Visibility,
    /// Entity's transform relative to its parent's transform
    pub transform: Transform,
    /// Entity's transform relative to the world origin
    pub global_transform: GlobalTransform,
}

impl Default for BlenderObjectBundle {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            visibility: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

impl BlenderObjectBundle {
    /// Creates a new BlenderObjectBundle from a .blend file path and an object within it
    /// It will automatically apply the Blender object's transform and material if applicable
    pub fn new(
        asset_server: &ResMut<AssetServer>,
        blender_file: &str,
        object_name: &str,
    ) -> anyhow::Result<Self> {
        let blend = Blend::from_path(
            std::env::current_dir()
                .unwrap()
                .join(std::path::PathBuf::from("assets").join(blender_file)),
        ).expect("loading failed");

        Self::new_from_blend(asset_server, &blend, blender_file, object_name)
    }

    /// Creates a new BlenderObjectBundle from a Blend object
    /// It will automatically apply the Blender object's transform and material if applicable
    pub fn new_from_blend(
        asset_server: &ResMut<AssetServer>,
        blend: &Blend,
        blender_file: &str,
        object_name: &str,
    ) -> anyhow::Result<Self> {
        let obj = match get_object_by_name(blend, format!("OB{}", object_name).as_str()) {
            Some(o) => o,
            None => {
                return Err(anyhow::Error::new(BevyBlenderError::MissingAsset {
                    asset_name: object_name.into(),
                    blend_file: blender_file.into(),
                }));
            }
        };

        // Get the first material, if it is not a nodes based material
        // TODO: load all materials instead of just the first
        let mut materials = obj.get("data").get_iter("mat");
        let material: Handle<StandardMaterial> = match materials.next() {
            None => Handle::default(),
            Some(material) => {
                if (material.get_char("use_nodes") as u8) == 0 {
                    asset_server.load(
                        format!("{}#{}", blender_file, material.get("id").get_string("name"))
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
            mesh: Mesh3d(asset_server.load(
                format!(
                    "{}#{}",
                    blender_file,
                    obj.get("data").get("id").get_string("name")
                ),
            )),
            material: MeshMaterial3d(material),
            transform,
            ..Default::default()
        });
    }
}

/// Iterates over the objects in the blend file and returns Some(Instance) if object
/// is present and None otherwise
fn get_object_by_name<'a>(blend: &'a Blend, name: &str) -> Option<Instance<'a>> {
    for obj in blend.instances_with_code(*b"OB") {
        if obj.get("id").get_string("name") == name {
            return Some(obj);
        }
    }

    None
}

/// Returns a list of all of the children belonging the object in the blend file with the name "name"
fn get_children<'a>(blend: &'a Blend, name: &str) -> Vec<Instance<'a>> {
    let mut children: Vec<Instance<'a>> = Vec::new();

    for obj in blend.instances_with_code(*b"OB") {
        if obj.is_valid("parent") && obj.get("parent").get("id").get_string("name") == name {
            children.push(obj);
        }
    }

    children
}

/// Get the world relative 4x4 matrix of an object
/// This will be in Blender coordinate system (Right Handed, Z-up)
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

/// Creates a BlenderObjectBundle for the object "root_object_name" and spaws it. This object
/// will maintain its Blender transform and will have its material applied (if it is not nodes-based).
/// If "spawn_children" is true, then all of the object's children will also be created as
/// BlenderObjectBundles and spawed as children of the root object. If parent_transform is Some(t),
/// then t will be used as it's transform. If parent_transform is None, then the object's Blender transform
/// will be used (and converted to the Bevy coordinate system).
pub fn spawn_blender_object(
    commands: &mut Commands,
    asset_server: &ResMut<AssetServer>,
    blender_file: &str,
    root_object_name: &str,
    spawn_children: bool,
    parent_transform: Option<Transform>,
) {
    match spawn_blender_object_with_error(
        commands,
        asset_server,
        blender_file,
        root_object_name,
        spawn_children,
        parent_transform,
    ) {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
        }
    }
}

fn spawn_blender_object_with_error(
    commands: &mut Commands,
    asset_server: &ResMut<AssetServer>,
    blender_file: &str,
    root_object_name: &str,
    spawn_children: bool,
    parent_transform: Option<Transform>,
) -> anyhow::Result<()> {
    // Read blend file, we will pass this along to recurisive calls
    let blend = Blend::from_path(
        std::env::current_dir()
            .unwrap()
            .join(std::path::PathBuf::from("assets").join(blender_file)),
    ).expect("loading failed");

    // Get object
    let obj = match get_object_by_name(&blend, format!("OB{}", root_object_name).as_str()) {
        Some(o) => o,
        None => {
            return Err(anyhow::Error::new(BevyBlenderError::MissingAsset {
                asset_name: root_object_name.into(),
                blend_file: blender_file.into(),
            }));
        }
    };

    // Get mesh
    let mesh = asset_server.load(
        format!(
            "{}#{}",
            blender_file,
            obj.get("data").get("id").get_string("name")
        ),
    );

    // Get the first material, if it is not a nodes based material
    // TODO: load all materials instead of just the first
    let material: Handle<StandardMaterial> = if obj.get("data").is_valid("mat") {
        let mut materials = obj.get("data").get_iter("mat");
        match materials.next() {
            None => asset_server
                .load(format!("{}#{}", blender_file, "bevy_blender_missing_material")),
            Some(material) => asset_server.load(
                format!("{}#{}", blender_file, material.get("id").get_string("name")),
            ),
        }
    } else {
        asset_server.load(format!("{}#{}", blender_file, "bevy_blender_missing_material"))
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
        mesh: Mesh3d(mesh),
        material: MeshMaterial3d(material),
        transform,
        ..Default::default()
    };

    // Spawn Bundle
    commands.spawn(bundle).with_children(|parent| {
        if !spawn_children {
            return;
        }
        for child in get_children(&blend, format!("OB{}", root_object_name).as_str()) {
            spawn_children_objects(
                parent.commands_mut(),
                asset_server,
                &blend,
                blender_file,
                child,
                world_matrix,
            );
        }
    });

    Ok(())
}

/// Helper recursive function called by spawn_blender_object to spawn children
fn spawn_children_objects(
    builder: &mut Commands,
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
    );

    // Get the first material, if it is not a nodes based material
    // TODO: load all materials instead of just the first
    let mut materials = obj.get("data").get_iter("mat");
    let material: Handle<StandardMaterial> = match materials.next() {
        None => asset_server
            .load(format!("{}#{}", blender_file, "bevy_blender_missing_material")),
        Some(material) => asset_server
            .load(format!("{}#{}", blender_file, material.get("id").get_string("name"))),
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
        mesh: Mesh3d(mesh),
        material: MeshMaterial3d(material),
        transform,
        ..Default::default()
    };

    // Spawn Bundle
    builder.spawn(bundle).with_children(|parent| {
        for child in get_children(&blend, obj.get("id").get_string("name").as_str()) {
            spawn_children_objects(
                parent.commands_mut(),
                asset_server,
                &blend,
                blender_file,
                child,
                world_matrix,
            );
        }
    });
}
