use blend::{Blend, runtime::Instance};
use std::io::{Error, ErrorKind};

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, AssetLoader, LoadContext, LoadedAsset};
use bevy_render::{
    mesh::{Indices, Mesh},
    pipeline::PrimitiveTopology,
};
use bevy_utils::BoxedFuture;



pub struct BlenderPlugin;

impl Plugin for BlenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_asset_loader::<BlenderLoader>();
    }
}

#[derive(Default)]
struct BlenderLoader;

impl AssetLoader for BlenderLoader {

    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<()>> {
        Box::pin(async move { Ok(load_blend_meshes(bytes, load_context).await?) })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["blend"];
        EXTENSIONS
    }
}



async fn load_blend_meshes <'a, 'b>(
    bytes: &'a [u8],
    load_context: &'a mut LoadContext<'b>,
    ) -> anyhow::Result<()> {
        let blend = Blend::new(bytes);
        for mesh in blend.get_by_code(*b"ME") {
            let mesh_name = mesh.get("id").get_string("name");
            let mut label = mesh_name.clone();
            label.replace_range(0..2, "");
            println!("{} {}", mesh_name, label);
            load_context.set_labeled_asset(label.as_str(), LoadedAsset::new(instance_to_mesh(mesh)?));
        }

        Ok(())
}

fn instance_to_mesh(instance: Instance) -> anyhow::Result<Mesh> {

    let faces = instance.get_iter("mpoly").collect::<Vec<_>>();
    let loops = instance.get_iter("mloop").collect::<Vec<_>>();
    let uvs = instance.get_iter("mloopuv").collect::<Vec<_>>();
    let verts = instance.get_iter("mvert").collect::<Vec<_>>();

    println!("n_faces: {}", faces.len());
    //println!("{:?}", faces);
    println!("n_loops: {}", loops.len());
    println!("n_uvs: {}", uvs.len());
    println!("n_verts: {}", verts.len());
    println!("{:?}", verts[0]);

    let vertices = vec![
        ([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0]),
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0]),
        ([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 1.0]),
    ];

    let faces = [[0, 1, 2]];

    let mut indices = Vec::new();

    for face in faces {
        indices.push(face[2]); indices.push(face[0]);
        indices.push(face[1]); indices.push(face[2]);
        indices.push(face[0]); indices.push(face[1]);
    }

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    for (position, normal, uv) in vertices.iter() {
        positions.push(*position);
        normals.push(*normal);
        uvs.push(*uv);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    Ok(mesh)
}










