use bevy::{color::Color, pbr::StandardMaterial};
use blend::runtime::Instance;

use crate::BevyBlenderError;

/// Takes a .blend file location and a material name and generates
/// an appropriate asset_loader string. For example,
/// blender_material!("demo.blend", "Material") turns to "demo.blend#MAMaterial".
#[macro_export]
macro_rules! blender_material {
    ($blend_file:literal, $material_name:literal) => {
        format!("{}#MA{}", $blend_file, $material_name)
    };
}

/// Takes a Blend::Instance material and converts it to a Bevy material. If the Blender material
/// is a basic material (not nodes based), the bevy_pbr::StandardMaterial will be used.
pub(crate) fn instance_to_material(
    instance: Instance,
    _blend_version: (u8, u8, u8),
) -> anyhow::Result<StandardMaterial> {
    // Don't process instances of types other than material
    if instance.type_name != "Material" {
        return Err(anyhow::Error::new(BevyBlenderError::InvalidInstanceType {
            expected: String::from("Material"),
            found: instance.type_name,
        }));
    }

    // If material.use_nodes == false we are going to use bevy_pbr::StandardMaterial as the
    // material type.
    if (instance.get_char("use_nodes") as u8) == 0 {
        return Ok(StandardMaterial {
            base_color: Color::linear_rgba(
                instance.get_f32("r"),
                instance.get_f32("g"),
                instance.get_f32("b"),
                instance.get_f32("a"),
            ),
            perceptual_roughness: instance.get_f32("roughness"),
            metallic: instance.get_f32("metallic"),
            reflectance: instance.get_f32("spec"),
            ..Default::default()
        });
    }

    // If we are here then the material is nodes based, which is not currently supported.
    Err(anyhow::Error::new(BevyBlenderError::UnsupportedAsset {
        asset_type: String::from("Nodes based material"),
    }))
}
