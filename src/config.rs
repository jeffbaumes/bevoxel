use bevy::prelude::*;

#[derive(Resource, Clone, Debug)]
pub struct GameConfig {
    pub render_distance: i32,
    pub unload_distance: i32,
    pub max_chunks_per_frame: usize,
    pub max_meshes_per_frame: usize,
    pub raycast_step_size: f32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            render_distance: 8,
            unload_distance: 12,
            max_chunks_per_frame: 2,
            max_meshes_per_frame: 16,
            raycast_step_size: 0.1,
        }
    }
}