# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Bevoxel is a high-performance voxel engine built with Rust and Bevy, designed for infinite worlds with rapid terrain modification. The project uses Bevy's Entity Component System (ECS) architecture for performance and maintainability.

## Build and Development Commands

```bash
# Build the project (debug mode with optimizations)
cargo build

# Run the game (debug mode)
cargo run

# Build for release (full optimizations)
cargo build --release

# Run release version
cargo run --release

# Check code without building
cargo check

# Run tests (if any exist)
cargo test

# Format code
cargo fmt

# Run clippy lints
cargo clippy
```

## Core Architecture

### Module Structure
- `main.rs`: Application entry point, sets up Bevy app and systems
- `voxel.rs`: Defines `VoxelType` enum and `Voxel` struct with type/color mapping
- `chunk.rs`: Core chunk system with 32³ voxel containers and coordinate system
- `world.rs`: World management with infinite chunk loading/unloading
- `player.rs`: Player component and first-person camera setup
- `systems.rs`: All game logic systems (movement, chunk loading, meshing, interaction)

### Key Components and Resources
- **VoxelWorld** (Resource): Manages all chunks, loading/meshing queues, player position
- **ChunkData**: Contains 32³ voxel array with coordinate system and terrain generation
- **Player**: Component with speed, sensitivity, and reach settings
- **ChunkMesh**: Component for rendered chunk entities

### System Architecture
The game runs four main systems in Update schedule:
1. **player_movement_system**: Handles WASD movement, mouse look, cursor grabbing
2. **chunk_loading_system**: Processes chunk loading queue (max 2 chunks/frame)
3. **chunk_meshing_system**: Converts voxel data to meshes (max 4 meshes/frame)  
4. **voxel_interaction_system**: Handles block placement/breaking with raycasting

### Chunk System Details
- **Chunk Size**: 32×32×32 voxels per chunk
- **Coordinate System**: ChunkCoord handles world ↔ chunk position conversion
- **Render Distance**: 8 chunks (configurable in world.rs:7)
- **Unload Distance**: 12 chunks (configurable in world.rs:8)
- **Terrain Generation**: Uses Perlin noise for height maps in `chunk.generate_terrain()`

### Performance Optimizations
- Face culling: Only renders faces adjacent to transparent voxels
- Chunk-based LOD: Only loads/meshes chunks within render distance
- Multi-threaded processing: Configurable limits for chunk loading/meshing per frame
- Greedy meshing: Implemented in `generate_chunk_mesh()` function
- Memory efficiency: Chunks unload when beyond unload distance

## Key File Locations

- Chunk loading logic: `systems.rs:84-96`
- Mesh generation: `systems.rs:176-210` 
- Terrain generation: `chunk.rs:120-152`
- Player controls: `systems.rs:9-82`
- Voxel interaction: `systems.rs:142-174`
- World coordinate conversion: `chunk.rs:22-36`

## Development Notes

### Adding New Voxel Types
1. Add variant to `VoxelType` enum in `voxel.rs:6-15`
2. Update `get_color()` method in `voxel.rs:26-37`
3. Update `is_solid()` and `is_transparent()` methods if needed
4. Add corresponding key binding in `voxel_interaction_system`

### Performance Tuning
- Adjust `MAX_CHUNKS_PER_FRAME` and `MAX_MESHES_PER_FRAME` in systems.rs
- Modify `RENDER_DISTANCE` and `UNLOAD_DISTANCE` in world.rs
- Chunk size is hardcoded as 32³ - changing requires updating CHUNK_SIZE constant

### World Persistence 
- Chunk serialization is implemented with serde
- Disk I/O stubs exist in `world.rs:144-149` but are not implemented
- Modified chunks are marked for saving in `ChunkData.modified`

### Build Recommendations
- To test that it builds properly, always use `cargo build --release`. The user will actually run the program to test it.
- Always build with --release