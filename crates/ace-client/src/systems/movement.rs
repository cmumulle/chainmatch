use bevy::prelude::*;
use crate::resources::court;

/// Marker for the player entity.
#[derive(Component)]
pub struct Player;

/// Player movement parameters.
#[derive(Component)]
pub struct PlayerMovement {
    pub base_speed: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub velocity: Vec3,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            base_speed: 8.0,
            acceleration: 25.0,
            deceleration: 20.0,
            velocity: Vec3::ZERO,
        }
    }
}

/// Spawns the player entity at the near baseline center.
pub fn spawn_player(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let player_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.6, 1.0),
        ..default()
    });

    let player_height = 1.8;
    let player_radius = 0.3;

    // Spawn at near baseline (positive Z = player's side)
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(player_radius, player_height - 2.0 * player_radius))),
        MeshMaterial3d(player_mat),
        Transform::from_xyz(0.0, player_height / 2.0, court::HALF_COURT_LENGTH - 1.0),
        Player,
        PlayerMovement::default(),
        court::CourtEntity,
    ));
}

/// System that moves the player based on WASD input with acceleration/deceleration.
pub fn player_movement_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut PlayerMovement), With<Player>>,
) {
    for (mut transform, mut movement) in query.iter_mut() {
        let dt = time.delta_secs();

        // Read input direction
        let mut input_dir = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            input_dir.z -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            input_dir.z += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            input_dir.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            input_dir.x += 1.0;
        }

        if input_dir.length_squared() > 0.0 {
            input_dir = input_dir.normalize();
            // Accelerate toward target velocity
            let target_vel = input_dir * movement.base_speed;
            movement.velocity = movement.velocity.lerp(target_vel, movement.acceleration * dt);
        } else {
            // Decelerate to zero
            let decel = movement.deceleration * dt;
            if movement.velocity.length() < decel {
                movement.velocity = Vec3::ZERO;
            } else {
                let dir = movement.velocity.normalize();
                movement.velocity -= dir * decel;
            }
        }

        // Apply velocity
        transform.translation += movement.velocity * dt;

        // Clamp to court + runoff bounds
        let half_x = court::HALF_COURT_WIDTH + court::SIDE_RUNOFF;
        let half_z = court::HALF_COURT_LENGTH + court::BASELINE_RUNOFF;
        transform.translation.x = transform.translation.x.clamp(-half_x, half_x);
        transform.translation.z = transform.translation.z.clamp(-half_z, half_z);
    }
}
