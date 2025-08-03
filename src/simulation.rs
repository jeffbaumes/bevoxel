use crate::chunk::ChunkCoord;
use crate::voxel::MaterialRegistry;
use crate::world::VoxelWorld;
use bevy::prelude::*;
use rand::prelude::*;

#[derive(Resource)]
pub struct SimulationConfig {
    pub enabled: bool,
    pub step_interval: f32,
    pub voxel_fraction_per_step: f32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            step_interval: 0.5,             // 0.5 seconds between steps
            voxel_fraction_per_step: 0.003, // Process 0.3% of voxels per chunk each step (roughly 98 out of 32768)
        }
    }
}

#[derive(Resource)]
pub struct SimulationTimer {
    pub timer: Timer,
}

impl Default for SimulationTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        }
    }
}

pub type SimulationStepCallback = fn(&mut VoxelWorld, &MaterialRegistry, Vec3) -> bool;

#[derive(Resource)]
pub struct SimulationCallbacks {
    pub callbacks: Vec<SimulationStepCallback>,
}

impl Default for SimulationCallbacks {
    fn default() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }
}

impl SimulationCallbacks {
    pub fn add_callback(&mut self, callback: SimulationStepCallback) {
        self.callbacks.push(callback);
    }
}

pub fn simulation_timer_system(
    time: Res<Time>,
    mut simulation_timer: ResMut<SimulationTimer>,
    simulation_config: Res<SimulationConfig>,
    mut world: ResMut<VoxelWorld>,
) {
    if !simulation_config.enabled {
        return;
    }

    simulation_timer.timer.tick(time.delta());

    if simulation_timer.timer.just_finished() {
        // Get all loaded chunk coordinates and add them to simulation queue
        let loaded_chunks: Vec<ChunkCoord> = world.chunks.keys().copied().collect();
        
        for chunk_coord in loaded_chunks {
            // Only add if not already in queue to avoid duplicates
            if !world.simulation_queue.contains(&chunk_coord) {
                world.simulation_queue.push_back(chunk_coord);
            }
        }
    }
}

pub fn chunk_simulation_system(
    simulation_config: Res<SimulationConfig>,
    mut world: ResMut<VoxelWorld>,
    registry: Res<MaterialRegistry>,
    callbacks: Res<SimulationCallbacks>,
    config: Res<crate::config::GameConfig>,
) {
    if !simulation_config.enabled {
        return;
    }

    let mut rng = thread_rng();

    // Process limited number of chunks from simulation queue per frame
    for _ in 0..config.max_chunks_simulated_per_frame {
        let chunk_coord = if let Some(coord) = world.simulation_queue.pop_front() {
            coord
        } else {
            break; // No more chunks to process
        };

        // Check if chunk still exists (might have been unloaded)
        if let Some(_chunk) = world.chunks.get(&chunk_coord) {
            let chunk_world_pos = chunk_coord.to_world_pos_with_size(world.chunk_size);

            if simulation_config.voxel_fraction_per_step >= 0.5 {
                // For high fractions (≥50%), iterate through all positions with biased coin flip
                // This avoids duplicate random sampling and ensures exact probability
                for local_x in 0..world.chunk_size {
                    for local_y in 0..world.chunk_size {
                        for local_z in 0..world.chunk_size {
                            // Flip biased coin to decide whether to process this voxel
                            if rng.gen::<f32>() < simulation_config.voxel_fraction_per_step {
                                let world_pos = Vec3::new(
                                    chunk_world_pos.x + local_x as f32,
                                    chunk_world_pos.y + local_y as f32,
                                    chunk_world_pos.z + local_z as f32,
                                );

                                // Run all simulation callbacks on this position
                                for callback in &callbacks.callbacks {
                                    let changed = callback(&mut world, &registry, world_pos);
                                    if changed {
                                        break; // Only one callback should modify a voxel per step
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // For low fractions (<50%), use random sampling to avoid iterating through all voxels
                let chunk_volume = world.chunk_size * world.chunk_size * world.chunk_size;
                let voxels_to_process = (chunk_volume as f32
                    * simulation_config.voxel_fraction_per_step)
                    .round() as usize;

                for _ in 0..voxels_to_process {
                    let local_x = rng.gen_range(0..world.chunk_size);
                    let local_y = rng.gen_range(0..world.chunk_size);
                    let local_z = rng.gen_range(0..world.chunk_size);

                    let world_pos = Vec3::new(
                        chunk_world_pos.x + local_x as f32,
                        chunk_world_pos.y + local_y as f32,
                        chunk_world_pos.z + local_z as f32,
                    );

                    // Run all simulation callbacks on this position
                    for callback in &callbacks.callbacks {
                        let changed = callback(&mut world, &registry, world_pos);
                        if changed {
                            break; // Only one callback should modify a voxel per step
                        }
                    }
                }
            }
        }
    }
}

pub fn setup_simulation_timer(mut commands: Commands, simulation_config: Res<SimulationConfig>) {
    let mut timer = SimulationTimer::default();
    timer.timer = Timer::from_seconds(simulation_config.step_interval, TimerMode::Repeating);
    commands.insert_resource(timer);
}
