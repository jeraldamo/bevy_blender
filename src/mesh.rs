use bevy_render::{
    mesh::{Indices, Mesh},
    render_resource::PrimitiveTopology,
};
use blend::runtime::Instance;

use crate::BevyBlenderError;

/// Takes a .blend file location and a mesh name and generates
/// an appropriate asset_loader string. For example,
/// blender_mesh!("demo.blend", "Suzanne") turns to "demo.blend#MESuzanne".
#[macro_export]
macro_rules! blender_mesh {
    ($blend_file:literal, $mesh_name:literal) => {
        format!("{}#ME{}", $blend_file, $mesh_name).as_str()
    };
}

/// Takes a Blend::Instance mesh and converts it to a Bevy mesh
pub(crate) fn instance_to_mesh(instance: Instance) -> anyhow::Result<Mesh> {
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
    for blender_face in blender_faces {
        let start = blender_face.get_i32("loopstart");
        let end = start + blender_face.get_i32("totloop");
        let mut faceloop: Vec<u32> = Vec::new();
        for i in start..end {
            faceloop.push(blender_loops[i as usize].get_i32("v") as u32);
        }

        let mut faces: Vec<Vec<u32>> = Vec::new();

        // triangulate ngons using ear clipping method
        let mut i = 0;
        while faceloop.len() > 3 {
            if i >= faceloop.len() {
                i = 0;
            }

            let mut face = Vec::new();
            face.push(faceloop[i]);

            i += 1;
            if i >= faceloop.len() {
                i = 0;
            }

            face.push(faceloop[i]);

            let mut j = i + 1;
            if j >= faceloop.len() {
                j = 0;
            }

            face.push(faceloop[j]);

            faces.push(face);

            faceloop.remove(i);
        }

        faces.push(faceloop);

        for face in faces {
            indices.push(face[2]);
            indices.push(face[0]);
            indices.push(face[1]);
            indices.push(face[2]);
            indices.push(face[0]);
            indices.push(face[1]);
        }
    }

    // Create vectors for mesh attributes
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; blender_verts.len()];

    // Fill position and normal attributes from blender_verts, swapping Y and Z
    for vert in blender_verts {
        let p = vert.get_f32_vec("co");
        positions.push([p[0], p[2], -p[1]]);

        let n = no_to_f32(vert.get_i16_vec("no"));
        normals.push([n[0], n[2], -n[1]]);
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
