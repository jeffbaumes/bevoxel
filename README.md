# Bevoxel - Next-Generation Voxel Engine

A high-performance voxel engine built with Rust and Bevy, designed for infinite worlds with rapid terrain modification.

## Features Implemented

✅ **Core Voxel System**
- VoxelType enum with multiple block types (Air, Stone, Dirt, Grass, Water, Sand, Wood, Leaves)
- Voxel struct with type information and color mapping
- Efficient chunk-based world representation (32³ voxels per chunk)

✅ **Infinite World System**
- Dynamic chunk loading/unloading based on player position
- Configurable render distance (8 chunks) and unload distance (12 chunks)
- Procedural terrain generation using Perlin noise
- Chunk serialization support (ready for disk storage)

✅ **High-Performance Meshing**
- Optimized greedy meshing algorithm
- GPU-accelerated chunk mesh generation
- Face culling for hidden voxel faces
- Multi-threaded chunk processing with configurable limits

✅ **Player Controller**
- First-person camera with mouse look
- WASD movement with space/shift for vertical movement
- Mouse cursor capture for immersive gameplay
- Configurable movement speed and mouse sensitivity

✅ **Voxel Interaction System**
- Ray-casting for voxel selection
- Left-click to break blocks
- Right-click to place blocks
- Number keys (1-3) to select block types
- Real-time world modification with mesh updates

## Architecture

### Core Components
- **VoxelWorld**: Resource managing all chunks and world state
- **Chunk**: 32³ voxel data container with coordinate system
- **ChunkMesh**: Component for rendered chunk entities
- **Player**: Component with movement and interaction settings

### Core Systems
- **chunk_loading_system**: Manages infinite world generation
- **chunk_meshing_system**: Converts voxel data to renderable meshes
- **player_movement_system**: Handles input and camera controls
- **voxel_interaction_system**: Manages block placement/breaking

## Controls

### Movement
- **Mouse**: Look around
- **WASD**: Move horizontally
- **Space**: Jump
- **Escape**: Release mouse cursor

### Voxel Editing
- **Left Click**: Remove voxels (brush area)
- **Right Click**: Place voxels (brush area)
- **1/2/3**: Select block type (Stone/Dirt/Grass)
- **B**: Toggle brush shape (Ball/Cube)
- **[ ]**: Decrease/Increase brush size
- **Default brush**: 2.0 radius ball, 8.0 reach distance

### Player Physics Configuration
- **P**: Toggle collision mode (Basic/Capsule)
- **+/-**: Increase/Decrease player size (camera height adjusts automatically)
- **Default physics**: 1.2x3.6 capsule collision with step-up
- **Eye height**: 80% of player height (updates dynamically)

## Performance Features

- **Chunk-based LOD**: Only generate meshes for visible chunks
- **Greedy meshing**: Reduces vertex count by combining adjacent faces
- **Multi-threaded processing**: Configurable chunk processing limits
- **Memory efficient**: Sparse chunk storage, unload distant chunks
- **GPU optimization**: Modern Bevy renderer with PBR materials

## Building and Running

```bash
# Build the project
cargo build --release

# Run the game
cargo run --release
```

## Technical Specifications

- **Chunk Size**: 32×32×32 voxels
- **World Height**: Unlimited (configurable chunks in Y axis)
- **Render Distance**: 8 chunks (configurable)
- **Max Chunks per Frame**: 2 for loading, 4 for meshing
- **Terrain Generator**: Perlin noise with height mapping
- **Graphics API**: Modern Bevy renderer (Vulkan/DirectX 12/Metal)

## Next Steps

The engine provides a solid foundation for:
- Advanced terrain features (caves, overhangs, complex structures)
- Multiplayer networking
- Advanced lighting and shadows
- Texture atlasing and materials
- Biome generation
- Water physics and fluid simulation
- Advanced player physics (collision detection, gravity)

## Architecture Design Goals

Bevoxel was designed with these principles:
- **Performance**: GPU-accelerated operations where possible
- **Scalability**: Infinite worlds with efficient memory usage
- **Modularity**: Clean separation of concerns with Bevy's ECS
- **Extensibility**: Easy to add new voxel types and behaviors
- **Modern Rust**: Memory safety without garbage collection overhead
