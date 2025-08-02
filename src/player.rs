use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub sensitivity: f32,
    pub reach: f32,
    pub velocity: Vec3,
    pub is_grounded: bool,
    pub jump_strength: f32,
    pub gravity: f32,
    pub pitch: f32, // Track accumulated camera pitch
}

impl Default for Player {
    fn default() -> Self {
        Self {
            speed: 10.0,
            sensitivity: 0.002,
            reach: 5.0,
            velocity: Vec3::ZERO,
            is_grounded: false,
            jump_strength: 15.0,
            gravity: -30.0,
            pitch: 0.0,
        }
    }
}

#[derive(Component)]
pub struct PlayerCamera;

pub fn setup_player(
    mut commands: Commands,
    physics_config: Res<crate::world::PlayerPhysicsConfig>,
) {
    let player_pos = Vec3::new(0.0, 70.0, 0.0);
    let eye_height = physics_config.height * 0.8; // 80% of player height
    
    commands
        .spawn((
            Player::default(),
            Transform::from_translation(player_pos),
        ))
        .with_children(|parent| {
            parent.spawn((
                PlayerCamera,
                Camera3d::default(),
                Transform::from_xyz(0.0, eye_height, 0.0)
                    .looking_at(Vec3::new(0.0, eye_height, -1.0), Vec3::Y),
            ));
        });
}