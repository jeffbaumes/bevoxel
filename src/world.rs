use bevy::prelude::*;
use ahash::AHashMap;
use std::collections::VecDeque;
use crate::chunk::{ChunkCoord, ChunkData, ChunkMap};
use crate::voxel::Voxel;

#[derive(Clone, Copy, Debug)]
pub enum BrushShape {
    Ball,
    Cube,
}

#[derive(Clone, Debug, Resource)]
pub struct VoxelEditingConfig {
    pub reach_distance: f32,
    pub brush_radius: f32,
    pub brush_shape: BrushShape,
}

impl Default for VoxelEditingConfig {
    fn default() -> Self {
        Self {
            reach_distance: 8.0,  // Increased from player's 5.0
            brush_radius: 2.0,    // 2-voxel radius brush
            brush_shape: BrushShape::Ball,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CollisionMode {
    Basic,        // Simple box collision
    Capsule,      // Capsule collision with step-up
}

#[derive(Clone, Debug, Resource)]
pub struct PlayerPhysicsConfig {
    pub width: f32,
    pub height: f32,
    pub collision_mode: CollisionMode,
    pub step_height: f32,      // Maximum step height for capsule mode
    pub collision_samples: u32, // Number of collision sampling points
}

impl Default for PlayerPhysicsConfig {
    fn default() -> Self {
        Self {
            width: 1.2,   // Current doubled size
            height: 3.6,  // Current doubled size
            collision_mode: CollisionMode::Capsule,
            step_height: 1.0,
            collision_samples: 8, // Default sampling resolution
        }
    }
}

#[derive(Clone, Debug, Resource)]
pub struct RenderingConfig {
    pub normal_sampling_radius: i32,  // Radius for smooth normal calculation
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            normal_sampling_radius: 2,  // Default radius for smooth normals
        }
    }
}

pub const RENDER_DISTANCE: i32 = 8;
pub const UNLOAD_DISTANCE: i32 = 12;

#[derive(Resource)]
pub struct VoxelWorld {
    pub chunks: ChunkMap,
    pub loading_queue: VecDeque<ChunkCoord>,
    pub meshing_queue: VecDeque<ChunkCoord>,
    pub priority_meshing_queue: VecDeque<ChunkCoord>, // For chunks modified by player
    pub player_chunk: Option<ChunkCoord>,
    pub save_path: String,
}

impl Default for VoxelWorld {
    fn default() -> Self {
        Self {
            chunks: AHashMap::default(),
            loading_queue: VecDeque::new(),
            meshing_queue: VecDeque::new(),
            priority_meshing_queue: VecDeque::new(),
            player_chunk: None,
            save_path: "world".to_string(),
        }
    }
}

impl VoxelWorld {
    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<&ChunkData> {
        self.chunks.get(&coord)
    }
    
    pub fn get_chunk_mut(&mut self, coord: ChunkCoord) -> Option<&mut ChunkData> {
        self.chunks.get_mut(&coord)
    }
    
    pub fn get_chunk_at_world_pos(&self, world_pos: Vec3) -> Option<&ChunkData> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        self.get_chunk(chunk_coord)
    }
    
    pub fn get_chunk_at_world_pos_mut(&mut self, world_pos: Vec3) -> Option<&mut ChunkData> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        self.get_chunk_mut(chunk_coord)
    }
    
    pub fn load_chunk(&mut self, coord: ChunkCoord) -> &mut ChunkData {
        if !self.chunks.contains_key(&coord) {
            let mut chunk = ChunkData::new(coord);
            
            if !self.try_load_chunk_from_disk(&mut chunk) {
                // Terrain generation will be handled externally
            }
            
            self.chunks.insert(coord, chunk);
            self.meshing_queue.push_back(coord);
        }
        
        self.chunks.get_mut(&coord).unwrap()
    }
    
    pub fn unload_chunk(&mut self, coord: ChunkCoord) {
        if let Some(chunk) = self.chunks.remove(&coord) {
            if chunk.modified {
                self.save_chunk_to_disk(&chunk);
            }
        }
    }
    
    pub fn get_voxel_at_world_pos(&self, world_pos: Vec3) -> Voxel {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        
        if let Some(chunk) = self.get_chunk(chunk_coord) {
            chunk.get_voxel_world_pos(world_pos).unwrap_or_default()
        } else {
            Voxel::default()
        }
    }
    
    pub fn set_voxel_at_world_pos(&mut self, world_pos: Vec3, voxel: Voxel) -> bool {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        
        if let Some(chunk) = self.get_chunk_mut(chunk_coord) {
            let result = chunk.set_voxel_world_pos(world_pos, voxel);
            if result {
                self.mark_chunk_and_neighbors_for_remesh(chunk_coord);
            }
            result
        } else {
            false
        }
    }

    /// Marks a chunk and all necessary neighbors for remeshing based on normal sampling requirements
    pub fn mark_chunk_and_neighbors_for_remesh(&mut self, chunk_coord: ChunkCoord) {
        // Always mark the modified chunk itself
        if !self.priority_meshing_queue.contains(&chunk_coord) {
            self.priority_meshing_queue.push_back(chunk_coord);
        }
        
        // Mark all 26 neighbors for remeshing since normal calculation
        // samples in all directions and could be affected by this change
        for neighbor_coord in chunk_coord.all_neighbors() {
            if self.chunks.contains_key(&neighbor_coord) && !self.priority_meshing_queue.contains(&neighbor_coord) {
                self.priority_meshing_queue.push_back(neighbor_coord);
            }
        }
    }
    
    pub fn update_player_position(&mut self, player_pos: Vec3) {
        let new_chunk = ChunkCoord::from_world_pos(player_pos);
        
        if self.player_chunk != Some(new_chunk) {
            self.player_chunk = Some(new_chunk);
            self.queue_chunks_for_loading(new_chunk);
            self.unload_distant_chunks(new_chunk);
        }
    }
    
    fn queue_chunks_for_loading(&mut self, center: ChunkCoord) {
        for dx in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for dy in -RENDER_DISTANCE..=RENDER_DISTANCE {
                for dz in -RENDER_DISTANCE..=RENDER_DISTANCE {
                    let coord = ChunkCoord::new(
                        center.x + dx,
                        center.y + dy,
                        center.z + dz,
                    );
                    
                    let distance_sq = dx * dx + dy * dy + dz * dz;
                    if distance_sq <= RENDER_DISTANCE * RENDER_DISTANCE {
                        if !self.chunks.contains_key(&coord) 
                            && !self.loading_queue.contains(&coord) {
                            self.loading_queue.push_back(coord);
                        }
                    }
                }
            }
        }
    }
    
    fn unload_distant_chunks(&mut self, center: ChunkCoord) {
        let mut chunks_to_unload = Vec::new();
        
        for &coord in self.chunks.keys() {
            let dx = (coord.x - center.x).abs();
            let dy = (coord.y - center.y).abs();
            let dz = (coord.z - center.z).abs();
            
            let max_distance = dx.max(dy).max(dz);
            if max_distance > UNLOAD_DISTANCE {
                chunks_to_unload.push(coord);
            }
        }
        
        for coord in chunks_to_unload {
            self.unload_chunk(coord);
        }
    }
    
    fn try_load_chunk_from_disk(&self, _chunk: &mut ChunkData) -> bool {
        false
    }
    
    fn save_chunk_to_disk(&self, _chunk: &ChunkData) {
    }
}