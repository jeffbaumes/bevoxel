use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

mod chunk;
mod config;
mod inventory;
mod player;
mod systems;
mod voxel;
mod world;

use chunk::*;
use config::*;
use inventory::*;
use player::*;
use systems::*;
use voxel::{Material as VoxelMaterial, MaterialRegistry};
use world::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevoxel - Next-Gen Voxel Engine".into(),
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
        .init_resource::<GameConfig>()
        .init_resource::<VoxelTintState>()
        .add_systems(
            Startup,
            (
                setup_material_registry,
                setup_rendering_config,
                setup_world,
                setup_player,
                setup_crosshair,
                setup_voxel_tint_overlay,
                setup_inventory,
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
                voxel_tint_system,
                update_voxel_tint_overlay,
                handle_inventory_navigation,
                update_inventory_ui,
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

    // Register basic materials with color variation
    registry.register(VoxelMaterial::new("air", [0.0, 0.0, 0.0, 0.0], false));
    registry.register(VoxelMaterial::with_variance(
        "stone",
        [0.5, 0.5, 0.5, 1.0],
        true,
        0.08,
    ));
    registry.register(VoxelMaterial::with_variance(
        "dirt",
        [0.4, 0.2, 0.1, 1.0],
        true,
        0.06,
    ));
    registry.register(VoxelMaterial::with_variance(
        "grass",
        [0.2, 0.7, 0.2, 1.0],
        true,
        0.1,
    ));
    registry.register(VoxelMaterial::with_buoyancy("water", [0.2, 0.4, 0.8, 0.7], false, 0.3, 0.6));
    registry.register(VoxelMaterial::with_buoyancy(
        "murky_water",
        [0.3, 0.5, 0.4, 0.8],
        false,
        0.1, // More sluggish - stronger gravity effect
        0.4, // Weaker swimming
    ));
    registry.register(VoxelMaterial::new("glass", [0.9, 0.9, 0.9, 0.3], true));
    registry.register(VoxelMaterial::with_variance(
        "sand",
        [0.9, 0.8, 0.6, 1.0],
        true,
        0.05,
    ));
    registry.register(VoxelMaterial::with_variance(
        "wood",
        [0.6, 0.4, 0.2, 1.0],
        true,
        0.07,
    ));
    registry.register(VoxelMaterial::with_variance(
        "leaves",
        [0.1, 0.6, 0.1, 0.8],
        true,
        0.12,
    ));

    commands.insert_resource(registry);
}

fn setup_rendering_config(mut commands: Commands) {
    let mut config = RenderingConfig::default();

    // Configure normal sampling radius for terrain smoothness
    // RADIUS = 1: Fast, basic smoothing
    // RADIUS = 2: Higher quality, balanced performance (default)
    // RADIUS = 3: Maximum quality, more expensive
    config.normal_sampling_radius = 3;

    // Configure transparency chunk size for better sorting
    // Smaller values = more mesh entities but better transparency sorting
    // 8 = good balance, 4 = more entities/better sorting, 16 = fewer entities/worse sorting
    config.transparency_chunk_size = 8;

    // Enable basic normals mode (flat face normals)
    // When enabled, transparent geometry horizontal faces always use Y-up normals
    config.use_basic_normals = true;

    commands.insert_resource(config);
}

fn setup_inventory(mut commands: Commands) {
    let mut inventory = Inventory::new(4, 8); // 4 rows, 8 columns
    inventory.initialize_with_test_content();
    
    setup_inventory_ui(&mut commands, &inventory);
    commands.insert_resource(inventory);
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
                    // "air"
                    if world_y < 45 {
                        "murky_water" // Add water below sea level
                    } else if world_y < 50 {
                        "water"
                    } else {
                        "air"
                    }
                } else if world_y == height && height >= 45 {
                    "grass"
                } else if world_y == height && height < 45 {
                    "grass" // Sand at water level
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
