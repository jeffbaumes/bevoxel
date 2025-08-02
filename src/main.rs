use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

mod chunk;
mod player;
mod systems;
mod voxel;
mod world;

use chunk::*;
use player::*;
use systems::*;
use voxel::{Material as VoxelMaterial, MaterialRegistry};
use world::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevox - Next-Gen Voxel Engine".into(),
                resolution: (1280., 720.).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .init_resource::<VoxelWorld>()
        .init_resource::<VoxelEditingConfig>()
        .init_resource::<PlayerPhysicsConfig>()
        .init_resource::<RenderingConfig>()
        .add_systems(
            Startup,
            (
                setup_material_registry,
                setup_rendering_config,
                setup_world,
                setup_player,
                setup_crosshair,
            )
                .chain(),
        )
        .add_systems(Update, world_generation_system.before(chunk_loading_system))
        .add_systems(
            Update,
            (
                player_movement_system,
                player_world_update_system,
                chunk_loading_system,
                chunk_meshing_system,
                voxel_interaction_system,
            ),
        )
        .run();
}

fn setup_world(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn setup_crosshair(mut commands: Commands) {
    // Create a UI root node
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(1000), // Ensure it's on top
        ))
        .with_children(|parent| {
            // Horizontal line
            parent.spawn((
                Node {
                    width: Val::Px(20.0),
                    height: Val::Px(2.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            ));

            // Vertical line
            parent.spawn((
                Node {
                    width: Val::Px(2.0),
                    height: Val::Px(20.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            ));
        });
}

fn setup_material_registry(mut commands: Commands) {
    let mut registry = MaterialRegistry::new();

    // Register basic materials
    registry.register(VoxelMaterial::new("air", [0.0, 0.0, 0.0, 0.0], false));
    registry.register(VoxelMaterial::new("stone", [0.5, 0.5, 0.5, 1.0], true));
    registry.register(VoxelMaterial::new("dirt", [0.4, 0.2, 0.1, 1.0], true));
    registry.register(VoxelMaterial::new("grass", [0.2, 0.7, 0.2, 1.0], true));
    registry.register(VoxelMaterial::new("water", [0.2, 0.4, 0.8, 0.7], false));
    registry.register(VoxelMaterial::new("sand", [0.9, 0.8, 0.6, 1.0], true));
    registry.register(VoxelMaterial::new("wood", [0.6, 0.4, 0.2, 1.0], true));
    registry.register(VoxelMaterial::new("leaves", [0.1, 0.6, 0.1, 1.0], true));

    commands.insert_resource(registry);
}

fn setup_rendering_config(mut commands: Commands) {
    let mut config = RenderingConfig::default();

    // Configure normal sampling radius for terrain smoothness
    // RADIUS = 1: Fast, basic smoothing
    // RADIUS = 2: Higher quality, balanced performance (default)
    // RADIUS = 3: Maximum quality, more expensive
    config.normal_sampling_radius = 3;

    commands.insert_resource(config);
}

fn world_generation_system(mut world: ResMut<VoxelWorld>) {
    // Check if there are any chunks that need terrain generation
    let chunks_to_generate: Vec<ChunkCoord> = world
        .chunks
        .iter()
        .filter_map(|(coord, chunk)| {
            // Check if chunk needs generation (empty and not modified)
            if !chunk.modified && chunk.material_palette.len() == 1 {
                // Only has "air"
                Some(*coord)
            } else {
                None
            }
        })
        .collect();

    for coord in chunks_to_generate {
        if let Some(chunk) = world.chunks.get_mut(&coord) {
            generate_terrain(chunk);
        }
    }
}

fn generate_terrain(chunk: &mut ChunkData) {
    let noise = Perlin::new(42);
    let chunk_world_pos = chunk.coord.to_world_pos();

    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let world_x = chunk_world_pos.x + x as f32;
            let world_z = chunk_world_pos.z + z as f32;

            let height =
                (noise.get([world_x as f64 * 0.01, world_z as f64 * 0.01]) * 20.0 + 50.0) as i32;

            for y in 0..CHUNK_SIZE {
                let world_y = chunk_world_pos.y as i32 + y as i32;

                let material_name = if world_y > height {
                    "air"
                } else if world_y == height {
                    "grass"
                } else if world_y > height - 4 {
                    "dirt"
                } else {
                    "stone"
                };

                chunk.set_voxel_by_material(x, y, z, material_name);
            }
        }
    }

    chunk.modified = true;
}
