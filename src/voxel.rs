use bevy::prelude::*;
use ahash::AHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub color: [f32; 4], // RGBA
    pub solid: bool,
}

impl Material {
    pub fn new(name: impl Into<String>, color: [f32; 4], solid: bool) -> Self {
        Self {
            name: name.into(),
            color,
            solid,
        }
    }
    
    pub fn get_color(&self) -> Color {
        Color::srgba(self.color[0], self.color[1], self.color[2], self.color[3])
    }
    
    pub fn is_solid(&self) -> bool {
        self.solid
    }
    
    pub fn is_transparent(&self) -> bool {
        self.color[3] < 1.0 || !self.solid
    }
}

#[derive(Debug, Clone, Resource)]
pub struct MaterialRegistry {
    materials: AHashMap<String, Material>,
    unknown_material: Material,
}

impl MaterialRegistry {
    pub fn new() -> Self {
        let unknown_material = Material::new(
            "unknown",
            [1.0, 0.0, 1.0, 1.0], // Bright magenta for missing materials
            true,
        );
        
        Self {
            materials: AHashMap::new(),
            unknown_material,
        }
    }
    
    pub fn register(&mut self, material: Material) {
        self.materials.insert(material.name.clone(), material);
    }
    
    pub fn get(&self, name: &str) -> &Material {
        self.materials.get(name).unwrap_or(&self.unknown_material)
    }
    
    pub fn contains(&self, name: &str) -> bool {
        self.materials.contains_key(name)
    }
    
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Material)> {
        self.materials.iter()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voxel {
    pub material_id: u8, // Index into chunk's material palette
}

impl Voxel {
    pub fn new(material_id: u8) -> Self {
        Self { material_id }
    }
    
    pub fn air() -> Self {
        Self::new(0) // Air is always index 0 in palette
    }
}

impl Default for Voxel {
    fn default() -> Self {
        Self::air()
    }
}