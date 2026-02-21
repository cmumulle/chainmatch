use bevy::prelude::*;
use ace_shared::physics::BallPhysicsParams;
use crate::resources::court::CourtEntity;

/// Marker for the ball entity.
#[derive(Component)]
pub struct Ball;

/// Ball state tracked each physics tick.
#[derive(Component)]
pub struct BallState {
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub params: BallPhysicsParams,
    pub grounded: bool,
}

impl Default for BallState {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            params: BallPhysicsParams::hard_court(),
            grounded: false,
        }
    }
}

/// Visual scale multiplier for the ball (actual radius is tiny).
const BALL_VISUAL_SCALE: f32 = 3.0;

/// Minimum speed below which the ball is considered stopped.
const STOP_THRESHOLD: f32 = 0.1;

/// Spawns a ball entity at the given position.
pub fn spawn_ball(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
) {
    let params = BallPhysicsParams::hard_court();
    let visual_radius = params.ball_radius * BALL_VISUAL_SCALE;

    let ball_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.85, 0.0), // Yellow
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(visual_radius))),
        MeshMaterial3d(ball_mat),
        Transform::from_translation(position),
        Ball,
        BallState::default(),
        CourtEntity,
    ));
}

/// Fixed-timestep ball physics system (120Hz).
pub fn ball_physics_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut BallState), With<Ball>>,
) {
    let dt = time.delta_secs();

    for (mut transform, mut state) in query.iter_mut() {
        if state.grounded {
            continue;
        }

        let gravity = state.params.gravity;
        let ball_radius = state.params.ball_radius;
        let restitution = state.params.restitution;
        let max_speed = state.params.max_speed;

        // 1. Apply gravity
        state.velocity.y += gravity * dt;

        // 2. Update position
        transform.translation += state.velocity * dt;

        // 3. Bounce detection: if ball reaches court surface
        if transform.translation.y <= ball_radius {
            transform.translation.y = ball_radius;

            // Reflect vertical velocity with restitution loss
            state.velocity.y = -state.velocity.y * restitution;

            // Apply friction to horizontal velocity on bounce
            let friction = 0.85;
            state.velocity.x *= friction;
            state.velocity.z *= friction;

            // Check if ball has effectively stopped
            if state.velocity.length() < STOP_THRESHOLD {
                state.velocity = Vec3::ZERO;
                state.grounded = true;
            }
        }

        // Clamp to max speed
        let speed = state.velocity.length();
        if speed > max_speed {
            state.velocity = state.velocity.normalize() * max_speed;
        }
    }
}
