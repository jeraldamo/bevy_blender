# bevy_blender

[![Crate version](https://img.shields.io/crates/v/bevy_blender?style=flat-square)](https://crates.io/crates/bevy_blender/)
![Crate license](https://img.shields.io/crates/l/bevy_blender?style=flat-square)

bevy_blender is a [Bevy](https://bevyengine.org) library that allows you to use assets created in [Blender](https://blender.org) directly from the .blend file.

### Purpose
1) I am learning Rust and it seemed like a not-too-easy and not-too-hard problem.
1) I would like to be able to maintain several assets in the same .blend file and not have to worry about exporting them.
1) It seems like a good way to contribute to the Bevy project.

### Related Works
1) [Arsenal](https://github.com/katharostech/arsenal) is a project with the goal of using Blender as a UI for creating Bevy games.
1) Reddit/Github user sdfgeoff created the [Blender Bevy Toolkit](https://www.reddit.com/r/rust_gamedev/comments/mr60x4/my_workflow_for_3d_assets_and_custom_components/) which exports Blender objects to Bevy readable scene files (as well as some other cool things).

Both of these projects are neat, but do not serve my desired use case. They both act as extensions of the Bevy game engine, using Blender almost as a front-end framework. I simply want to create assets in Blender and have a ridiculously easy way to access those assets in Bevy with a minimal amount of middle work.

### Credit where credit is due
1) This project was heavily modeled after the [bevy_stl](https://github.com/nilclass/bevy_stl) project, so thanks nilclass!
1) Much of the heavy lifting is accomplished using the [Blend crate](https://github.com/lukebitts/blend), thanks lukebitts!

### Usage
1) Add `bevy_blender` to your `Cargo.toml` dependencies.
1) Add `bevy_blender::BlenderPlugin` plugin to the bevy `App`
1) Load Blender mesh assets by using the included macro with `asset_server.load`. For example: `asset_server.load(blender_mesh!("blend_file.blend", "mesh_name"))`

*If the asset name in Blender starts with an underscore, it will not be loaded. You can use this to have extra assets in the .blend file that you do not want loaded to the AssetServer.*

#### Example
```rust
fn main() {
    App::build()
        .add_plugin(bevy_blender::BlenderPlugin)
        .add_startup_system(setup.system())
        // ...
        .run();
}

fn setup(commands: &mut Commands, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn_bundle(PbrBundle {
            mesh: asset_server.load(blender_mesh!("demo.blend", "Suzanne")),
            material: materials.add(Color::rgb(0.9, 0.4, 0.3).into()),
            ..Default::default()
        })
        // ...
}
```

A full example can be found in `examples/demo.rs`. Simply run `cargo run --example demo` to execute it. This will open a .blend file located at `assets/demo.blend`. Running this demo will look like this (note that the ngon cap on the cylinder is missing, see the limitations section for more details):
![demo bevy window](assets/demo_bevy.png "Demo Bevy Window")

### Aspirations
Currently, bevy_blend can only load mesh data from a .blend file. I plan to extend this to allow users to import any asset type supported by both Blender and Bevy (assuming there is some reasonable way to convert between the two). Ultimately I would like to create a `BlenderObjectBundle` which would load all assets belonging to a Blender object, as well as it descendents. The goal would be that, barring minor rendering differences, an object rendered in Blender will look the same rendered in Bevy.

**If you have other ideas for how this project could be used, please let me know!**

### Known limitations
* Currently only tri and quad faces are supported, ngon face support is coming though.
* Only non-compressed .blend files work. Though Blender uses the standard zlib compression (I think), so it should be easy enough to detect a compressed .blend file and uncompress it.
* Only the named mesh is constructed, children meshes have to be manually constructed and placed relative to the parent mesh. See the above aspiration section.
* Blender modifiers are not applied before constructing the mesh.
* The .blend file is read, and all meshes are parsed, for each call to asset_server.load(). Ideally we would parse the .blend file once and just make all of the meshes available. Still trying to find the best way to accomplish this.
