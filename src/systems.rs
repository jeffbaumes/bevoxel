use crate::chunk::{ChunkMesh, OpaqueMesh, TransparentMesh};
use crate::inventory::Inventory;
use crate::player::{Player, PlayerCamera};
use crate::voxel::{MaterialRegistry, Voxel};
use crate::world::{
    BrushShape, CollisionMode, PlayerPhysicsConfig, RenderingConfig, VoxelEditingConfig, VoxelWorld,
};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::alpha::AlphaMode;
use bevy::window::CursorGrabMode;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[derive(Resource, Default)]
pub struct VoxelTintState {
    pub current_tint: Color,
    pub target_tint: Color,
    pub tint_strength: f32,
}

impl VoxelTintState {
    pub fn new() -> Self {
        Self {
            current_tint: Color::NONE,
            target_tint: Color::NONE,
            tint_strength: 0.0,
        }
    }
}

fn is_voxel_solid_at_pos(
    world: &VoxelWorld,
    pos: Vec3,
    material_registry: &MaterialRegistry,
) -> bool {
    let voxel = world.get_voxel_at_world_pos(pos);
    if let Some(chunk) = world.get_chunk_at_world_pos(pos) {
        if let Some(material_name) = chunk.get_material_name(voxel.material_id) {
            return material_registry.get(material_name).is_solid();
        }
    }
    false
}

fn get_material_at_position<'a>(
    world: &VoxelWorld,
    pos: Vec3,
    material_registry: &'a MaterialRegistry,
) -> &'a crate::voxel::Material {
    let voxel = world.get_voxel_at_world_pos(pos);
    if let Some(chunk) = world.get_chunk_at_world_pos(pos) {
        if let Some(material_name) = chunk.get_material_name(voxel.material_id) {
            return material_registry.get(material_name);
        }
    }
    // Default to air if no chunk or material found
    material_registry.get("air")
}

fn apply_movement_with_collision(
    current_pos: Vec3,
    movement: Vec3,
    world: &VoxelWorld,
    player: &mut crate::player::Player,
    physics_config: &PlayerPhysicsConfig,
    material_registry: &MaterialRegistry,
) -> Vec3 {
    let mut new_pos = current_pos;

    // Test movement in each axis separately to allow sliding
    // X-axis movement
    if movement.x.abs() > 0.001 {
        let test_pos = Vec3::new(current_pos.x + movement.x, current_pos.y, current_pos.z);
        if !check_collision(test_pos, world, physics_config, material_registry) {
            new_pos.x = test_pos.x;
        } else {
            player.velocity.x = 0.0;
        }
    }

    // Z-axis movement
    if movement.z.abs() > 0.001 {
        let test_pos = Vec3::new(new_pos.x, current_pos.y, current_pos.z + movement.z);
        if !check_collision(test_pos, world, physics_config, material_registry) {
            new_pos.z = test_pos.z;
        } else {
            player.velocity.z = 0.0;
        }
    }

    // Y-axis movement (vertical)
    if movement.y.abs() > 0.001 {
        let test_pos = Vec3::new(new_pos.x, current_pos.y + movement.y, new_pos.z);
        if !check_collision(test_pos, world, physics_config, material_registry) {
            new_pos.y = test_pos.y;
            if movement.y < 0.0 {
                player.is_grounded = false;
            }
        } else {
            if movement.y < 0.0 {
                // Hit ground
                player.is_grounded = true;
                player.velocity.y = 0.0;
            } else {
                // Hit ceiling
                player.velocity.y = 0.0;
            }
        }
    } else {
        // Check if still grounded when not moving vertically
        let ground_test = Vec3::new(new_pos.x, current_pos.y - 0.1, new_pos.z);
        player.is_grounded = check_collision(ground_test, world, physics_config, material_registry);
    }

    new_pos
}

fn apply_capsule_movement_with_collision(
    current_pos: Vec3,
    movement: Vec3,
    world: &VoxelWorld,
    player: &mut crate::player::Player,
    physics_config: &PlayerPhysicsConfig,
    material_registry: &MaterialRegistry,
) -> Vec3 {
    let player_radius = physics_config.width * 0.5; // Capsule radius (half of width)
    let player_height = physics_config.height; // Total height
    let step_height = physics_config.step_height; // Maximum step height

    let mut new_pos = current_pos;

    // Horizontal movement with step-up
    let horizontal_movement = Vec3::new(movement.x, 0.0, movement.z);
    if horizontal_movement.length() > 0.001 {
        new_pos = apply_horizontal_movement_with_stepup(
            new_pos,
            horizontal_movement,
            world,
            player,
            player_radius,
            player_height,
            step_height,
            material_registry,
        );
    }

    // Vertical movement
    if movement.y.abs() > 0.001 {
        let test_pos = Vec3::new(new_pos.x, current_pos.y + movement.y, new_pos.z);
        if !check_capsule_collision(
            test_pos,
            world,
            player_radius,
            player_height,
            material_registry,
        ) {
            new_pos.y = test_pos.y;
            if movement.y < 0.0 {
                player.is_grounded = false;
            }
        } else {
            if movement.y < 0.0 {
                // Hit ground
                player.is_grounded = true;
                player.velocity.y = 0.0;
            } else {
                // Hit ceiling
                player.velocity.y = 0.0;
            }
        }
    } else {
        // Check if still grounded when not moving vertically
        let ground_test = Vec3::new(new_pos.x, current_pos.y - 0.1, new_pos.z);
        player.is_grounded = check_capsule_collision(
            ground_test,
            world,
            player_radius,
            player_height,
            material_registry,
        );
    }

    new_pos
}

fn apply_horizontal_movement_with_stepup(
    current_pos: Vec3,
    horizontal_movement: Vec3,
    world: &VoxelWorld,
    player: &mut crate::player::Player,
    radius: f32,
    height: f32,
    _step_height: f32,
    material_registry: &MaterialRegistry,
) -> Vec3 {
    let mut new_pos = current_pos;

    // Try normal horizontal movement first
    let test_pos = Vec3::new(
        current_pos.x + horizontal_movement.x,
        current_pos.y,
        current_pos.z + horizontal_movement.z,
    );

    if !check_capsule_collision(test_pos, world, radius, height, material_registry) {
        // Normal movement works
        new_pos.x = test_pos.x;
        new_pos.z = test_pos.z;
    } else {
        // Try step-up: check if we can move up and then forward
        for step_up in [0.5, 1.0] {
            // Try half-step then full step
            let step_test_pos = Vec3::new(
                current_pos.x + horizontal_movement.x,
                current_pos.y + step_up,
                current_pos.z + horizontal_movement.z,
            );

            if !check_capsule_collision(step_test_pos, world, radius, height, material_registry) {
                // We can step up and move forward
                new_pos.x = step_test_pos.x;
                new_pos.z = step_test_pos.z;
                new_pos.y = step_test_pos.y;
                break;
            }
        }

        // If step-up didn't work, try sliding along walls
        if new_pos.x == current_pos.x && new_pos.z == current_pos.z {
            // Try X movement only
            let x_test = Vec3::new(
                current_pos.x + horizontal_movement.x,
                current_pos.y,
                current_pos.z,
            );
            if !check_capsule_collision(x_test, world, radius, height, material_registry) {
                new_pos.x = x_test.x;
            } else {
                player.velocity.x = 0.0;
            }

            // Try Z movement only
            let z_test = Vec3::new(
                new_pos.x,
                current_pos.y,
                current_pos.z + horizontal_movement.z,
            );
            if !check_capsule_collision(z_test, world, radius, height, material_registry) {
                new_pos.z = z_test.z;
            } else {
                player.velocity.z = 0.0;
            }
        }
    }

    new_pos
}

fn check_capsule_collision(
    pos: Vec3,
    world: &VoxelWorld,
    radius: f32,
    height: f32,
    material_registry: &MaterialRegistry,
) -> bool {
    // Check collision using a capsule shape (cylinder with rounded ends)
    let bottom_center = pos;
    let top_center = pos + Vec3::new(0.0, height - radius * 2.0, 0.0);

    // Check cylinder body
    let num_height_samples = ((height - radius * 2.0) / 0.5).ceil() as i32 + 1;
    for i in 0..num_height_samples {
        let t = if num_height_samples > 1 {
            i as f32 / (num_height_samples - 1) as f32
        } else {
            0.0
        };
        let sample_pos = bottom_center.lerp(top_center, t) + Vec3::new(0.0, radius, 0.0);

        if check_circle_collision(sample_pos, world, radius, material_registry) {
            return true;
        }
    }

    // Check bottom hemisphere
    if check_hemisphere_collision(
        bottom_center + Vec3::new(0.0, radius, 0.0),
        world,
        radius,
        false,
        material_registry,
    ) {
        return true;
    }

    // Check top hemisphere
    if check_hemisphere_collision(
        top_center + Vec3::new(0.0, radius, 0.0),
        world,
        radius,
        true,
        material_registry,
    ) {
        return true;
    }

    false
}

fn check_circle_collision(
    center: Vec3,
    world: &VoxelWorld,
    radius: f32,
    material_registry: &MaterialRegistry,
) -> bool {
    // Sample points in a circle around the center
    let num_samples = 8;
    for i in 0..num_samples {
        let angle = (i as f32 / num_samples as f32) * 2.0 * std::f32::consts::PI;
        let offset = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let check_pos = center + offset;

        if is_voxel_solid_at_pos(world, check_pos, material_registry) {
            return true;
        }
    }

    // Also check center
    is_voxel_solid_at_pos(world, center, material_registry)
}

fn check_hemisphere_collision(
    center: Vec3,
    world: &VoxelWorld,
    radius: f32,
    is_top: bool,
    material_registry: &MaterialRegistry,
) -> bool {
    // Sample points in a hemisphere
    let num_samples = 6;
    for i in 0..num_samples {
        let phi = (i as f32 / num_samples as f32) * std::f32::consts::PI; // 0 to PI
        let theta_samples = (4.0 * phi.sin()).max(1.0) as i32;

        for j in 0..theta_samples {
            let theta = (j as f32 / theta_samples as f32) * 2.0 * std::f32::consts::PI;

            let y_offset = if is_top {
                phi.cos() * radius
            } else {
                -phi.cos() * radius
            };
            let x_offset = phi.sin() * radius * theta.cos();
            let z_offset = phi.sin() * radius * theta.sin();

            let check_pos = center + Vec3::new(x_offset, y_offset, z_offset);
            if is_voxel_solid_at_pos(world, check_pos, material_registry) {
                return true;
            }
        }
    }
    false
}

fn check_collision(
    pos: Vec3,
    world: &VoxelWorld,
    physics_config: &PlayerPhysicsConfig,
    material_registry: &MaterialRegistry,
) -> bool {
    let half_width = physics_config.width * 0.5;
    let height = physics_config.height;
    let samples = physics_config.collision_samples;

    // Adaptive sampling based on player size and configuration
    let height_samples = (samples.max(3) / 3).max(2); // At least 2, typically 3+ height levels
    let width_samples = samples.max(3); // At least 3 width samples per height level

    for i in 0..height_samples {
        let y_offset = if height_samples == 1 {
            height * 0.5
        } else {
            (i as f32 / (height_samples - 1) as f32) * height
        };

        // Sample in a circle pattern for better coverage
        for j in 0..width_samples {
            let angle = (j as f32 / width_samples as f32) * 2.0 * std::f32::consts::PI;
            let x_offset = angle.cos() * half_width;
            let z_offset = angle.sin() * half_width;

            let check_pos = pos + Vec3::new(x_offset, y_offset, z_offset);
            if is_voxel_solid_at_pos(world, check_pos, material_registry) {
                return true;
            }
        }

        // Also check center at each height level
        let check_pos = pos + Vec3::new(0.0, y_offset, 0.0);
        if is_voxel_solid_at_pos(world, check_pos, material_registry) {
            return true;
        }
    }
    false
}

pub fn player_movement_system(
    mut player_query: Query<(&mut Transform, &mut Player), Without<PlayerCamera>>,
    mut camera_query: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut windows: Query<&mut Window>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    world: Res<VoxelWorld>,
    physics_config: Res<PlayerPhysicsConfig>,
    material_registry: Res<MaterialRegistry>,
) {
    let Ok((mut player_transform, mut player)) = player_query.get_single_mut() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.get_single_mut() else {
        return;
    };
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    // Update camera height based on player configuration (80% of player height)
    let target_eye_height = physics_config.height * 0.8;
    if (camera_transform.translation.y - target_eye_height).abs() > 0.01 {
        camera_transform.translation.y = target_eye_height;
    }

    if mouse.just_pressed(MouseButton::Left) {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }

    if window.cursor_options.grab_mode == CursorGrabMode::Locked {
        // Mouse look
        for motion in mouse_motion.read() {
            let yaw = -motion.delta.x * player.sensitivity;
            let pitch_delta = -motion.delta.y * player.sensitivity;

            // Update yaw (horizontal rotation on player)
            player_transform.rotate_y(yaw);

            // Update and clamp accumulated pitch
            player.pitch += pitch_delta;
            let pitch_limit = std::f32::consts::FRAC_PI_2 - 0.01; // Just shy of 90 degrees
            player.pitch = player.pitch.clamp(-pitch_limit, pitch_limit);

            // Set camera rotation directly from accumulated pitch
            camera_transform.rotation = Quat::from_rotation_x(player.pitch);
        }

        // Horizontal movement input
        let mut horizontal_input = Vec3::ZERO;

        if keyboard.pressed(KeyCode::KeyW) {
            horizontal_input += player_transform.forward().as_vec3();
        }
        if keyboard.pressed(KeyCode::KeyS) {
            horizontal_input -= player_transform.forward().as_vec3();
        }
        if keyboard.pressed(KeyCode::KeyA) {
            horizontal_input -= player_transform.right().as_vec3();
        }
        if keyboard.pressed(KeyCode::KeyD) {
            horizontal_input += player_transform.right().as_vec3();
        }

        // Get the material the player is currently in (at center of player)
        let player_center =
            player_transform.translation + Vec3::new(0.0, physics_config.height * 0.5, 0.0);
        let current_material = get_material_at_position(&world, player_center, &material_registry);

        // Remove Y component for horizontal movement
        horizontal_input.y = 0.0;
        if horizontal_input.length() > 0.0 {
            horizontal_input = horizontal_input.normalize();
        }

        // Apply horizontal velocity with fluid resistance
        let horizontal_velocity = horizontal_input * player.speed;

        // If in a fluid, apply some resistance to horizontal movement
        if current_material.swim_strength > 0.0 {
            let fluid_resistance = 1.0 - (1.0 - current_material.gravity_modifier) * 0.5;
            player.velocity.x = horizontal_velocity.x * fluid_resistance;
            player.velocity.z = horizontal_velocity.z * fluid_resistance;
        } else {
            player.velocity.x = horizontal_velocity.x;
            player.velocity.z = horizontal_velocity.z;
        }

        // Jumping and swimming
        if keyboard.just_pressed(KeyCode::Space) {
            if player.is_grounded {
                // Ground jump - but modified by fluid if underwater
                if current_material.swim_strength > 0.0 {
                    // Underwater ground jump - slower like swimming
                    player.velocity.y = player.jump_strength * current_material.swim_strength;
                } else {
                    // Normal air ground jump
                    player.velocity.y = player.jump_strength;
                }
                player.is_grounded = false;
            } else if current_material.swim_strength > 0.0 {
                // Swimming in fluid when not grounded
                player.velocity.y += player.jump_strength * current_material.swim_strength;
                // Cap swimming velocity to prevent infinite acceleration
                player.velocity.y = player.velocity.y.min(player.jump_strength * 0.8);
            }
        }

        // Apply gravity modified by current material
        let effective_gravity = player.gravity * current_material.gravity_modifier;
        player.velocity.y += effective_gravity * time.delta_secs();

        // Calculate movement with collision
        let mut new_position = player_transform.translation;
        let dt = time.delta_secs();

        // Apply movement with collision detection based on configuration
        new_position = match physics_config.collision_mode {
            CollisionMode::Capsule => apply_capsule_movement_with_collision(
                new_position,
                player.velocity * dt,
                &world,
                &mut player,
                &physics_config,
                &material_registry,
            ),
            CollisionMode::Basic => apply_movement_with_collision(
                new_position,
                player.velocity * dt,
                &world,
                &mut player,
                &physics_config,
                &material_registry,
            ),
        };

        player_transform.translation = new_position;
    }
}

pub fn player_world_update_system(
    player_query: Query<&Transform, (With<Player>, Changed<Transform>)>,
    mut world: ResMut<VoxelWorld>,
    config: Res<crate::config::GameConfig>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        world.update_player_position(player_transform.translation, &config);
    }
}

pub fn chunk_loading_system(
    mut world: ResMut<VoxelWorld>,
    config: Res<crate::config::GameConfig>,
    rendering_config: Res<RenderingConfig>,
    player_query: Query<&Transform, With<Player>>,
) {
    // Get player position for distance-based sorting
    let player_pos = if let Ok(player_transform) = player_query.get_single() {
        player_transform.translation
    } else {
        Vec3::ZERO // Fallback if no player found
    };

    // Convert loading queue to sorted vector based on distance to player
    let mut chunks_to_load: Vec<_> = world.loading_queue.drain(..).collect();

    // Sort by distance to player (closest first)
    chunks_to_load.sort_by(|a, b| {
        let dist_a = a
            .to_world_pos_with_size(rendering_config.chunk_size)
            .distance_squared(player_pos);
        let dist_b = b
            .to_world_pos_with_size(rendering_config.chunk_size)
            .distance_squared(player_pos);
        dist_a
            .partial_cmp(&dist_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Load closest chunks first (up to max per frame)
    let chunks_to_process = chunks_to_load.len().min(config.max_chunks_per_frame);
    for i in 0..chunks_to_process {
        world.load_chunk(chunks_to_load[i]);
    }

    // Put any remaining chunks back into the loading queue for next frame
    for coord in chunks_to_load.into_iter().skip(chunks_to_process) {
        world.loading_queue.push_back(coord);
    }
}

pub fn chunk_meshing_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut world: ResMut<VoxelWorld>,
    existing_opaque_meshes: Query<(Entity, &OpaqueMesh)>,
    existing_transparent_meshes: Query<(Entity, &TransparentMesh)>,
    material_registry: Res<MaterialRegistry>,
    rendering_config: Res<RenderingConfig>,
    config: Res<crate::config::GameConfig>,
) {
    let mut existing_opaque_map = std::collections::HashMap::new();
    for (entity, mesh) in existing_opaque_meshes.iter() {
        existing_opaque_map.insert(mesh.coord, entity);
    }

    let mut existing_transparent_map: std::collections::HashMap<
        crate::chunk::ChunkCoord,
        Vec<Entity>,
    > = std::collections::HashMap::new();
    for (entity, mesh) in existing_transparent_meshes.iter() {
        existing_transparent_map
            .entry(mesh.coord)
            .or_insert_with(Vec::new)
            .push(entity);
    }

    // Simple FIFO approach - combine both queues and process in order
    let mut all_chunks: Vec<_> = world.meshing_queue.drain(..).collect();
    all_chunks.extend(world.priority_meshing_queue.drain(..));

    
    for _ in 0..config.max_meshes_per_frame {
        // Simple FIFO processing
        let coord = if !all_chunks.is_empty() {
            all_chunks.remove(0)
        } else {
            break;
        };

        // Require all 26 neighbors for proper normal sampling
        let neighbors_loaded = coord
            .all_neighbors()
            .iter()
            .all(|&neighbor| world.get_chunk(neighbor).is_some());

        if !neighbors_loaded {
            // Put chunk back for later processing if neighbors aren't ready
            all_chunks.push(coord); // Put back at end of queue
            continue;
        }

        if let Some(chunk) = world.get_chunk(coord) {
            
            let opaque_mesh =
                generate_chunk_mesh(chunk, &world, &material_registry, &rendering_config);
            let transparent_meshes = generate_transparent_chunk_meshes_by_layer(
                chunk,
                &world,
                &material_registry,
                &rendering_config,
            );

            // Despawn existing meshes for this chunk
            if let Some(existing_entity) = existing_opaque_map.get(&coord) {
                commands.entity(*existing_entity).despawn();
            }
            if let Some(existing_entities) = existing_transparent_map.get(&coord) {
                for &entity in existing_entities {
                    commands.entity(entity).despawn();
                }
            }

            // Spawn opaque mesh if it has geometry
            if let Some(mesh) = opaque_mesh {
                let mesh_handle = meshes.add(mesh);
                let material_handle = materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    ..default()
                });

                commands.spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material_handle),
                    Transform::from_translation(
                        coord.to_world_pos_with_size(rendering_config.chunk_size),
                    ),
                    ChunkMesh::new(coord),
                    OpaqueMesh { coord },
                ));
            }

            // Spawn separate transparent mesh entities for each layer to allow proper sorting
            for (layer_offset, mesh) in transparent_meshes {
                let mesh_handle = meshes.add(mesh);
                let material_handle = materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                });

                // Position each subchunk at its center in world coordinates for better sorting
                let subchunk_world_center =
                    coord.to_world_pos_with_size(rendering_config.chunk_size) + layer_offset;
                let layer_translation = subchunk_world_center;
                commands.spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material_handle),
                    Transform::from_translation(layer_translation),
                    ChunkMesh::new(coord),
                    TransparentMesh { coord },
                ));
            }
        }
    }

    // Put any remaining chunks back into the regular queue for next frame
    for coord in all_chunks {
        world.meshing_queue.push_back(coord);
    }
}

pub fn voxel_tint_system(
    camera_query: Query<&GlobalTransform, With<PlayerCamera>>,
    world: Res<VoxelWorld>,
    material_registry: Res<MaterialRegistry>,
    mut tint_state: ResMut<VoxelTintState>,
    time: Res<Time>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let camera_pos = camera_transform.translation();
    let voxel = world.get_voxel_at_world_pos(camera_pos);

    if let Some(chunk) = world.get_chunk_at_world_pos(camera_pos) {
        if let Some(material_name) = chunk.get_material_name(voxel.material_id) {
            let material = material_registry.get(material_name);

            // Only apply tint for non-air, non-solid voxels (like water)
            if material_name != "air" && !material.is_solid() {
                println!("Applying tint to voxel: {}", material_name);
                let base_color = material.get_color();
                // Use alpha to determine tint strength
                let alpha = base_color.alpha();
                tint_state.target_tint = base_color;
                tint_state.tint_strength = alpha * 0.8; // Scale down for subtlety
            } else {
                tint_state.target_tint = Color::NONE;
                tint_state.tint_strength = 0.0;
            }
        } else {
            tint_state.target_tint = Color::NONE;
            tint_state.tint_strength = 0.0;
        }
    } else {
        tint_state.target_tint = Color::NONE;
        tint_state.tint_strength = 0.0;
    }

    // Smoothly interpolate towards target tint
    let lerp_speed = 5.0;
    let dt = time.delta_secs();

    let current_srgba = tint_state.current_tint.to_srgba();
    let target_srgba = tint_state.target_tint.to_srgba();

    tint_state.current_tint = Color::srgba(
        current_srgba.red + (target_srgba.red - current_srgba.red) * lerp_speed * dt,
        current_srgba.green + (target_srgba.green - current_srgba.green) * lerp_speed * dt,
        current_srgba.blue + (target_srgba.blue - current_srgba.blue) * lerp_speed * dt,
        current_srgba.alpha + (target_srgba.alpha - current_srgba.alpha) * lerp_speed * dt,
    );
}

#[derive(Component)]
pub struct VoxelTintOverlay;

pub fn setup_voxel_tint_overlay(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::NONE),
        GlobalZIndex(500), // Below crosshair but above everything else
        VoxelTintOverlay,
    ));
}

pub fn update_voxel_tint_overlay(
    tint_state: Res<VoxelTintState>,
    mut tint_overlay_query: Query<&mut BackgroundColor, With<VoxelTintOverlay>>,
) {
    if let Ok(mut background_color) = tint_overlay_query.get_single_mut() {
        let tint_srgba = tint_state.current_tint.to_srgba();
        let final_alpha = tint_srgba.alpha * tint_state.tint_strength;

        background_color.0 = Color::srgba(
            tint_srgba.red,
            tint_srgba.green,
            tint_srgba.blue,
            final_alpha,
        );
    }
}

pub fn voxel_interaction_system(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&GlobalTransform, With<PlayerCamera>>,
    mut world: ResMut<VoxelWorld>,
    mut editing_config: ResMut<VoxelEditingConfig>,
    mut physics_config: ResMut<PlayerPhysicsConfig>,
    mut inventory: ResMut<Inventory>,
    material_registry: Res<MaterialRegistry>,
    config: Res<crate::config::GameConfig>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    // Handle brush configuration changes
    if keyboard.just_pressed(KeyCode::KeyB) {
        // Toggle brush shape
        editing_config.brush_shape = match editing_config.brush_shape {
            BrushShape::Ball => BrushShape::Cube,
            BrushShape::Cube => BrushShape::Ball,
        };
        println!("Brush shape: {:?}", editing_config.brush_shape);
    }

    if keyboard.just_pressed(KeyCode::BracketLeft) {
        // Decrease brush size
        editing_config.brush_radius = (editing_config.brush_radius - 0.5).max(0.5);
        println!("Brush radius: {}", editing_config.brush_radius);
    }

    if keyboard.just_pressed(KeyCode::BracketRight) {
        // Increase brush size
        editing_config.brush_radius = (editing_config.brush_radius + 0.5).min(10.0);
        println!("Brush radius: {}", editing_config.brush_radius);
    }

    // Player physics configuration controls
    if keyboard.just_pressed(KeyCode::KeyP) {
        // Toggle collision mode
        physics_config.collision_mode = match physics_config.collision_mode {
            CollisionMode::Basic => CollisionMode::Capsule,
            CollisionMode::Capsule => CollisionMode::Basic,
        };
        println!("Collision mode: {:?}", physics_config.collision_mode);
    }

    if keyboard.just_pressed(KeyCode::Equal) {
        // Increase player size
        physics_config.width = (physics_config.width + 0.2).min(3.0);
        physics_config.height = (physics_config.height + 0.4).min(6.0);
        let eye_height = physics_config.height * 0.8;
        println!(
            "Player size: {}x{} (eye height: {:.1})",
            physics_config.width, physics_config.height, eye_height
        );
    }

    if keyboard.just_pressed(KeyCode::Minus) {
        // Decrease player size
        physics_config.width = (physics_config.width - 0.2).max(0.4);
        physics_config.height = (physics_config.height - 0.4).max(1.0);
        let eye_height = physics_config.height * 0.8;
        println!(
            "Player size: {}x{} (eye height: {:.1})",
            physics_config.width, physics_config.height, eye_height
        );
    }

    if mouse.just_pressed(MouseButton::Right) || mouse.just_pressed(MouseButton::Left) {
        let ray_origin = camera_transform.translation();
        let ray_direction = camera_transform.forward().as_vec3();

        if let Some((hit_pos, place_pos)) = cast_voxel_ray(
            &world,
            ray_origin,
            ray_direction,
            editing_config.reach_distance,
            &material_registry,
            &config,
        ) {
            if mouse.just_pressed(MouseButton::Left) {
                // Remove voxels in brush area and add to inventory
                apply_brush_with_inventory(
                    &mut world,
                    hit_pos,
                    &editing_config,
                    &mut inventory,
                    &material_registry,
                    true,
                );
            } else if mouse.just_pressed(MouseButton::Right) {
                // Get material from current inventory selection or fallback to number keys
                let material_name = {
                    let selected_slot = inventory.get_selected_slot();
                    if !selected_slot.is_empty() {
                        selected_slot.material_name.clone()
                    } else if keyboard.pressed(KeyCode::Digit1) {
                        "stone".to_string()
                    } else if keyboard.pressed(KeyCode::Digit2) {
                        "dirt".to_string()
                    } else if keyboard.pressed(KeyCode::Digit3) {
                        "grass".to_string()
                    } else if keyboard.pressed(KeyCode::Digit4) {
                        "water".to_string()
                    } else if keyboard.pressed(KeyCode::Digit5) {
                        "glass".to_string()
                    } else if keyboard.pressed(KeyCode::Digit6) {
                        "murky_water".to_string()
                    } else {
                        "stone".to_string()
                    }
                };

                // Calculate how many voxels will be placed
                let voxel_count = calculate_brush_voxel_count(&editing_config);

                // Check if we have enough material in inventory
                if inventory.has_material(&material_name, voxel_count) {
                    // Remove material from inventory and place voxels
                    inventory.remove_material(&material_name, voxel_count);
                    apply_brush_with_material(
                        &mut world,
                        place_pos,
                        &editing_config,
                        &material_name,
                    );
                } else {
                    println!(
                        "Not enough {} in inventory! Have: {}, Need: {}",
                        material_name,
                        inventory.get_material_count(&material_name),
                        voxel_count
                    );
                }
            }
        }
    }
}

fn apply_brush(world: &mut VoxelWorld, center: Vec3, config: &VoxelEditingConfig, remove: bool) {
    if remove {
        apply_brush_with_material(world, center, config, "air");
    }
}

fn apply_brush_with_material(
    world: &mut VoxelWorld,
    center: Vec3,
    config: &VoxelEditingConfig,
    material_name: &str,
) {
    match config.brush_shape {
        BrushShape::Ball => {
            apply_ball_brush_with_material(world, center, config.brush_radius, material_name)
        }
        BrushShape::Cube => {
            apply_cube_brush_with_material(world, center, config.brush_radius, material_name)
        }
    }
}

fn apply_ball_brush_with_material(
    world: &mut VoxelWorld,
    center: Vec3,
    radius: f32,
    material_name: &str,
) {
    let radius_squared = radius * radius;
    let min_bounds = center - Vec3::splat(radius);
    let max_bounds = center + Vec3::splat(radius);

    let mut modified_chunks = std::collections::HashSet::new();

    // Iterate through all voxels in the bounding box
    for x in (min_bounds.x.floor() as i32)..=(max_bounds.x.ceil() as i32) {
        for y in (min_bounds.y.floor() as i32)..=(max_bounds.y.ceil() as i32) {
            for z in (min_bounds.z.floor() as i32)..=(max_bounds.z.ceil() as i32) {
                let voxel_pos = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                let distance_squared = (voxel_pos - center).length_squared();

                if distance_squared <= radius_squared {
                    // Get chunk and set voxel by material name
                    if let Some(chunk) = world.get_chunk_at_world_pos_mut(voxel_pos) {
                        let chunk_coord = chunk.coord;
                        let chunk_world_pos = chunk.coord.to_world_pos_with_size(chunk.chunk_size);
                        let local_pos = voxel_pos - chunk_world_pos;
                        let x = local_pos.x as usize;
                        let y = local_pos.y as usize;
                        let z = local_pos.z as usize;

                        if chunk.set_voxel_by_material(x, y, z, material_name) {
                            modified_chunks.insert(chunk_coord);
                        }
                    }
                }
            }
        }
    }

    // Mark all modified chunks and their neighbors for remeshing
    for chunk_coord in modified_chunks {
        world.mark_chunk_and_neighbors_for_remesh(chunk_coord);
    }
}

fn apply_cube_brush_with_material(
    world: &mut VoxelWorld,
    center: Vec3,
    radius: f32,
    material_name: &str,
) {
    let min_bounds = center - Vec3::splat(radius);
    let max_bounds = center + Vec3::splat(radius);

    let mut modified_chunks = std::collections::HashSet::new();

    // Iterate through all voxels in the cube
    for x in (min_bounds.x.floor() as i32)..=(max_bounds.x.ceil() as i32) {
        for y in (min_bounds.y.floor() as i32)..=(max_bounds.y.ceil() as i32) {
            for z in (min_bounds.z.floor() as i32)..=(max_bounds.z.ceil() as i32) {
                let voxel_pos = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                // Get chunk and set voxel by material name
                if let Some(chunk) = world.get_chunk_at_world_pos_mut(voxel_pos) {
                    let chunk_coord = chunk.coord;
                    let chunk_world_pos = chunk.coord.to_world_pos_with_size(chunk.chunk_size);
                    let local_pos = voxel_pos - chunk_world_pos;
                    let x = local_pos.x as usize;
                    let y = local_pos.y as usize;
                    let z = local_pos.z as usize;

                    if chunk.set_voxel_by_material(x, y, z, material_name) {
                        modified_chunks.insert(chunk_coord);
                    }
                }
            }
        }
    }

    // Mark all modified chunks and their neighbors for remeshing
    for chunk_coord in modified_chunks {
        world.mark_chunk_and_neighbors_for_remesh(chunk_coord);
    }
}

fn generate_chunk_mesh(
    chunk: &crate::chunk::ChunkData,
    world: &VoxelWorld,
    material_registry: &MaterialRegistry,
    rendering_config: &RenderingConfig,
) -> Option<Mesh> {
    generate_chunk_mesh_filtered(chunk, world, material_registry, rendering_config, false)
}

fn generate_transparent_chunk_mesh(
    chunk: &crate::chunk::ChunkData,
    world: &VoxelWorld,
    material_registry: &MaterialRegistry,
    rendering_config: &RenderingConfig,
) -> Option<Mesh> {
    generate_chunk_mesh_filtered(chunk, world, material_registry, rendering_config, true)
}

fn generate_transparent_chunk_meshes_by_layer(
    chunk: &crate::chunk::ChunkData,
    world: &VoxelWorld,
    material_registry: &MaterialRegistry,
    rendering_config: &RenderingConfig,
) -> Vec<(Vec3, Mesh)> {
    let mut subchunk_meshes = Vec::new();
    let subchunk_size = rendering_config.transparency_chunk_size;

    // Calculate how many subchunks fit in each dimension
    let subchunks_per_axis = (chunk.chunk_size + subchunk_size - 1) / subchunk_size;

    // Process each NxNxN subregion
    for sx in 0..subchunks_per_axis {
        for sy in 0..subchunks_per_axis {
            for sz in 0..subchunks_per_axis {
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                let mut normals = Vec::new();
                let mut colors = Vec::new();

                // Calculate bounds for this subchunk (don't let it span chunk boundaries)
                let start_x = sx * subchunk_size;
                let start_y = sy * subchunk_size;
                let start_z = sz * subchunk_size;
                let end_x = ((sx + 1) * subchunk_size).min(chunk.chunk_size);
                let end_y = ((sy + 1) * subchunk_size).min(chunk.chunk_size);
                let end_z = ((sz + 1) * subchunk_size).min(chunk.chunk_size);

                // Calculate subchunk center for vertex adjustment
                let subchunk_center = Vec3::new(
                    (start_x + end_x) as f32 / 2.0,
                    (start_y + end_y) as f32 / 2.0,
                    (start_z + end_z) as f32 / 2.0,
                );

                // Collect all transparent voxels in this subchunk
                for x in start_x..end_x {
                    for y in start_y..end_y {
                        for z in start_z..end_z {
                            if let Some(voxel) = chunk.get_voxel(x, y, z) {
                                if let Some(material_name) =
                                    chunk.get_material_name(voxel.material_id)
                                {
                                    let material = material_registry.get(material_name);

                                    // Only include truly transparent materials
                                    let is_truly_transparent = !material.is_solid()
                                        && material.is_transparent()
                                        && material_name != "air";

                                    if is_truly_transparent {
                                        // Use original chunk-relative position for neighbor checking
                                        let chunk_relative_pos =
                                            Vec3::new(x as f32, y as f32, z as f32);

                                        // But adjust vertex positions to be relative to subchunk center
                                        let vertex_offset = chunk_relative_pos - subchunk_center;
                                        add_voxel_faces_with_offset(
                                            &mut vertices,
                                            &mut indices,
                                            &mut normals,
                                            &mut colors,
                                            chunk_relative_pos, // For neighbor checking
                                            vertex_offset,      // For vertex positioning
                                            voxel,
                                            chunk,
                                            world,
                                            material_registry,
                                            rendering_config,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // Create mesh for this subchunk if it has geometry
                if !vertices.is_empty() {
                    let mut mesh = Mesh::new(
                        bevy::render::render_resource::PrimitiveTopology::TriangleList,
                        bevy::render::render_asset::RenderAssetUsages::default(),
                    );
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
                    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

                    // Return the subchunk center for positioning
                    subchunk_meshes.push((subchunk_center, mesh));
                }
            }
        }
    }

    subchunk_meshes
}

fn generate_chunk_mesh_filtered(
    chunk: &crate::chunk::ChunkData,
    world: &VoxelWorld,
    material_registry: &MaterialRegistry,
    rendering_config: &RenderingConfig,
    transparent_only: bool,
) -> Option<Mesh> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();

    let mesh_type = if transparent_only {
        "transparent"
    } else {
        "opaque"
    };

    for x in 0..chunk.chunk_size {
        for y in 0..chunk.chunk_size {
            for z in 0..chunk.chunk_size {
                if let Some(voxel) = chunk.get_voxel(x, y, z) {
                    if let Some(material_name) = chunk.get_material_name(voxel.material_id) {
                        let material = material_registry.get(material_name);

                        // Only include truly transparent materials (not solid, like water/glass)
                        // in transparent mesh. Semi-transparent solids like leaves go in opaque mesh.
                        // Exclude air from transparent mesh entirely.
                        let is_truly_transparent = !material.is_solid()
                            && material.is_transparent()
                            && material_name != "air";

                        // Skip if material doesn't match the filter
                        if transparent_only != is_truly_transparent {
                            continue;
                        }

                        // For opaque mesh, include all solid materials (even if semi-transparent)
                        if !transparent_only && !material.is_solid() {
                            continue;
                        }
                    } else {
                        continue;
                    }

                    let local_pos = Vec3::new(x as f32, y as f32, z as f32);
                    add_voxel_faces(
                        &mut vertices,
                        &mut indices,
                        &mut normals,
                        &mut colors,
                        local_pos,
                        voxel,
                        chunk,
                        world,
                        material_registry,
                        rendering_config,
                    );
                }
            }
        }
    }

    if vertices.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    Some(mesh)
}

fn add_voxel_faces(
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    pos: Vec3,
    voxel: Voxel,
    chunk: &crate::chunk::ChunkData,
    world: &VoxelWorld,
    material_registry: &MaterialRegistry,
    rendering_config: &RenderingConfig,
) {
    let material_name = chunk
        .get_material_name(voxel.material_id)
        .map(|s| s.as_str())
        .unwrap_or("unknown");
    let material = material_registry.get(material_name);

    // Create a deterministic seed based on world position for consistent color variation
    let world_pos = chunk.coord.to_world_pos_with_size(chunk.chunk_size) + pos;
    let x = world_pos.x as i32 as u32;
    let y = world_pos.y as i32 as u32;
    let z = world_pos.z as i32 as u32;

    // Use a proper 3D hash function that ensures good distribution across all dimensions
    let mut seed = x as u64;
    seed = seed.wrapping_mul(0x9e3779b97f4a7c15_u64); // Golden ratio hash
    seed ^= (y as u64) << 16;
    seed = seed.wrapping_mul(0x9e3779b97f4a7c15_u64);
    seed ^= (z as u64) << 32;
    seed = seed.wrapping_mul(0xc6a4a7935bd1e995_u64); // Additional mixing
    seed ^= seed >> 32;

    let mut rng = StdRng::seed_from_u64(seed);
    let varied_color = material.get_varied_color(&mut rng);
    let color_array = [
        varied_color.to_srgba().red,
        varied_color.to_srgba().green,
        varied_color.to_srgba().blue,
        varied_color.to_srgba().alpha,
    ];

    let faces = [
        // +X face
        (
            Vec3::X,
            [
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
                [1.0, 0.0, 1.0],
            ],
        ),
        // -X face
        (
            Vec3::NEG_X,
            [
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
        ),
        // +Y face
        (
            Vec3::Y,
            [
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
            ],
        ),
        // -Y face
        (
            Vec3::NEG_Y,
            [
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
            ],
        ),
        // +Z face
        (
            Vec3::Z,
            [
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 1.0],
            ],
        ),
        // -Z face
        (
            Vec3::NEG_Z,
            [
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
        ),
    ];

    for (normal, face_vertices) in faces {
        let neighbor_pos = pos + normal;
        let neighbor_voxel = get_voxel_with_neighbor_check(chunk, world, neighbor_pos);

        // Get neighbor material info
        let neighbor_material_name = if let Some(neighbor_chunk) = world.get_chunk_at_world_pos(
            chunk.coord.to_world_pos_with_size(chunk.chunk_size) + neighbor_pos,
        ) {
            neighbor_chunk
                .get_material_name(neighbor_voxel.material_id)
                .map(|s| s.as_str())
        } else {
            Some("air") // Outside chunks are air
        };

        let neighbor_material = if let Some(name) = neighbor_material_name {
            material_registry.get(name)
        } else {
            material_registry.get("air")
        };

        // Face culling logic:
        // - Always render faces adjacent to air
        // - For opaque materials, only render faces adjacent to transparent materials or air
        // - For transparent materials, render faces at any material boundary
        let has_air_neighbor = neighbor_material_name == Some("air");
        let materials_different = material != neighbor_material;
        let material_is_opaque = material.is_solid() && !material.is_transparent();
        let neighbor_truly_transparent = !neighbor_material.is_solid()
            && neighbor_material.is_transparent()
            && neighbor_material_name != Some("air");

        let should_render_face = has_air_neighbor
            || (material_is_opaque && neighbor_truly_transparent)
            || (!material_is_opaque && materials_different);

        if should_render_face {
            let base_index = vertices.len() as u32;

            let face_normal = if rendering_config.use_basic_normals {
                // Use basic face normals with special handling for transparent horizontal faces
                calculate_basic_normal(normal, material)
            } else {
                // Calculate face center in world coordinates - this ensures adjacent faces
                // across chunk boundaries sample from the exact same world position
                let world_voxel_center =
                    chunk.coord.to_world_pos_with_size(chunk.chunk_size) + pos + Vec3::splat(0.5);
                let world_face_center = world_voxel_center + normal * 0.5;

                // Use world coordinates for normal calculation to ensure consistency
                calculate_smooth_normal(
                    chunk,
                    world,
                    world_face_center,
                    material_registry,
                    rendering_config,
                )
            };

            for vertex in face_vertices {
                let vertex_pos = Vec3::new(pos.x + vertex[0], pos.y + vertex[1], pos.z + vertex[2]);

                vertices.push([vertex_pos.x, vertex_pos.y, vertex_pos.z]);
                normals.push([face_normal.x, face_normal.y, face_normal.z]);
                colors.push(color_array);
            }

            // Add front-facing triangles
            indices.extend_from_slice(&[
                base_index,
                base_index + 1,
                base_index + 2,
                base_index,
                base_index + 2,
                base_index + 3,
            ]);

            // For transparent-air boundaries and truly transparent boundaries,
            // add back-facing triangles with appropriate material color
            let material_truly_transparent = !material.is_solid() && material.is_transparent();
            let neighbor_truly_transparent =
                !neighbor_material.is_solid() && neighbor_material.is_transparent();
            let has_transparent_air_boundary = material_truly_transparent && has_air_neighbor;

            if materials_different
                && ((material_truly_transparent && neighbor_truly_transparent)
                    || has_transparent_air_boundary)
            {
                let back_base_index = vertices.len() as u32;

                // Add the same vertices again but with appropriate back-face normal
                for vertex in face_vertices {
                    let vertex_pos =
                        Vec3::new(pos.x + vertex[0], pos.y + vertex[1], pos.z + vertex[2]);
                    vertices.push([vertex_pos.x, vertex_pos.y, vertex_pos.z]);

                    // For transparent-air boundaries on horizontal faces, always use Y-up normal
                    // to match the transparent-to-transparent behavior
                    let back_face_normal = if has_transparent_air_boundary
                        && (normal == Vec3::Y || normal == Vec3::NEG_Y)
                    {
                        Vec3::Y
                    } else {
                        -face_normal
                    };

                    normals.push([back_face_normal.x, back_face_normal.y, back_face_normal.z]);
                }

                // Use appropriate color for back faces
                let back_face_color = if has_transparent_air_boundary {
                    // For transparent-air boundaries, use the transparent material color on both sides
                    material.get_varied_color(&mut rng)
                } else {
                    // For transparent-transparent boundaries, use the neighbor material color
                    neighbor_material.get_varied_color(&mut rng)
                };
                let back_face_color_array = [
                    back_face_color.to_srgba().red,
                    back_face_color.to_srgba().green,
                    back_face_color.to_srgba().blue,
                    back_face_color.to_srgba().alpha,
                ];

                for _ in 0..4 {
                    colors.push(back_face_color_array);
                }

                // Add back-facing triangles (reversed winding order)
                indices.extend_from_slice(&[
                    back_base_index,
                    back_base_index + 2,
                    back_base_index + 1,
                    back_base_index,
                    back_base_index + 3,
                    back_base_index + 2,
                ]);
            }
        }
    }
}

fn add_voxel_faces_with_offset(
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    pos: Vec3,        // Original chunk-relative position for neighbor checking
    vertex_pos: Vec3, // Adjusted position for vertex coordinates
    voxel: Voxel,
    chunk: &crate::chunk::ChunkData,
    world: &VoxelWorld,
    material_registry: &MaterialRegistry,
    rendering_config: &RenderingConfig,
) {
    let material_name = chunk
        .get_material_name(voxel.material_id)
        .map(|s| s.as_str())
        .unwrap_or("unknown");
    let material = material_registry.get(material_name);

    // Create a deterministic seed based on world position for consistent color variation
    let world_pos = chunk.coord.to_world_pos_with_size(chunk.chunk_size) + pos; // Use original pos for color consistency
    let x = world_pos.x as i32 as u32;
    let y = world_pos.y as i32 as u32;
    let z = world_pos.z as i32 as u32;

    // Use a proper 3D hash function that ensures good distribution across all dimensions
    let mut seed = x as u64;
    seed = seed.wrapping_mul(0x9e3779b97f4a7c15_u64); // Golden ratio hash
    seed ^= (y as u64) << 16;
    seed = seed.wrapping_mul(0x9e3779b97f4a7c15_u64);
    seed ^= (z as u64) << 32;
    seed = seed.wrapping_mul(0xc6a4a7935bd1e995_u64); // Additional mixing
    seed ^= seed >> 32;

    let mut rng = StdRng::seed_from_u64(seed);
    let varied_color = material.get_varied_color(&mut rng);
    let color_array = [
        varied_color.to_srgba().red,
        varied_color.to_srgba().green,
        varied_color.to_srgba().blue,
        varied_color.to_srgba().alpha,
    ];

    let faces = [
        // +X face
        (
            Vec3::X,
            [
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
                [1.0, 0.0, 1.0],
            ],
        ),
        // -X face
        (
            Vec3::NEG_X,
            [
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
        ),
        // +Y face
        (
            Vec3::Y,
            [
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
            ],
        ),
        // -Y face
        (
            Vec3::NEG_Y,
            [
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
            ],
        ),
        // +Z face
        (
            Vec3::Z,
            [
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 1.0],
            ],
        ),
        // -Z face
        (
            Vec3::NEG_Z,
            [
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
        ),
    ];

    for (normal, face_vertices) in faces {
        let neighbor_pos = pos + normal; // Use original pos for neighbor checking
        let neighbor_voxel = get_voxel_with_neighbor_check(chunk, world, neighbor_pos);

        // Get neighbor material info
        let neighbor_material_name = if let Some(neighbor_chunk) = world.get_chunk_at_world_pos(
            chunk.coord.to_world_pos_with_size(chunk.chunk_size) + neighbor_pos,
        ) {
            neighbor_chunk
                .get_material_name(neighbor_voxel.material_id)
                .map(|s| s.as_str())
        } else {
            Some("air") // Outside chunks are air
        };

        let neighbor_material = if let Some(name) = neighbor_material_name {
            material_registry.get(name)
        } else {
            material_registry.get("air")
        };

        // Face culling logic (same as original)
        let has_air_neighbor = neighbor_material_name == Some("air");
        let materials_different = material != neighbor_material;
        let material_is_opaque = material.is_solid() && !material.is_transparent();
        let neighbor_truly_transparent = !neighbor_material.is_solid()
            && neighbor_material.is_transparent()
            && neighbor_material_name != Some("air");

        let should_render_face = has_air_neighbor
            || (material_is_opaque && neighbor_truly_transparent)
            || (!material_is_opaque && materials_different);

        if should_render_face {
            let base_index = vertices.len() as u32;

            let face_normal = if rendering_config.use_basic_normals {
                calculate_basic_normal(normal, material)
            } else {
                let world_voxel_center =
                    chunk.coord.to_world_pos_with_size(chunk.chunk_size) + pos + Vec3::splat(0.5);
                let world_face_center = world_voxel_center + normal * 0.5;
                calculate_smooth_normal(
                    chunk,
                    world,
                    world_face_center,
                    material_registry,
                    rendering_config,
                )
            };

            for vertex in face_vertices {
                // Use vertex_pos (offset position) for actual vertex coordinates
                let vertex_pos_final = Vec3::new(
                    vertex_pos.x + vertex[0],
                    vertex_pos.y + vertex[1],
                    vertex_pos.z + vertex[2],
                );

                vertices.push([vertex_pos_final.x, vertex_pos_final.y, vertex_pos_final.z]);
                normals.push([face_normal.x, face_normal.y, face_normal.z]);
                colors.push(color_array);
            }

            // Add front-facing triangles
            indices.extend_from_slice(&[
                base_index,
                base_index + 1,
                base_index + 2,
                base_index,
                base_index + 2,
                base_index + 3,
            ]);

            // For transparent-air boundaries and truly transparent boundaries,
            // add back-facing triangles with appropriate material color
            let material_truly_transparent = !material.is_solid() && material.is_transparent();
            let neighbor_truly_transparent =
                !neighbor_material.is_solid() && neighbor_material.is_transparent();
            let has_transparent_air_boundary = material_truly_transparent && has_air_neighbor;

            if materials_different
                && ((material_truly_transparent && neighbor_truly_transparent)
                    || has_transparent_air_boundary)
            {
                let back_base_index = vertices.len() as u32;

                // Add the same vertices again but with appropriate back-face normal
                for vertex in face_vertices {
                    let vertex_pos_final = Vec3::new(
                        vertex_pos.x + vertex[0],
                        vertex_pos.y + vertex[1],
                        vertex_pos.z + vertex[2],
                    );
                    vertices.push([vertex_pos_final.x, vertex_pos_final.y, vertex_pos_final.z]);

                    // For transparent-air boundaries on horizontal faces, always use Y-up normal
                    // to match the transparent-to-transparent behavior
                    let back_face_normal = if has_transparent_air_boundary
                        && (normal == Vec3::Y || normal == Vec3::NEG_Y)
                    {
                        Vec3::Y
                    } else {
                        -face_normal
                    };

                    normals.push([back_face_normal.x, back_face_normal.y, back_face_normal.z]);
                }

                // Use appropriate color for back faces
                let back_face_color = if has_transparent_air_boundary {
                    material.get_varied_color(&mut rng)
                } else {
                    neighbor_material.get_varied_color(&mut rng)
                };
                let back_face_color_array = [
                    back_face_color.to_srgba().red,
                    back_face_color.to_srgba().green,
                    back_face_color.to_srgba().blue,
                    back_face_color.to_srgba().alpha,
                ];

                for _ in 0..4 {
                    colors.push(back_face_color_array);
                }

                // Add back-facing triangles (reversed winding order)
                indices.extend_from_slice(&[
                    back_base_index,
                    back_base_index + 2,
                    back_base_index + 1,
                    back_base_index,
                    back_base_index + 3,
                    back_base_index + 2,
                ]);
            }
        }
    }
}

fn calculate_smooth_normal(
    _chunk: &crate::chunk::ChunkData,
    world: &VoxelWorld,
    world_sample_pos: Vec3,
    material_registry: &MaterialRegistry,
    rendering_config: &RenderingConfig,
) -> Vec3 {
    // Use configurable radius for sampling
    let sampling_radius = rendering_config.normal_sampling_radius;

    // Find center of mass of all air cells in the sampling radius
    // Work entirely in world coordinates to ensure consistency across chunk boundaries
    let mut air_center_of_mass = Vec3::ZERO;
    let mut air_cell_count = 0;

    // Sample all cells in a cube around the world position
    for dx in -sampling_radius..=sampling_radius {
        for dy in -sampling_radius..=sampling_radius {
            for dz in -sampling_radius..=sampling_radius {
                let world_sample = world_sample_pos + Vec3::new(dx as f32, dy as f32, dz as f32);
                let density = get_world_voxel_density(world, world_sample, material_registry);

                // If this is an air cell (density = 0), add it to center of mass calculation
                if density < 0.5 {
                    air_center_of_mass += world_sample;
                    air_cell_count += 1;
                }
            }
        }
    }

    // Calculate the direction from our position toward the center of mass of air cells
    if air_cell_count > 0 {
        air_center_of_mass /= air_cell_count as f32;
        let direction_to_air = air_center_of_mass - world_sample_pos;

        // Return normalized direction as the surface normal
        if direction_to_air.length() > 0.001 {
            direction_to_air.normalize()
        } else {
            Vec3::Y // Default fallback normal
        }
    } else {
        // If no air cells found, use upward normal
        Vec3::Y
    }
}

fn calculate_basic_normal(face_normal: Vec3, material: &crate::voxel::Material) -> Vec3 {
    // For transparent materials on horizontal faces, always use Y-up normal
    if material.is_transparent() && !material.is_solid() {
        if face_normal == Vec3::Y || face_normal == Vec3::NEG_Y {
            return Vec3::Y;
        }
    }

    // For all other cases, use the face normal directly
    face_normal
}

/// Get voxel density at a world position - used for consistent sampling across chunk boundaries
fn get_world_voxel_density(
    world: &VoxelWorld,
    world_pos: Vec3,
    material_registry: &MaterialRegistry,
) -> f32 {
    if let Some(chunk) = world.get_chunk_at_world_pos(world_pos) {
        // Calculate local position within the chunk
        let chunk_world_pos = chunk.coord.to_world_pos_with_size(chunk.chunk_size);
        let local_pos = world_pos - chunk_world_pos;
        let x = local_pos.x as i32;
        let y = local_pos.y as i32;
        let z = local_pos.z as i32;

        // Bounds check and get voxel
        if x >= 0
            && x < chunk.chunk_size as i32
            && y >= 0
            && y < chunk.chunk_size as i32
            && z >= 0
            && z < chunk.chunk_size as i32
        {
            if let Some(voxel) = chunk.get_voxel(x as usize, y as usize, z as usize) {
                if let Some(material_name) = chunk.get_material_name(voxel.material_id) {
                    return if material_registry.get(material_name).is_solid() {
                        1.0
                    } else {
                        0.0
                    };
                }
            }
        }
    }
    0.0 // Default to air if chunk not loaded or voxel not found
}

fn get_voxel_density(
    chunk: &crate::chunk::ChunkData,
    world: &VoxelWorld,
    local_pos: Vec3,
    material_registry: &MaterialRegistry,
) -> f32 {
    let x = local_pos.x as i32;
    let y = local_pos.y as i32;
    let z = local_pos.z as i32;

    // If within current chunk bounds, get from current chunk
    if x >= 0
        && x < chunk.chunk_size as i32
        && y >= 0
        && y < chunk.chunk_size as i32
        && z >= 0
        && z < chunk.chunk_size as i32
    {
        if let Some(voxel) = chunk.get_voxel(x as usize, y as usize, z as usize) {
            if let Some(material_name) = chunk.get_material_name(voxel.material_id) {
                return if material_registry.get(material_name).is_solid() {
                    1.0
                } else {
                    0.0
                };
            }
        }
        return 0.0; // Default to air if voxel not found in current chunk
    }

    // For cross-chunk sampling, get from world and use the correct chunk's material palette
    let world_pos = chunk.coord.to_world_pos_with_size(chunk.chunk_size) + local_pos;
    if let Some(neighbor_chunk) = world.get_chunk_at_world_pos(world_pos) {
        // Calculate local position within the neighbor chunk
        let neighbor_chunk_pos = neighbor_chunk
            .coord
            .to_world_pos_with_size(neighbor_chunk.chunk_size);
        let neighbor_local_pos = world_pos - neighbor_chunk_pos;
        let nx = neighbor_local_pos.x as i32;
        let ny = neighbor_local_pos.y as i32;
        let nz = neighbor_local_pos.z as i32;

        // Bounds check and get voxel directly from the neighbor chunk
        if nx >= 0
            && nx < neighbor_chunk.chunk_size as i32
            && ny >= 0
            && ny < neighbor_chunk.chunk_size as i32
            && nz >= 0
            && nz < neighbor_chunk.chunk_size as i32
        {
            if let Some(voxel) = neighbor_chunk.get_voxel(nx as usize, ny as usize, nz as usize) {
                if let Some(material_name) = neighbor_chunk.get_material_name(voxel.material_id) {
                    if material_registry.get(material_name).is_solid() {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        }
    } else {
        // If neighboring chunk isn't loaded, assume it's air (this is the fallback case)
        0.0
    }
}

fn get_voxel_with_neighbor_check(
    chunk: &crate::chunk::ChunkData,
    world: &VoxelWorld,
    local_pos: Vec3,
) -> crate::voxel::Voxel {
    let x = local_pos.x as i32;
    let y = local_pos.y as i32;
    let z = local_pos.z as i32;

    // If within current chunk bounds, get from current chunk
    if x >= 0
        && x < chunk.chunk_size as i32
        && y >= 0
        && y < chunk.chunk_size as i32
        && z >= 0
        && z < chunk.chunk_size as i32
    {
        return chunk
            .get_voxel(x as usize, y as usize, z as usize)
            .unwrap_or_default();
    }

    // Otherwise, convert to world position and get from world
    let world_pos = chunk.coord.to_world_pos_with_size(chunk.chunk_size) + local_pos;
    world.get_voxel_at_world_pos(world_pos)
}

fn cast_voxel_ray(
    world: &VoxelWorld,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    material_registry: &MaterialRegistry,
    config: &crate::config::GameConfig,
) -> Option<(Vec3, Vec3)> {
    let step_size = config.raycast_step_size;
    let max_steps = (max_distance / step_size) as i32;

    for i in 0..max_steps {
        let current_pos = origin + direction * (i as f32 * step_size);

        if is_voxel_solid_at_pos(world, current_pos, material_registry) {
            let previous_pos = origin + direction * ((i - 1) as f32 * step_size);
            return Some((current_pos, previous_pos));
        }
    }

    None
}

fn apply_brush_with_inventory(
    world: &mut VoxelWorld,
    center: Vec3,
    config: &VoxelEditingConfig,
    inventory: &mut Inventory,
    material_registry: &MaterialRegistry,
    remove: bool,
) {
    if !remove {
        return;
    }

    // Collect materials before removing them
    let mut materials_collected: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();

    match config.brush_shape {
        BrushShape::Ball => {
            collect_materials_from_ball_brush(
                world,
                center,
                config.brush_radius,
                &mut materials_collected,
                material_registry,
            );
            apply_ball_brush_with_material(world, center, config.brush_radius, "air");
        }
        BrushShape::Cube => {
            collect_materials_from_cube_brush(
                world,
                center,
                config.brush_radius,
                &mut materials_collected,
                material_registry,
            );
            apply_cube_brush_with_material(world, center, config.brush_radius, "air");
        }
    }

    // Add collected materials to inventory
    for (material_name, count) in materials_collected {
        if material_name != "air" && count > 0 {
            let added = inventory.add_material(&material_name, count);
            if added < count {
                println!(
                    "Inventory full! Only added {} of {} {}",
                    added, count, material_name
                );
            }
        }
    }
}

fn collect_materials_from_ball_brush(
    world: &VoxelWorld,
    center: Vec3,
    radius: f32,
    materials_collected: &mut std::collections::HashMap<String, u32>,
    material_registry: &MaterialRegistry,
) {
    let radius_squared = radius * radius;
    let min_bounds = center - Vec3::splat(radius);
    let max_bounds = center + Vec3::splat(radius);

    for x in (min_bounds.x.floor() as i32)..=(max_bounds.x.ceil() as i32) {
        for y in (min_bounds.y.floor() as i32)..=(max_bounds.y.ceil() as i32) {
            for z in (min_bounds.z.floor() as i32)..=(max_bounds.z.ceil() as i32) {
                let voxel_pos = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                let distance_squared = (voxel_pos - center).length_squared();

                if distance_squared <= radius_squared {
                    if let Some(chunk) = world.get_chunk_at_world_pos(voxel_pos) {
                        let chunk_world_pos = chunk.coord.to_world_pos_with_size(chunk.chunk_size);
                        let local_pos = voxel_pos - chunk_world_pos;
                        let x = local_pos.x as usize;
                        let y = local_pos.y as usize;
                        let z = local_pos.z as usize;

                        if let Some(voxel) = chunk.get_voxel(x, y, z) {
                            if let Some(material_name) = chunk.get_material_name(voxel.material_id)
                            {
                                let material = material_registry.get(material_name);
                                if material.is_solid() && material_name != "air" {
                                    *materials_collected
                                        .entry(material_name.to_string())
                                        .or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn collect_materials_from_cube_brush(
    world: &VoxelWorld,
    center: Vec3,
    radius: f32,
    materials_collected: &mut std::collections::HashMap<String, u32>,
    material_registry: &MaterialRegistry,
) {
    let min_bounds = center - Vec3::splat(radius);
    let max_bounds = center + Vec3::splat(radius);

    for x in (min_bounds.x.floor() as i32)..=(max_bounds.x.ceil() as i32) {
        for y in (min_bounds.y.floor() as i32)..=(max_bounds.y.ceil() as i32) {
            for z in (min_bounds.z.floor() as i32)..=(max_bounds.z.ceil() as i32) {
                let voxel_pos = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);

                if let Some(chunk) = world.get_chunk_at_world_pos(voxel_pos) {
                    let chunk_world_pos = chunk.coord.to_world_pos_with_size(chunk.chunk_size);
                    let local_pos = voxel_pos - chunk_world_pos;
                    let x = local_pos.x as usize;
                    let y = local_pos.y as usize;
                    let z = local_pos.z as usize;

                    if let Some(voxel) = chunk.get_voxel(x, y, z) {
                        if let Some(material_name) = chunk.get_material_name(voxel.material_id) {
                            let material = material_registry.get(material_name);
                            if material.is_solid() && material_name != "air" {
                                *materials_collected
                                    .entry(material_name.to_string())
                                    .or_insert(0) += 1;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn calculate_brush_voxel_count(config: &VoxelEditingConfig) -> u32 {
    match config.brush_shape {
        BrushShape::Ball => {
            let radius_squared = config.brush_radius * config.brush_radius;
            let min_bounds = -config.brush_radius;
            let max_bounds = config.brush_radius;

            let mut count = 0;
            for x in (min_bounds.floor() as i32)..=(max_bounds.ceil() as i32) {
                for y in (min_bounds.floor() as i32)..=(max_bounds.ceil() as i32) {
                    for z in (min_bounds.floor() as i32)..=(max_bounds.ceil() as i32) {
                        let voxel_pos = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                        let distance_squared = voxel_pos.length_squared();

                        if distance_squared <= radius_squared {
                            count += 1;
                        }
                    }
                }
            }
            count
        }
        BrushShape::Cube => {
            let size = (config.brush_radius * 2.0).ceil() as i32;
            (size * size * size) as u32
        }
    }
}
