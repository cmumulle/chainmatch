use bevy::prelude::*;
use crate::systems::aiming::AimTarget;
use crate::systems::ball_physics::{Ball, BallState};
use crate::systems::movement::Player;
use crate::resources::court::{HALF_COURT_LENGTH, NET_HEIGHT_CENTER};

/// Serve state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServePhase {
    #[default]
    /// Not serving.
    Idle,
    /// Ball tossed, rising.
    Tossing,
    /// Ball at/past apex, descending — player can hit.
    Descending,
}

/// Resource tracking serve state.
#[derive(Resource, Default)]
pub struct ServeState {
    pub phase: ServePhase,
    /// Ball Y at the moment of click (for power/control calculation).
    pub contact_height: f32,
    /// Whether a serve is pending (ball needs to be positioned for serve).
    pub serve_pending: bool,
}

/// Ball toss initial upward velocity (m/s).
const TOSS_VELOCITY: f32 = 8.0;

/// Minimum contact height: if ball falls below this, it's a missed toss (fault).
const MIN_CONTACT_HEIGHT: f32 = 0.5;

/// Maximum contact height for scaling (apex of a typical toss).
const MAX_CONTACT_HEIGHT: f32 = 4.0;

/// Serve speed range based on contact height.
const SERVE_MIN_SPEED: f32 = 20.0;
const SERVE_MAX_SPEED: f32 = 55.0;

/// Precision range: higher contact = larger reticle (more power, less control).
const SERVE_MIN_PRECISION: f32 = 0.3;
const SERVE_MAX_PRECISION: f32 = 1.5;

/// System: Spacebar triggers ball toss during idle serve state.
pub fn serve_toss_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut serve_state: ResMut<ServeState>,
    mut ball_query: Query<(&mut Transform, &mut BallState), With<Ball>>,
    player_query: Query<&Transform, (With<Player>, Without<Ball>)>,
) {
    if serve_state.phase != ServePhase::Idle || !serve_state.serve_pending {
        return;
    }

    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    let player_pos = player_transform.translation;

    for (mut ball_transform, mut ball_state) in ball_query.iter_mut() {
        // Position ball at player's hand height
        ball_transform.translation = Vec3::new(player_pos.x, 2.0, player_pos.z);
        ball_state.velocity = Vec3::new(0.0, TOSS_VELOCITY, 0.0);
        ball_state.angular_velocity = Vec3::ZERO;
        ball_state.grounded = false;
    }

    serve_state.phase = ServePhase::Tossing;
    info!("Serve toss! Ball launched upward.");
}

/// System: Track ball apex and transition to Descending phase.
pub fn serve_track_system(
    mut serve_state: ResMut<ServeState>,
    ball_query: Query<&BallState, With<Ball>>,
) {
    if serve_state.phase != ServePhase::Tossing {
        return;
    }

    for ball_state in ball_query.iter() {
        // Ball reached apex when vertical velocity goes negative
        if ball_state.velocity.y <= 0.0 {
            serve_state.phase = ServePhase::Descending;
            info!("Ball at apex — click to serve!");
        }
    }
}

/// System: Left click during descent executes serve. Ball falling below threshold = fault.
pub fn serve_hit_system(
    mouse: Res<ButtonInput<MouseButton>>,
    mut serve_state: ResMut<ServeState>,
    aim_target: Res<AimTarget>,
    mut ball_query: Query<(&mut Transform, &mut BallState), With<Ball>>,
    player_query: Query<&Transform, (With<Player>, Without<Ball>)>,
) {
    if serve_state.phase != ServePhase::Descending {
        return;
    }

    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let player_pos = player_transform.translation;

    for (mut ball_transform, mut ball_state) in ball_query.iter_mut() {
        let ball_y = ball_transform.translation.y;

        // Check if ball fell too low (missed toss = fault)
        if ball_y < MIN_CONTACT_HEIGHT {
            info!("FAULT: Missed the toss (ball too low)");
            serve_state.phase = ServePhase::Idle;
            // Stop the ball on the ground
            ball_state.velocity = Vec3::ZERO;
            ball_state.grounded = true;
            ball_transform.translation.y = 0.033; // ball radius
            return;
        }

        if !mouse.just_pressed(MouseButton::Left) {
            return;
        }

        // Execute serve
        serve_state.contact_height = ball_y;
        serve_state.phase = ServePhase::Idle;
        serve_state.serve_pending = false;

        // Compute power/precision based on contact height
        let height_factor = ((ball_y - MIN_CONTACT_HEIGHT) / (MAX_CONTACT_HEIGHT - MIN_CONTACT_HEIGHT))
            .clamp(0.0, 1.0);

        // Higher contact = more power, less control
        let speed = SERVE_MIN_SPEED + height_factor * (SERVE_MAX_SPEED - SERVE_MIN_SPEED);
        let precision = SERVE_MIN_PRECISION + height_factor * (SERVE_MAX_PRECISION - SERVE_MIN_PRECISION);

        // Get aim target (or default to center of service box)
        let target = aim_target.position.unwrap_or(Vec3::new(0.0, 0.0, -HALF_COURT_LENGTH / 2.0));

        // Apply scatter based on precision
        let scatter = random_in_circle(precision);
        let actual_target = Vec3::new(
            target.x + scatter.x,
            0.0,
            target.z + scatter.y,
        );

        // Compute serve velocity
        let velocity = compute_serve_velocity(player_pos, actual_target, speed);

        ball_transform.translation = Vec3::new(player_pos.x, ball_y, player_pos.z);
        ball_state.velocity = velocity;
        ball_state.angular_velocity = Vec3::ZERO;
        ball_state.grounded = false;

        info!(
            "SERVE! Contact height={:.2}m, speed={:.1}m/s, precision={:.2}m, target=({:.1}, {:.1})",
            ball_y, speed, precision, actual_target.x, actual_target.z
        );
    }
}

/// Compute serve velocity toward target, ensuring ball clears net.
fn compute_serve_velocity(from: Vec3, to: Vec3, speed: f32) -> Vec3 {
    let dx = to.x - from.x;
    let dz = to.z - from.z;
    let horizontal_dist = (dx * dx + dz * dz).sqrt();

    if horizontal_dist < 0.1 {
        return Vec3::new(0.0, 2.0, -speed);
    }

    let dir_x = dx / horizontal_dist;
    let dir_z = dz / horizontal_dist;

    // Serve is typically hit downward or slightly upward from high contact point
    // Use a modest launch angle since serve starts from ~2-4m height
    let gravity = 9.81;
    let net_clearance = NET_HEIGHT_CENTER + 0.3;

    // Net is at z=0. Fraction of horizontal distance to net.
    let net_fraction = if dz.abs() > 0.1 {
        (0.0 - from.z) / dz
    } else {
        0.5
    };
    let net_fraction = net_fraction.clamp(0.0, 1.0);

    // Find minimum angle to clear net (start from negative angles for downward serves)
    let mut best_angle = 0.1_f32;
    let launch_height = from.y; // Serve contact height (2-4m typically)

    for angle_deg in -10..30 {
        let angle = (angle_deg as f32).to_radians();
        let vh = speed * angle.cos();
        let vy = speed * angle.sin();

        if vh < 0.1 {
            continue;
        }

        let t_net = net_fraction * horizontal_dist / vh;
        let height_at_net = launch_height + vy * t_net - 0.5 * gravity * t_net * t_net;

        if height_at_net >= net_clearance {
            best_angle = angle;
            break;
        }
    }

    let vh = speed * best_angle.cos();
    let vy = speed * best_angle.sin();

    Vec3::new(dir_x * vh, vy, dir_z * vh)
}

/// Random point in circle (reused from shot.rs pattern).
fn random_in_circle(radius: f32) -> Vec2 {
    loop {
        let x = (rand::random::<f32>() * 2.0 - 1.0) * radius;
        let y = (rand::random::<f32>() * 2.0 - 1.0) * radius;
        if x * x + y * y <= radius * radius {
            return Vec2::new(x, y);
        }
    }
}
