use bevy::prelude::*;
use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use rand::prelude::*;
use rand_distr::{Distribution, Normal};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Material {
    pub name: String,
    pub color: [f32; 4], // RGBA
    pub solid: bool,
    pub color_variance: f32, // Standard deviation for color variation
    pub gravity_modifier: f32, // Multiplier for gravity when inside this material (1.0 = normal, 0.0 = no gravity, -1.0 = upward force)
    pub swim_strength: f32, // Strength of swimming/jumping when inside this material (0.0 = no swimming, 1.0 = normal jump strength)
}

impl Material {
    pub fn new(name: impl Into<String>, color: [f32; 4], solid: bool) -> Self {
        Self {
            name: name.into(),
            color,
            solid,
            color_variance: 0.0, // No variance by default
            gravity_modifier: 1.0, // Normal gravity by default
            swim_strength: 0.0, // No swimming by default
        }
    }
    
    pub fn with_variance(name: impl Into<String>, color: [f32; 4], solid: bool, variance: f32) -> Self {
        Self {
            name: name.into(),
            color,
            solid,
            color_variance: variance,
            gravity_modifier: 1.0, // Normal gravity by default
            swim_strength: 0.0, // No swimming by default
        }
    }
    
    pub fn with_buoyancy(name: impl Into<String>, color: [f32; 4], solid: bool, gravity_modifier: f32, swim_strength: f32) -> Self {
        Self {
            name: name.into(),
            color,
            solid,
            color_variance: 0.0,
            gravity_modifier,
            swim_strength,
        }
    }
    
    pub fn get_color(&self) -> Color {
        Color::srgba(self.color[0], self.color[1], self.color[2], self.color[3])
    }
    
    pub fn get_varied_color(&self, rng: &mut impl Rng) -> Color {
        if self.color_variance <= 0.0 {
            return self.get_color();
        }
        
        let normal = Normal::new(0.0, self.color_variance).unwrap();
        
        let mut varied_color = [0.0; 4];
        for i in 0..3 { // Only vary RGB, not alpha
            let variation = normal.sample(rng);
            varied_color[i] = (self.color[i] + variation).clamp(0.0, 1.0);
        }
        varied_color[3] = self.color[3]; // Keep original alpha
        
        Color::srgba(varied_color[0], varied_color[1], varied_color[2], varied_color[3])
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