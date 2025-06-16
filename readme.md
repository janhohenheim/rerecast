# Fix glTF coordinate system

[![crates.io](https://img.shields.io/crates/v/bevy_fix_gltf_coordinate_system)](https://crates.io/crates/bevy_fix_gltf_coordinate_system)
[![docs.rs](https://docs.rs/bevy_fix_gltf_coordinate_system/badge.svg)](https://docs.rs/bevy_fix_gltf_coordinate_system)

A tiny plugin to fix the fact that Bevy does not respect the GLTF coordinate system.


## Usage

Just add the plugin, that's it:

```rust,no_run
use bevy::prelude::*;
use bevy_fix_gltf_coordinate_system::prelude::*;

App::new()
  .add_plugins(DefaultPlugins)
  .add_plugins(FixGltfCoordinateSystemPlugin);
```

Now, all `SceneRoot`s you spawn are now correctly oriented. If you want to exclude a specific scene, add a `DoNotFixGltfCoordinateSystem` to it.

## Background

glTF uses +Z as forward, while Bevy uses -Z. However, [the glTF importer ignores this fact](https://github.com/bevyengine/bevy/issues/5670) and pretends
that glTF and Bevy use the same coordinate system.
The result is that all glTFs imported into Bevy are rotated by 180 degrees. This plugin fixes that.
