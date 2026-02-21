use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::resources::court::{
    CourtEntity, HALF_COURT_LENGTH, HALF_COURT_WIDTH,
};
use crate::systems::shot::ShotTimingState;

/// Marker for the aim reticle (projected circle on court).
#[derive(Component)]
pub struct AimIndicator;

/// Resource storing the current aim position on the court (if valid).
#[derive(Resource, Default)]
pub struct AimTarget {
    /// Court position where the mouse ray intersects y=0 (opponent's half only).
    pub position: Option<Vec3>,
}

/// Base precision radius for the aim reticle (meters).
/// Will later be driven by hero stats.
const BASE_PRECISION_RADIUS: f32 = 0.5;

/// Reticle ring thickness (visual only).
const RING_THICKNESS: f32 = 0.04;

/// Reticle colors based on proximity to lines.
const COLOR_GREEN: Color = Color::srgba(0.0, 0.9, 0.2, 0.6);
const COLOR_YELLOW: Color = Color::srgba(0.9, 0.9, 0.0, 0.6);
const COLOR_RED: Color = Color::srgba(0.9, 0.1, 0.1, 0.6);

/// Spawns the aim reticle (a flat torus ring on the court surface).
pub fn spawn_aim_indicator(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mat = materials.add(StandardMaterial {
        base_color: COLOR_GREEN,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Torus::new(
            BASE_PRECISION_RADIUS - RING_THICKNESS,
            BASE_PRECISION_RADIUS,
        ))),
        MeshMaterial3d(mat),
        // Torus lies in XZ by default in Bevy; place it just above court surface
        Transform::from_xyz(0.0, -10.0, 0.0),
        Visibility::Hidden,
        AimIndicator,
        CourtEntity,
    ));
}

/// Returns the minimum distance from position to the nearest court line (sidelines + baselines).
/// Only considers opponent's half for baseline distance.
fn distance_to_nearest_line(pos: Vec3) -> f32 {
    let dx_left = (pos.x - (-HALF_COURT_WIDTH)).abs();
    let dx_right = (pos.x - HALF_COURT_WIDTH).abs();
    let dz_baseline = (pos.z - (-HALF_COURT_LENGTH)).abs();
    let dz_net = pos.z.abs(); // distance to net line (z=0)

    dx_left.min(dx_right).min(dz_baseline).min(dz_net)
}

/// Returns whether the position is inside the opponent's court bounds.
fn is_in_opponent_court(pos: Vec3) -> bool {
    pos.x.abs() <= HALF_COURT_WIDTH && pos.z >= -HALF_COURT_LENGTH && pos.z <= 0.0
}

/// Choose reticle color based on proximity to lines and out-of-bounds.
fn reticle_color(pos: Vec3) -> Color {
    if !is_in_opponent_court(pos) {
        return COLOR_RED;
    }

    let dist = distance_to_nearest_line(pos);

    if dist < 0.3 {
        COLOR_RED
    } else if dist < 1.0 {
        COLOR_YELLOW
    } else {
        COLOR_GREEN
    }
}

/// System that projects mouse position onto the court plane (y=0) and
/// updates the aim reticle position and color.
pub fn update_aim_target(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut aim_target: ResMut<AimTarget>,
    mut indicator: Query<
        (&mut Transform, &mut Visibility, &MeshMaterial3d<StandardMaterial>),
        With<AimIndicator>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    timing: Res<ShotTimingState>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, camera_transform)) = cameras.get_single() else {
        return;
    };

    // Get cursor position
    let Some(cursor_pos) = window.cursor_position() else {
        aim_target.position = None;
        for (_, mut vis, _) in indicator.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    };

    // Cast ray from camera through cursor position
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        aim_target.position = None;
        for (_, mut vis, _) in indicator.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    };

    // Intersect ray with y=0 plane
    let direction_y = ray.direction.y;
    if direction_y.abs() < 1e-6 {
        aim_target.position = None;
        for (_, mut vis, _) in indicator.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let t = -ray.origin.y / direction_y;
    if t < 0.0 {
        aim_target.position = None;
        for (_, mut vis, _) in indicator.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let hit_point = ray.origin + *ray.direction * t;

    // Only valid on opponent's half (negative z)
    if hit_point.z > 0.0 {
        aim_target.position = None;
        for (_, mut vis, _) in indicator.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    aim_target.position = Some(hit_point);

    // Update reticle position, color, and scale (timing quality preview)
    let color = reticle_color(hit_point);
    let scale = if timing.in_zone {
        timing.quality_preview.reticle_scale()
    } else {
        1.0
    };

    for (mut transform, mut vis, mat_handle) in indicator.iter_mut() {
        transform.translation = Vec3::new(hit_point.x, 0.01, hit_point.z);
        transform.scale = Vec3::splat(scale);
        *vis = Visibility::Visible;

        // Update material color
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = color;
        }
    }
}
