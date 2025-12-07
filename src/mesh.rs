use bevy::{asset::RenderAssetUsages, math::Vec3, mesh::{Indices, Mesh, PrimitiveTopology}};
use blend::runtime::Instance;

use crate::BevyBlenderError;

/// Takes a .blend file location and a mesh name and generates
/// an appropriate asset_loader string. For example,
/// blender_mesh!("demo.blend", "Suzanne") turns to "demo.blend#MESuzanne".
#[macro_export]
macro_rules! blender_mesh {
    ($blend_file:literal, $mesh_name:literal) => {
        format!("{}#ME{}", $blend_file, $mesh_name)
    };
}

/// Takes a Blend::Instance mesh and converts it to a Bevy mesh
pub(crate) fn instance_to_mesh(
    instance: Instance,
    blend_version: (u8, u8, u8),
) -> Result<Mesh, BevyBlenderError> {
    // Don't process instances of types other than mesh
    if instance.type_name != "Mesh" {
        return Err(BevyBlenderError::InvalidInstanceType {
            expected: String::from("Mesh"),
            found: instance.type_name,
        });
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
    for blender_face in &blender_faces {
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
    for vert in &blender_verts {
        let p = vert.get_f32_vec("co");
        positions.push([p[0], p[2], -p[1]]);
    }

    match blend_version {
        (0..=2, _, _) => {
            for vert in &blender_verts {
                let n = no_to_f32(vert.get_i16_vec("no"));
                normals.push([n[0], n[2], -n[1]]);
            }
        }
        (3.., _, _) => {
            normals = calculate_vertex_normals(&blender_faces, &blender_loops, &positions);
        }
    }

    // Get UVs from loops indices
    for i in 0..blender_uvs.len() {
        let uv = blender_uvs[i].get_f32_vec("uv");
        uvs[blender_loops[i].get_i32("v") as usize] = [uv[0], uv[1]];
    }

    // Create Bevy mesh
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    // Return Bevy mesh
    Ok(mesh)
}

// Blender version 3+ does not include precalculated vertex normals in the .blend file
fn calculate_vertex_normals(
    blender_faces: &Vec<Instance>,
    blender_loops: &Vec<Instance>,
    positions: &Vec<[f32; 3]>,
) -> Vec<[f32; 3]> {
    let mut normals: Vec<Vec3> = Vec::new();

    for blender_face in blender_faces {
        let start = blender_face.get_i32("loopstart");
        let end = start + blender_face.get_i32("totloop");
        let mut faceloop: Vec<u32> = Vec::new();
        for i in start..end {
            faceloop.push(blender_loops[i as usize].get_i32("v") as u32);
        }
        // Calculate face normal as cross product of first and last edge in loop
        let v1 = positions[faceloop[0] as usize];
        let v2 = positions[faceloop[1] as usize];
        let v3 = positions[faceloop[faceloop.len() - 1] as usize];

        let e1 = Vec3::new(v2[0] - v1[0], v2[1] - v1[1], v2[2] - v1[2]);
        let e2 = Vec3::new(v1[0] - v3[0], v1[1] - v3[1], v1[2] - v3[2]);

        let n = e2.cross(e1).normalize();

        for vertex in faceloop {
            // Make sure vertex index exists in normals
            while vertex >= normals.len() as u32 {
                normals.push(Vec3::new(0.0, 0.0, 0.0));
            }
            // Add face normal to vertex index
            normals[vertex as usize] += n;
        }
    }

    let mut normals_out: Vec<[f32; 3]> = Vec::new();
    // Normalize vertex normals and cast to proper type
    for normal in normals {
        normals_out.push(normal.normalize().into());
    }

    return normals_out;
}

#[cfg(nightly)]
mod tests {
    use super::instance_to_mesh;
    use blend::{Blend, Instance};

    extern crate test;
    use test::Bencher;

    fn get_mesh_by_name<'a>(blend: &'a Blend, name: &str) -> Option<Instance<'a>> {
        for mesh in blend.get_by_code(*b"ME") {
            if mesh.get("id").get_string("name") == name {
                return Some(mesh);
            }
        }

        None
    }

    #[bench]
    fn instance_to_mesh_cube_192(b: &mut Bencher) {
        let blend = Blend::from_path(
            std::env::current_dir()
                .unwrap()
                .join(std::path::PathBuf::from("assets").join("benches31.blend")),
        );
        let version_raw = blend.blend.header.version;
        let version = (
            version_raw[0] - 48,
            version_raw[1] - 48,
            version_raw[2] - 48,
        );

        let mesh_instance = get_mesh_by_name(&blend, "MECube_192").unwrap();

        b.iter(|| {
            instance_to_mesh(mesh_instance.clone(), version).unwrap();
        });
    }

    #[bench]
    fn instance_to_mesh_cube_768(b: &mut Bencher) {
        let blend = Blend::from_path(
            std::env::current_dir()
                .unwrap()
                .join(std::path::PathBuf::from("assets").join("benches31.blend")),
        );
        let version_raw = blend.blend.header.version;
        let version = (
            version_raw[0] - 48,
            version_raw[1] - 48,
            version_raw[2] - 48,
        );

        let mesh_instance = get_mesh_by_name(&blend, "MECube_768").unwrap();

        b.iter(|| {
            instance_to_mesh(mesh_instance.clone(), version).unwrap();
        });
    }

    #[bench]
    fn instance_to_mesh_cube_3072(b: &mut Bencher) {
        let blend = Blend::from_path(
            std::env::current_dir()
                .unwrap()
                .join(std::path::PathBuf::from("assets").join("benches31.blend")),
        );
        let version_raw = blend.blend.header.version;
        let version = (
            version_raw[0] - 48,
            version_raw[1] - 48,
            version_raw[2] - 48,
        );

        let mesh_instance = get_mesh_by_name(&blend, "MECube_3072").unwrap();

        b.iter(|| {
            instance_to_mesh(mesh_instance.clone(), version).unwrap();
        });
    }

    #[bench]
    fn instance_to_mesh_cube_12288(b: &mut Bencher) {
        let blend = Blend::from_path(
            std::env::current_dir()
                .unwrap()
                .join(std::path::PathBuf::from("assets").join("benches31.blend")),
        );
        let version_raw = blend.blend.header.version;
        let version = (
            version_raw[0] - 48,
            version_raw[1] - 48,
            version_raw[2] - 48,
        );

        let mesh_instance = get_mesh_by_name(&blend, "MECube_12288").unwrap();

        b.iter(|| {
            instance_to_mesh(mesh_instance.clone(), version).unwrap();
        });
    }

    #[bench]
    fn instance_to_mesh_cube_49125(b: &mut Bencher) {
        let blend = Blend::from_path(
            std::env::current_dir()
                .unwrap()
                .join(std::path::PathBuf::from("assets").join("benches31.blend")),
        );
        let version_raw = blend.blend.header.version;
        let version = (
            version_raw[0] - 48,
            version_raw[1] - 48,
            version_raw[2] - 48,
        );

        let mesh_instance = get_mesh_by_name(&blend, "MECube_49125").unwrap();

        b.iter(|| {
            instance_to_mesh(mesh_instance.clone(), version).unwrap();
        });
    }
}
