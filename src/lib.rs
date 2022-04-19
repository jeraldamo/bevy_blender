#![warn(missing_docs)]
//! # bevy_blender
//!
//! [bevy_blender](https://github.com/jeraldamo/bevy_blender) is a [Bevy](https://bevyengine.org) library that allows you to use assets created in [Blender](https://blender.org) directly from the .blend file.
//!
//! ### Usage
//! 1) Add `bevy_blender` to your `Cargo.toml` dependencies.
//! 1) Add `bevy_blender::BlenderPlugin` plugin to the bevy `App`
//! 1) Load Blender assets (see examples)
//!
//! ### Supported Assets
//! * Meshes (using `AssetServer`)
//! * Basic. not node-based, materials (using `AssetServer`)
//! * Objects (using `BlenderObjectBundle`)
//!
//! *If the asset name in Blender starts with an underscore, it will not be loaded. You can use this to have extra assets in the .blend file that you do not want loaded to the AssetServer.*
//!
//! #### Example
//! ```rust
//! fn main() {
//!     App::build()
//!         .add_plugin(bevy_blender::BlenderPlugin)
//!         .add_startup_system(setup.system())
//!         // ...
//!         .run();
//! }
//!
//! fn setup(commands: &mut Commands, asset_server: Res<AssetServer>) {
//!     
//!     // Spawn the Suzanne Blender object with children and its Blender transform
//!     spawn_blender_object(&mut commands, &asset_server, "demo.blend", "Suzanne", true, None);
//!        .expect("Error spawning Blender object");
//!
//!     // Spawn the Suzanne mesh with the Red material
//!     commands.spawn_bundle(PbrBundle {
//!             mesh: asset_server.load(blender_mesh!("demo.blend", "Suzanne")),
//!             material: asset_server.load(blender_material!("demo.blend", "Red")),
//!             ..Default::default()
//!         })
//!         // ...
//! }
//! ```
//!
//! A suite of examples can be found in `examples/`. Currently, there are three examples, one that shows how to import just a mesh, one that shows how to import just a material, and one that shows how to import whole objects. Simply run `cargo run --example=object` (or `example=mesh`, or `example=material`) to execute it. This will open a .blend file located at `assets/demo.blend`.

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, AssetLoader, LoadContext, LoadedAsset};
use bevy_math::{Mat4, Quat, Vec3};
use bevy_utils::BoxedFuture;
use blend::Blend;

mod material;
mod mesh;
mod object;

pub use object::{spawn_blender_object, BlenderObjectBundle};

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
    #[error("Invalid .blend file: The file {blend_file:?} does not appear to be a valid Blender file. Please make sure it is not compressed.")]
    InvalidBlendFile {
        /// The name of the .blend file
        blend_file: String,
    },

    /// The library was trying to process a Blend::Instance of one type but got another. Probably an issue with the .blend file.
    #[error("Invalid instance type: Expected {expected:?}, got {found:?}.")]
    InvalidInstanceType {
        /// The type that was expected
        expected: String,
        /// The type that was found
        found: String,
    },

    /// The library tried to load a Blender asset that is not yet supported.
    #[error("Unsupported asset: The asset type {asset_type:?} is not currently supported.")]
    UnsupportedAsset {
        /// The type of asset trying to be loaded
        asset_type: String,
    },

    /// The library tried to access a Blender asset that was not there
    #[error("Missing asset: The asset {asset_name:?} could not be found in {blend_file:?}. Please make sure the asset name does not start with an underscore.")]
    MissingAsset {
        /// The asset name
        asset_name: String,
        /// The blender file
        blend_file: String,
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
    if bytes[0..7] != *b"BLENDER" {
        return Err(anyhow::Error::new(BevyBlenderError::InvalidBlendFile {
            blend_file: String::from(load_context.path().to_str().unwrap()),
        }));
    }

    let blend_version = (bytes[9] - 48, bytes[10] - 48, bytes[11] - 48);

    // TODO: check for compressed blend file and decompress if necessary
    let blend = Blend::new(bytes);

    // Load mesh assets
    for mesh in blend.get_by_code(*b"ME") {
        // Get the name of the mesh and remove the prepending "ME"
        let label = mesh.get("id").get_string("name");

        // Skip any mesh whose name starts with underscore
        if !label.starts_with("ME_") {
            // Add the created mesh with the proper label
            load_context.set_labeled_asset(
                label.as_str(),
                LoadedAsset::new(mesh::instance_to_mesh(mesh, blend_version)?),
            );
        }
    }

    // Load material assets
    for material in blend.get_by_code(*b"MA") {
        // Get the name of the material
        let label = material.get("id").get_string("name");

        // Skip any material whose name starts with underscore
        if !label.starts_with("MA_") {
            // Add the created material with the proper label
            load_context.set_labeled_asset(
                label.as_str(),
                LoadedAsset::new(material::instance_to_material(material, blend_version)?),
            );
        }
    }

    // TODO: load other kinds of assets

    Ok(())
}

/// Takes a right handed, z up transformation matrix (Blender) and returns a right handed, y up (Bevy) version of it
pub fn right_hand_zup_to_right_hand_yup(rhzup: &Mat4) -> Mat4 {
    let (scale, rotation, translation) = rhzup.to_scale_rotation_translation();
    let euler_rotation = rotation.to_euler(bevy_math::EulerRot::XYZ);

    Mat4::from_scale_rotation_translation(
        Vec3::new(scale[0], scale[2], scale[1]),
        Quat::from_euler(
            bevy_math::EulerRot::XZY,
            euler_rotation.0,
            -euler_rotation.1,
            euler_rotation.2,
        ),
        Vec3::new(translation[0], translation[2], -translation[1]),
    )
}
