use bevy::prelude::*;
use std::f32::consts::PI;

#[derive(Resource)]
pub struct DayNightCycle {
    pub time_of_day: f32,      // 0.0 to 1.0 (0 = midnight, 0.5 = noon)
    pub day_length: f32,       // Length of a full day in seconds
    pub speed_multiplier: f32, // Speed up time for testing
}

impl Default for DayNightCycle {
    fn default() -> Self {
        Self {
            time_of_day: 0.25, // Start at dawn (6 AM)
            day_length: 300.0, // 5 minutes for a full day
            speed_multiplier: 1.0,
        }
    }
}

#[derive(Component)]
pub struct Sun;

#[derive(Component)]
pub struct Moon;

#[derive(Component)]
pub struct SkyLight;

pub fn setup_sky_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create sun entity
    commands.spawn((
        Sun,
        Mesh3d(meshes.add(Sphere::new(10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.9, 0.7),
            emissive: LinearRgba::new(20.0, 18.0, 14.0, 1.0), // Bright emissive
            unlit: true,
            ..default()
        })),
        Transform::from_translation(Vec3::new(0.0, 100.0, 100.0)),
    ));

    // Create moon entity
    commands.spawn((
        Moon,
        Mesh3d(meshes.add(Sphere::new(8.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.8, 0.9),
            emissive: LinearRgba::new(0.2, 0.2, 0.3, 1.0), // Dim emissive
            unlit: true,
            ..default()
        })),
        Transform::from_translation(Vec3::new(0.0, -100.0, -100.0)),
    ));

    // Create directional light (will be positioned to match sun)
    commands.spawn((
        SkyLight,
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: false,
            color: Color::srgb(1.0, 0.95, 0.8),
            ..default()
        },
        Transform::from_xyz(0.0, 100.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

pub fn day_night_cycle_system(
    time: Res<Time>,
    mut cycle: ResMut<DayNightCycle>,
    mut sun_query: Query<&mut Transform, (With<Sun>, Without<Moon>, Without<SkyLight>)>,
    mut moon_query: Query<&mut Transform, (With<Moon>, Without<Sun>, Without<SkyLight>)>,
    mut light_query: Query<
        (&mut Transform, &mut DirectionalLight),
        (With<SkyLight>, Without<Sun>, Without<Moon>),
    >,
    mut clear_color: ResMut<ClearColor>,
) {
    // Update time of day
    cycle.time_of_day += (time.delta_secs() * cycle.speed_multiplier) / cycle.day_length;
    if cycle.time_of_day >= 1.0 {
        cycle.time_of_day -= 1.0;
    }

    // Calculate sun angle (0 = sunrise, 0.5 = noon, 1.0 = sunset)
    let sun_angle = cycle.time_of_day * 2.0 * PI;
    let sun_height = sun_angle.sin();
    let sun_distance = 200.0;

    // Sun position in a circular path
    let sun_x = sun_angle.cos() * sun_distance;
    let sun_y = sun_height * sun_distance;
    let sun_z = 0.0;

    // Moon is opposite to sun
    let moon_x = -sun_x;
    let moon_y = -sun_y;
    let moon_z = 0.0;

    // Update sun position
    if let Ok(mut sun_transform) = sun_query.get_single_mut() {
        sun_transform.translation = Vec3::new(sun_x, sun_y, sun_z);
    }

    // Update moon position
    if let Ok(mut moon_transform) = moon_query.get_single_mut() {
        moon_transform.translation = Vec3::new(moon_x, moon_y, moon_z);
    }

    // Update directional light
    if let Ok((mut light_transform, mut light)) = light_query.get_single_mut() {
        // Light points from sun toward origin
        light_transform.translation = Vec3::new(sun_x, sun_y, sun_z);
        light_transform.look_at(Vec3::ZERO, Vec3::Y);

        // Smooth transitions using continuous functions
        let sun_factor = (sun_height + 1.0) * 0.5; // Convert -1..1 to 0..1
        
        // Light intensity - smooth curve from night to day
        let light_intensity = if sun_height > -0.1 {
            // Above horizon or just below - day lighting
            let intensity_factor = ((sun_height + 0.1) * 0.9).max(0.0).min(1.0);
            intensity_factor * 15000.0
        } else {
            // Below horizon - night lighting
            let moon_factor = ((-sun_height - 0.1) * 2.0).max(0.0).min(1.0);
            moon_factor * 500.0
        };
        
        light.illuminance = light_intensity;

        // Smooth color transitions
        let day_color = Color::srgb(1.0, 0.95, 0.8);     // Warm white
        let sunset_color = Color::srgb(1.0, 0.7, 0.4);   // Orange
        let night_color = Color::srgb(0.7, 0.8, 1.0);    // Cool blue
        
        light.color = if sun_height > 0.3 {
            // High sun - pure day color
            day_color
        } else if sun_height > -0.1 {
            // Sunset/sunrise transition
            let transition_factor = (sun_height + 0.1) / 0.4; // 0 at horizon, 1 at 0.3 height
            Color::srgb(
                sunset_color.to_srgba().red + (day_color.to_srgba().red - sunset_color.to_srgba().red) * transition_factor,
                sunset_color.to_srgba().green + (day_color.to_srgba().green - sunset_color.to_srgba().green) * transition_factor,
                sunset_color.to_srgba().blue + (day_color.to_srgba().blue - sunset_color.to_srgba().blue) * transition_factor,
            )
        } else if sun_height > -0.4 {
            // Night transition - quick fade to night lighting
            let night_factor = ((-sun_height - 0.1) / 0.3).max(0.0).min(1.0); // Faster transition
            Color::srgb(
                sunset_color.to_srgba().red + (night_color.to_srgba().red - sunset_color.to_srgba().red) * night_factor,
                sunset_color.to_srgba().green + (night_color.to_srgba().green - sunset_color.to_srgba().green) * night_factor,
                sunset_color.to_srgba().blue + (night_color.to_srgba().blue - sunset_color.to_srgba().blue) * night_factor,
            )
        } else {
            // Full night - constant moonlight for most of the night
            night_color
        }
    }

    // Update sky color with smooth transitions
    let day_sky = Color::srgb(0.5, 0.7, 0.9);        // Blue sky
    let sunset_sky = Color::srgb(0.8, 0.5, 0.6);     // Orange/pink sunset
    let night_sky = Color::srgb(0.05, 0.05, 0.1);    // Dark night
    
    let sky_color = if sun_height > 0.2 {
        // High sun - pure day sky
        day_sky
    } else if sun_height > -0.2 {
        // Sunset/sunrise transition zone
        let transition_factor = (sun_height + 0.2) / 0.4; // 0 at -0.2, 1 at 0.2
        Color::srgb(
            sunset_sky.to_srgba().red + (day_sky.to_srgba().red - sunset_sky.to_srgba().red) * transition_factor,
            sunset_sky.to_srgba().green + (day_sky.to_srgba().green - sunset_sky.to_srgba().green) * transition_factor,
            sunset_sky.to_srgba().blue + (day_sky.to_srgba().blue - sunset_sky.to_srgba().blue) * transition_factor,
        )
    } else if sun_height > -0.5 {
        // Night transition - quick fade to full darkness
        let night_factor = ((-sun_height - 0.2) / 0.3).max(0.0).min(1.0); // Faster transition over smaller range
        Color::srgb(
            sunset_sky.to_srgba().red + (night_sky.to_srgba().red - sunset_sky.to_srgba().red) * night_factor,
            sunset_sky.to_srgba().green + (night_sky.to_srgba().green - sunset_sky.to_srgba().green) * night_factor,
            sunset_sky.to_srgba().blue + (night_sky.to_srgba().blue - sunset_sky.to_srgba().blue) * night_factor,
        )
    } else {
        // Full night - constant dark color for most of the night
        night_sky
    };

    clear_color.0 = sky_color;
}

pub fn toggle_time_speed_system(
    input: Res<ButtonInput<KeyCode>>,
    mut cycle: ResMut<DayNightCycle>,
) {
    if input.just_pressed(KeyCode::KeyT) {
        cycle.speed_multiplier = if cycle.speed_multiplier == 1.0 {
            10.0
        } else {
            1.0
        };
        info!("Time speed: {}x", cycle.speed_multiplier);
    }
}
