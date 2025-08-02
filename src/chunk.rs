use bevy::prelude::*;
use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use crate::voxel::Voxel;

pub const CHUNK_SIZE: usize = 32;
pub const CHUNK_SIZE_F32: f32 = CHUNK_SIZE as f32;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    
    pub fn from_world_pos(world_pos: Vec3) -> Self {
        Self {
            x: (world_pos.x / CHUNK_SIZE_F32).floor() as i32,
            y: (world_pos.y / CHUNK_SIZE_F32).floor() as i32,
            z: (world_pos.z / CHUNK_SIZE_F32).floor() as i32,
        }
    }
    
    pub fn to_world_pos(self) -> Vec3 {
        Vec3::new(
            self.x as f32 * CHUNK_SIZE_F32,
            self.y as f32 * CHUNK_SIZE_F32,
            self.z as f32 * CHUNK_SIZE_F32,
        )
    }
    
    pub fn neighbors(self) -> [ChunkCoord; 6] {
        [
            ChunkCoord::new(self.x + 1, self.y, self.z),     // +X
            ChunkCoord::new(self.x - 1, self.y, self.z),     // -X
            ChunkCoord::new(self.x, self.y + 1, self.z),     // +Y
            ChunkCoord::new(self.x, self.y - 1, self.z),     // -Y
            ChunkCoord::new(self.x, self.y, self.z + 1),     // +Z
            ChunkCoord::new(self.x, self.y, self.z - 1),     // -Z
        ]
    }

    /// Returns all 26 neighboring chunks (faces, edges, and corners)
    pub fn all_neighbors(self) -> Vec<ChunkCoord> {
        let mut neighbors = Vec::with_capacity(26);
        
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    // Skip the center chunk (self)
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    neighbors.push(ChunkCoord::new(self.x + dx, self.y + dy, self.z + dz));
                }
            }
        }
        
        neighbors
    }

    /// Returns all chunks within a given radius (for sampling-based operations)
    pub fn neighbors_within_radius(self, radius: i32) -> Vec<ChunkCoord> {
        let mut neighbors = Vec::new();
        
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                for dz in -radius..=radius {
                    // Skip the center chunk (self)
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    neighbors.push(ChunkCoord::new(self.x + dx, self.y + dy, self.z + dz));
                }
            }
        }
        
        neighbors
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ChunkData {
    pub coord: ChunkCoord,
    pub voxels: Box<[[[Voxel; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE]>,
    pub modified: bool,
    pub material_palette: Vec<String>, // Maps material_id -> material name
    #[serde(skip)]
    material_lookup: AHashMap<String, u8>, // Maps material name -> material_id (not serialized)
}

impl<'de> Deserialize<'de> for ChunkData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ChunkDataHelper {
            coord: ChunkCoord,
            voxels: Box<[[[Voxel; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE]>,
            modified: bool,
            material_palette: Vec<String>,
        }
        
        let helper = ChunkDataHelper::deserialize(deserializer)?;
        let mut chunk_data = ChunkData {
            coord: helper.coord,
            voxels: helper.voxels,
            modified: helper.modified,
            material_palette: helper.material_palette,
            material_lookup: AHashMap::new(),
        };
        
        chunk_data.rebuild_lookup();
        Ok(chunk_data)
    }
}

impl ChunkData {
    pub fn new(coord: ChunkCoord) -> Self {
        let mut palette = Vec::new();
        palette.push("air".to_string()); // Air is always at index 0
        
        let mut lookup = AHashMap::new();
        lookup.insert("air".to_string(), 0);
        
        Self {
            coord,
            voxels: Box::new([[[Voxel::default(); CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE]),
            modified: false,
            material_palette: palette,
            material_lookup: lookup,
        }
    }
    
    pub fn get_material_id(&mut self, material_name: &str) -> u8 {
        if let Some(&id) = self.material_lookup.get(material_name) {
            return id;
        }
        
        // Add new material to palette
        if self.material_palette.len() >= 256 {
            panic!("Chunk palette overflow: too many materials in chunk");
        }
        
        let id = self.material_palette.len() as u8;
        self.material_palette.push(material_name.to_string());
        self.material_lookup.insert(material_name.to_string(), id);
        id
    }
    
    pub fn get_material_name(&self, material_id: u8) -> Option<&String> {
        self.material_palette.get(material_id as usize)
    }
    
    pub fn rebuild_lookup(&mut self) {
        self.material_lookup.clear();
        for (id, name) in self.material_palette.iter().enumerate() {
            self.material_lookup.insert(name.clone(), id as u8);
        }
    }
    
    pub fn get_voxel(&self, x: usize, y: usize, z: usize) -> Option<Voxel> {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            return None;
        }
        Some(self.voxels[x][y][z])
    }
    
    pub fn set_voxel(&mut self, x: usize, y: usize, z: usize, voxel: Voxel) -> bool {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            return false;
        }
        if self.voxels[x][y][z] != voxel {
            self.voxels[x][y][z] = voxel;
            self.modified = true;
        }
        true
    }
    
    pub fn get_voxel_world_pos(&self, world_pos: Vec3) -> Option<Voxel> {
        let chunk_pos = self.coord.to_world_pos();
        let local_pos = world_pos - chunk_pos;
        
        if local_pos.x < 0.0 || local_pos.y < 0.0 || local_pos.z < 0.0 
            || local_pos.x >= CHUNK_SIZE_F32 
            || local_pos.y >= CHUNK_SIZE_F32 
            || local_pos.z >= CHUNK_SIZE_F32 {
            return None;
        }
        
        let x = local_pos.x as usize;
        let y = local_pos.y as usize;
        let z = local_pos.z as usize;
        
        self.get_voxel(x, y, z)
    }
    
    pub fn set_voxel_world_pos(&mut self, world_pos: Vec3, voxel: Voxel) -> bool {
        let chunk_pos = self.coord.to_world_pos();
        let local_pos = world_pos - chunk_pos;
        
        if local_pos.x < 0.0 || local_pos.y < 0.0 || local_pos.z < 0.0 
            || local_pos.x >= CHUNK_SIZE_F32 
            || local_pos.y >= CHUNK_SIZE_F32 
            || local_pos.z >= CHUNK_SIZE_F32 {
            return false;
        }
        
        let x = local_pos.x as usize;
        let y = local_pos.y as usize;
        let z = local_pos.z as usize;
        
        self.set_voxel(x, y, z, voxel)
    }
    
    pub fn set_voxel_by_material(&mut self, x: usize, y: usize, z: usize, material_name: &str) -> bool {
        let material_id = self.get_material_id(material_name);
        self.set_voxel(x, y, z, Voxel::new(material_id))
    }
    
}

#[derive(Component)]
pub struct ChunkMesh {
    pub coord: ChunkCoord,
    pub needs_update: bool,
}

impl ChunkMesh {
    pub fn new(coord: ChunkCoord) -> Self {
        Self {
            coord,
            needs_update: true,
        }
    }
}

#[derive(Component)]
pub struct OpaqueMesh {
    pub coord: ChunkCoord,
}

#[derive(Component)]
pub struct TransparentMesh {
    pub coord: ChunkCoord,
}

pub type ChunkMap = AHashMap<ChunkCoord, ChunkData>;