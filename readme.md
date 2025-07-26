# Rerecast
[![crates.io](https://img.shields.io/crates/v/rerecast)](https://crates.io/crates/rerecast)
[![docs.rs](https://docs.rs/rerecast/badge.svg)](https://docs.rs/rerecast)

Rust port of of [Recast](https://github.com/recastnavigation/recastnavigation), the industry-standard navigation mesh generator used
by Unreal, Unity, Godot, and other game engines.

## Features & Roadmap

### Rerecast

- [x] Generate polygon mesh
- [x] Generate detail meshes
- [ ] Generate tiles
- Partitioning
  - [x] Watershed
  - [ ] Monotone
  - [ ] Layer

### Bevy Integration

- Editor
  - [x] Extract meshes from running game
  - [ ] Configure navmesh generation
  - [ ] Visualize navmesh
  - [ ] Send navmesh to running game
  - [ ] Save and load navmesh
- API
  - [x] Optional editor communication
  - [ ] Generate navmeshes on demand
  - [ ] Automatically regenerate navmeshes
