use bevy::prelude::*;
use ace_shared::physics::BallPhysicsParams;
use crate::resources::court::{
    CourtEntity, HALF_COURT_LENGTH, HALF_COURT_WIDTH,
    NET_HEIGHT_CENTER, NET_HEIGHT_POSTS, COURT_WIDTH,
};

/// Marker for the ball entity.
#[derive(Component)]
pub struct Ball;

/// Which side of the court an event occurred on.
#[derive(Debug, Clone, Copy)]
pub enum CourtSide {
    Near,  // positive z (player side)
    Far,   // negative z (opponent side)
}

/// Event: ball hit the net.
#[derive(Event, Debug)]
pub struct NetFault {
    pub position: Vec3,
    pub side: CourtSide,
}

/// Event: ball bounced outside court lines.
#[derive(Event, Debug)]
pub struct OutOfBounds {
    pub position: Vec3,
    pub side: CourtSide,
}

/// Event: ball bounced inside court lines.
#[derive(Event, Debug)]
pub struct ValidBounce {
    pub position: Vec3,
    pub side: CourtSide,
}

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

/// Calculate net height at a given x position (interpolates between center and posts).
fn net_height_at_x(x: f32) -> f32 {
    let half_width = COURT_WIDTH / 2.0;
    let t = (x.abs() / half_width).clamp(0.0, 1.0);
    // Linear interpolation: center height at x=0, post height at edges
    NET_HEIGHT_CENTER + t * (NET_HEIGHT_POSTS - NET_HEIGHT_CENTER)
}

/// Determine which side of court a z position is on.
fn court_side(z: f32) -> CourtSide {
    if z >= 0.0 { CourtSide::Near } else { CourtSide::Far }
}

/// Check if a bounce position is within court lines.
fn is_in_bounds(pos: Vec3) -> bool {
    pos.x.abs() <= HALF_COURT_WIDTH && pos.z.abs() <= HALF_COURT_LENGTH
}

/// Fixed-timestep ball physics system (120Hz).
pub fn ball_physics_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut BallState), With<Ball>>,
    mut net_faults: EventWriter<NetFault>,
    mut out_of_bounds: EventWriter<OutOfBounds>,
    mut valid_bounces: EventWriter<ValidBounce>,
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
        let air_drag = state.params.air_drag;
        let magnus_coeff = state.params.magnus_coefficient;

        let prev_z = transform.translation.z;

        // 1. Apply gravity
        state.velocity.y += gravity * dt;

        // 2. Air drag: deceleration proportional to speed²
        let speed = state.velocity.length();
        if speed > 0.0 {
            let drag_magnitude = air_drag * speed * speed;
            let drag_force = -state.velocity.normalize() * drag_magnitude;
            state.velocity += drag_force * dt;
        }

        // 3. Magnus force: angular_velocity × velocity × coefficient
        let angular_vel = state.angular_velocity;
        if angular_vel.length() > 0.0 && speed > 0.0 {
            let magnus_force = angular_vel.cross(state.velocity) * magnus_coeff;
            state.velocity += magnus_force * dt;
        }

        // 4. Update position
        transform.translation += state.velocity * dt;

        let new_z = transform.translation.z;

        // 5. Net collision: ball crossed z=0 plane
        if (prev_z > 0.0 && new_z <= 0.0) || (prev_z < 0.0 && new_z >= 0.0) {
            let net_height = net_height_at_x(transform.translation.x);
            if transform.translation.y < net_height {
                let side = court_side(prev_z);
                net_faults.send(NetFault {
                    position: transform.translation,
                    side,
                });
                // Stop ball at net
                state.velocity = Vec3::ZERO;
                state.angular_velocity = Vec3::ZERO;
                state.grounded = true;
                continue;
            }
        }

        // 6. Bounce detection: if ball reaches court surface
        if transform.translation.y <= ball_radius {
            transform.translation.y = ball_radius;
            let bounce_pos = transform.translation;
            let side = court_side(bounce_pos.z);

            if is_in_bounds(bounce_pos) {
                valid_bounces.send(ValidBounce {
                    position: bounce_pos,
                    side,
                });
            } else {
                out_of_bounds.send(OutOfBounds {
                    position: bounce_pos,
                    side,
                });
            }

            // Reflect vertical velocity with restitution loss
            state.velocity.y = -state.velocity.y * restitution;

            // Apply friction to horizontal velocity on bounce
            let friction = 0.85;
            state.velocity.x *= friction;
            state.velocity.z *= friction;

            // Reduce spin on bounce
            state.angular_velocity *= 0.7;

            // Check if ball has effectively stopped
            if state.velocity.length() < STOP_THRESHOLD {
                state.velocity = Vec3::ZERO;
                state.angular_velocity = Vec3::ZERO;
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

/// Debug system: logs ball events to console.
pub fn debug_ball_events(
    mut net_faults: EventReader<NetFault>,
    mut out_of_bounds: EventReader<OutOfBounds>,
    mut valid_bounces: EventReader<ValidBounce>,
) {
    for event in net_faults.read() {
        info!("NET FAULT at {:?} (side: {:?})", event.position, event.side);
    }
    for event in out_of_bounds.read() {
        info!("OUT OF BOUNDS at {:?} (side: {:?})", event.position, event.side);
    }
    for event in valid_bounces.read() {
        info!("VALID BOUNCE at {:?} (side: {:?})", event.position, event.side);
    }
}

/// Debug launch preset index (cycles through shot types).
#[derive(Resource, Default)]
pub struct DebugLaunchIndex(usize);

/// Debug system: press F1 to launch ball with preset velocity/spin.
pub fn debug_ball_launch(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut launch_index: ResMut<DebugLaunchIndex>,
    mut query: Query<(&mut Transform, &mut BallState), With<Ball>>,
) {
    if !keyboard.just_pressed(KeyCode::F1) {
        return;
    }

    // Preset shots: (velocity, angular_velocity, description)
    let presets: [(Vec3, Vec3, &str); 6] = [
        // Flat serve: fast, no spin — lands in
        (Vec3::new(0.0, 5.0, -30.0), Vec3::ZERO, "Flat serve (in)"),
        // Topspin: forward + upward, spin around X axis (dips down) — lands in
        (Vec3::new(0.0, 8.0, -25.0), Vec3::new(80.0, 0.0, 0.0), "Topspin (in)"),
        // Slice: forward + slight side — lands in
        (Vec3::new(3.0, 6.0, -25.0), Vec3::new(-30.0, 50.0, 0.0), "Slice (in)"),
        // Low shot into net — net fault
        (Vec3::new(0.0, 0.2, -15.0), Vec3::ZERO, "Into net (fault)"),
        // Wide shot — out of bounds
        (Vec3::new(12.0, 5.0, -20.0), Vec3::ZERO, "Wide shot (out)"),
        // Long shot — out of bounds beyond baseline
        (Vec3::new(0.0, 3.0, -40.0), Vec3::ZERO, "Long shot (out)"),
    ];

    let idx = launch_index.0 % presets.len();
    let (vel, spin, name) = presets[idx];

    for (mut transform, mut state) in query.iter_mut() {
        // Reset ball position to serve position
        transform.translation = Vec3::new(0.0, 2.5, 10.0);
        state.velocity = vel;
        state.angular_velocity = spin;
        state.grounded = false;
    }

    info!("Debug launch [{}]: {} (vel={}, spin={})", idx, name, vel, spin);
    launch_index.0 += 1;
}
