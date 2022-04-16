#![warn(missing_docs)]
//! # bevy_blender
//!
//! [bevy_blender](https://github.com/jeraldamo/bevy_blender) is a [Bevy](https://bevyengine.org) library that allows you to use assets created in [Blender](https://blender.org) directly from the .blend file.
//!
//! # Usage
//! 1) Add `bevy_blender` to your `Cargo.toml` dependencies.
//! 1) Add `bevy_blender::BlenderPlugin` plugin to the bevy `App`
//! 1) Load Blender mesh assets by using the included macro with `asset_server.load`. For example: `asset_server.load(blender_mesh!("blend_file.blend", "mesh_name"))`
//!
//! *If the asset name in Blender starts with an underscore, it will not be loaded. You can use this to have extra assets in the .blend file that you do not want loaded to the AssetServer.*
//!
//! # Example
//! ```
//! fn main() {
//!     App::build()
//!         .add_plugin(bevy_blender::BlenderPlugin)
//!         .add_startup_system(setup.system())
//!         // ...
//!         .run();
//! }
//!
//! fn setup(commands: &mut Commands, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<StandardMaterial>>) {
//!     commands.spawn_bundle(PbrBundle {
//!             mesh: asset_server.load(blender_mesh!("demo.blend", "Suzanne")),
//!             material: materials.add(Color::rgb(0.9, 0.4, 0.3).into()),
//!             ..Default::default()
//!         })
//!         // ...
//! }
//! ```
//!
//! A full example can be found in `examples/demo.rs`. Simply run `cargo run --example demo` to execute it. This will open a .blend file located at `assets/demo.blend`.

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, AssetLoader, LoadContext, LoadedAsset};
use bevy_render::{
    mesh::{Indices, Mesh},
    render_resource::PrimitiveTopology,
};
use bevy_utils::BoxedFuture;
use blend::{runtime::Instance, Blend};

/// Takes a .blend file location and a mesh name and generates
/// an appropriate asset_loader string. For example,
/// blender_mesh!("demo.blend", "Suzanne") turns to "demo.blend#MESuzanne".
#[macro_export]
macro_rules! blender_mesh {
    ($blend_file:literal, $mesh_name:literal) => {
        format!("{}#ME{}", $blend_file, $mesh_name).as_str()
    };
}

/// Plugin for Bevy that allows for interaction with .blend files
pub struct BlenderPlugin;

impl Plugin for BlenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<BlenderLoader>();
    }
}

/// bevy_blender errors
#[derive(thiserror::Error, Debug)]
pub enum BevyBlenderError {
    /// The library tried to parse the .blend file, but could not find the magic number. Probably a corrupted or compressed .blend file.
    #[error("Invalid .blend file {file_name:?}, missing magic number")]
    InvalidBlendFile {
        /// The name of the .blend file
        file_name: String,
    },

    /// The library was trying to process a Blend::Instance of one type but got another. Probably an issue with the .blend file.
    #[error("Invalid instance type (expected {expected:?}, got {found:?})")]
    InvalidInstanceType {
        /// The type that was expected
        expected: String,
        /// The type that was found
        found: String,
    },
}

#[derive(Default)]
struct BlenderLoader;

impl AssetLoader for BlenderLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<()>> {
        Box::pin(async move { Ok(load_blend_assets(bytes, load_context).await?) })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["blend"];
        EXTENSIONS
    }
}

async fn load_blend_assets<'a, 'b>(
    bytes: &'a [u8],
    load_context: &'a mut LoadContext<'b>,
) -> anyhow::Result<()> {
    // Check to make sure that the blender file has the magic number
    //                  B     L     E     N     D     E     R
    if bytes[0..7] != [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52] {
        return Err(anyhow::Error::new(BevyBlenderError::InvalidBlendFile {
            file_name: String::from(load_context.path().to_str().unwrap()),
        }));
    }

    // TODO: check for compressed blend file and decompress if necessary
    let blend = Blend::new(bytes);

    // Load mesh assets
    for mesh in blend.get_by_code(*b"ME") {
        // Get the name of the mesh and remove the prepending "ME"
        let label = mesh.get("id").get_string("name");

        // Skip any mesh whose name starts with underscore
        if !label.starts_with("ME_") {
            // Add the created mesh with the proper label
            load_context
                .set_labeled_asset(label.as_str(), LoadedAsset::new(instance_to_mesh(mesh)?));
        }
    }

    // TODO: load other kinds of assets

    Ok(())
}

/// Takes a Blend::Instance mesh and converts it to a Bevy mesh
fn instance_to_mesh(instance: Instance) -> anyhow::Result<Mesh> {
    // Don't process instances of types other than mesh
    if instance.type_name != "Mesh" {
        return Err(anyhow::Error::new(BevyBlenderError::InvalidInstanceType {
            expected: String::from("Mesh"),
            found: instance.type_name,
        }));
    }

    // Takes a normalized i16 vector from instance, and converts it to a normalized f32 vector
    fn no_to_f32(no: Vec<i16>) -> Vec<f32> {
        let mut v = Vec::new();
        for i in no {
            v.push((i as f32) / (i16::MAX as f32));
        }
        v
    }

    // Extract Blender DNA blocks from instance
    let blender_faces = instance.get_iter("mpoly").collect::<Vec<_>>();
    let blender_loops = instance.get_iter("mloop").collect::<Vec<_>>();
    let blender_uvs = instance.get_iter("mloopuv").collect::<Vec<_>>();
    let blender_verts = instance.get_iter("mvert").collect::<Vec<_>>();

    // Create empty index list
    let mut indices: Vec<u32> = Vec::new();

    // Loop over blender faces and appropriately fill indices
    for face in blender_faces {
        let start = face.get_i32("loopstart");
        let end = start + face.get_i32("totloop");
        let mut faceloop: Vec<u32> = Vec::new();
        for i in start..end {
            faceloop.push(blender_loops[i as usize].get_i32("v") as u32);
        }

        // TODO: Implement triangulation algorithm to handle ngon face
        match faceloop.len() {
            3 => {
                indices.push(faceloop[1]);
                indices.push(faceloop[0]);
                indices.push(faceloop[2]);
                indices.push(faceloop[1]);
                indices.push(faceloop[0]);
                indices.push(faceloop[2]);
            }
            4 => {
                indices.push(faceloop[1]);
                indices.push(faceloop[0]);
                indices.push(faceloop[2]);
                indices.push(faceloop[1]);
                indices.push(faceloop[0]);
                indices.push(faceloop[2]);

                indices.push(faceloop[2]);
                indices.push(faceloop[0]);
                indices.push(faceloop[3]);
                indices.push(faceloop[2]);
                indices.push(faceloop[0]);
                indices.push(faceloop[3]);
            }
            _ => {
                eprintln!("Warning: bevy_blend trying to create {}-gon on mesh {}. This is not currently supported.", faceloop.len(), instance.get("id").get_string("name"));
            }
        }
    }

    // Create vectors for mesh attributes
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; blender_verts.len()];

    // Fill position and normal attributes from blender_verts, swapping Y and Z
    for vert in blender_verts {
        let p = vert.get_f32_vec("co");
        positions.push([p[0], p[2], p[1]]);

        let n = no_to_f32(vert.get_i16_vec("no"));
        normals.push([n[0], n[2], n[1]]);
    }

    // Get UVs from loops indices
    for i in 0..blender_uvs.len() {
        let uv = blender_uvs[i].get_f32_vec("uv");
        uvs[blender_loops[i].get_i32("v") as usize] = [uv[0], uv[1]];
    }

    // Create Bevy mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    // Return Bevy mesh
    Ok(mesh)
}
