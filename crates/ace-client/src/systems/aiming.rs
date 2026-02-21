use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::resources::court::CourtEntity;

/// Marker for the aim debug sphere.
#[derive(Component)]
pub struct AimIndicator;

/// Resource storing the current aim position on the court (if valid).
#[derive(Resource, Default)]
pub struct AimTarget {
    /// Court position where the mouse ray intersects y=0 (opponent's half only).
    pub position: Option<Vec3>,
}

/// Spawns the aim debug indicator sphere.
pub fn spawn_aim_indicator(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.2, 0.2, 0.7),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.15))),
        MeshMaterial3d(mat),
        Transform::from_xyz(0.0, -10.0, 0.0), // Start hidden below court
        Visibility::Hidden,
        AimIndicator,
        CourtEntity,
    ));
}

/// System that projects mouse position onto the court plane (y=0) and
/// updates the debug indicator sphere position.
pub fn update_aim_target(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut aim_target: ResMut<AimTarget>,
    mut indicator: Query<(&mut Transform, &mut Visibility), With<AimIndicator>>,
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
        for (_, mut vis) in indicator.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    };

    // Cast ray from camera through cursor position
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        aim_target.position = None;
        for (_, mut vis) in indicator.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    };

    // Intersect ray with y=0 plane
    // Ray: P = origin + t * direction
    // Plane: y = 0, so origin.y + t * direction.y = 0
    // t = -origin.y / direction.y
    let direction_y = ray.direction.y;
    if direction_y.abs() < 1e-6 {
        // Ray is parallel to ground plane
        aim_target.position = None;
        for (_, mut vis) in indicator.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let t = -ray.origin.y / direction_y;
    if t < 0.0 {
        // Intersection is behind the camera
        aim_target.position = None;
        for (_, mut vis) in indicator.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let hit_point = ray.origin + *ray.direction * t;

    // Only valid on opponent's half (negative z)
    if hit_point.z > 0.0 {
        aim_target.position = None;
        for (_, mut vis) in indicator.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    aim_target.position = Some(hit_point);

    // Update debug indicator position
    for (mut transform, mut vis) in indicator.iter_mut() {
        transform.translation = Vec3::new(hit_point.x, 0.15, hit_point.z);
        *vis = Visibility::Visible;
    }
}
