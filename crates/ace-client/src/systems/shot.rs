use bevy::prelude::*;
use crate::systems::aiming::AimTarget;
use crate::systems::ball_physics::{Ball, BallState};
use crate::systems::movement::Player;
use crate::resources::court::NET_HEIGHT_CENTER;

/// Charge duration to reach 100% power.
const CHARGE_DURATION: f32 = 1.2;

/// Duration past which overcharge begins.
const OVERCHARGE_THRESHOLD: f32 = 1.3;

/// Resource tracking the current shot charge state.
#[derive(Resource, Default)]
pub struct ShotChargeState {
    /// Whether the player is currently charging a shot.
    pub charging: bool,
    /// Accumulated charge time in seconds.
    pub charge_time: f32,
    /// Whether the charge has entered overcharge territory.
    pub overcharged: bool,
}

impl ShotChargeState {
    /// Returns charge power as 0.0 to 1.0 (can exceed 1.0 if overcharged).
    pub fn power(&self) -> f32 {
        (self.charge_time / CHARGE_DURATION).min(1.5)
    }

    /// Returns whether the shot is overcharged.
    pub fn is_overcharged(&self) -> bool {
        self.charge_time > OVERCHARGE_THRESHOLD
    }
}

/// Event emitted when the player releases the charge and fires a shot.
#[derive(Event, Debug)]
pub struct ShotCharged {
    /// Power from 0.0 to 1.0+ (overcharge).
    pub power: f32,
    /// How long the charge was held (seconds).
    pub charge_duration: f32,
    /// Whether the shot was overcharged.
    pub overcharged: bool,
}

/// System that handles shot charging via left mouse button.
pub fn shot_charge_system(
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut charge_state: ResMut<ShotChargeState>,
    mut shot_events: EventWriter<ShotCharged>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        charge_state.charging = true;
        charge_state.charge_time = 0.0;
        charge_state.overcharged = false;
    }

    if charge_state.charging {
        charge_state.charge_time += time.delta_secs();
        charge_state.overcharged = charge_state.is_overcharged();
    }

    if mouse.just_released(MouseButton::Left) && charge_state.charging {
        let power = charge_state.power();
        let duration = charge_state.charge_time;
        let overcharged = charge_state.overcharged;

        shot_events.send(ShotCharged {
            power: power.min(1.0),
            charge_duration: duration,
            overcharged,
        });

        info!(
            "Shot charged: power={:.0}%, duration={:.2}s, overcharged={}",
            power.min(1.0) * 100.0,
            duration,
            overcharged
        );

        charge_state.charging = false;
        charge_state.charge_time = 0.0;
        charge_state.overcharged = false;
    }
}

/// Maximum hit range: how close the ball must be to the player to hit it.
const HIT_RANGE: f32 = 2.5;

/// Base max shot speed at full power (m/s).
const BASE_MAX_SPEED: f32 = 40.0;

/// Base precision radius for scatter (meters).
const BASE_PRECISION_RADIUS: f32 = 0.5;

/// Hit height: ball launch height above ground.
const LAUNCH_HEIGHT: f32 = 1.0;

/// System that executes a shot when ShotCharged fires and ball is near player.
pub fn shot_execution_system(
    mut shot_events: EventReader<ShotCharged>,
    aim_target: Res<AimTarget>,
    player_query: Query<&Transform, (With<Player>, Without<Ball>)>,
    mut ball_query: Query<(&mut Transform, &mut BallState), With<Ball>>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    for event in shot_events.read() {
        let Some(target_pos) = aim_target.position else {
            info!("Shot fired but no aim target on opponent's court");
            continue;
        };

        // Check if ball is near the player
        let player_pos = player_transform.translation;

        for (mut ball_transform, mut ball_state) in ball_query.iter_mut() {
            let ball_pos = ball_transform.translation;
            let dist = (ball_pos - player_pos).length();

            if dist > HIT_RANGE {
                info!("Ball too far from player ({:.1}m > {:.1}m)", dist, HIT_RANGE);
                continue;
            }

            // Apply precision scatter: random offset within reticle radius
            let scatter = random_in_circle(BASE_PRECISION_RADIUS);
            let actual_target = Vec3::new(
                target_pos.x + scatter.x,
                0.0,
                target_pos.z + scatter.y,
            );

            // Compute launch velocity to reach target
            let speed = BASE_MAX_SPEED * event.power;

            // If overcharged, ball goes wild
            if event.overcharged {
                let wild_scatter = random_in_circle(5.0);
                let wild_target = Vec3::new(
                    actual_target.x + wild_scatter.x,
                    0.0,
                    actual_target.z + wild_scatter.y,
                );
                let velocity = compute_launch_velocity(player_pos, wild_target, speed);
                ball_transform.translation = Vec3::new(player_pos.x, LAUNCH_HEIGHT, player_pos.z);
                ball_state.velocity = velocity;
                ball_state.angular_velocity = Vec3::ZERO;
                ball_state.grounded = false;
                info!("OVERCHARGED shot! Ball goes wild toward {:.1}", wild_target);
                continue;
            }

            let velocity = compute_launch_velocity(player_pos, actual_target, speed);

            // Reset ball to launch position
            ball_transform.translation = Vec3::new(player_pos.x, LAUNCH_HEIGHT, player_pos.z);
            ball_state.velocity = velocity;
            ball_state.angular_velocity = Vec3::ZERO;
            ball_state.grounded = false;

            info!(
                "Shot executed: power={:.0}%, target=({:.1}, {:.1}), speed={:.1}m/s",
                event.power * 100.0,
                actual_target.x,
                actual_target.z,
                speed
            );
        }
    }
}

/// Compute launch velocity to send ball from origin toward target at given speed,
/// with enough arc to clear the net (at z=0, height = NET_HEIGHT_CENTER).
fn compute_launch_velocity(from: Vec3, to: Vec3, speed: f32) -> Vec3 {
    let dx = to.x - from.x;
    let dz = to.z - from.z;
    let horizontal_dist = (dx * dx + dz * dz).sqrt();

    if horizontal_dist < 0.1 {
        return Vec3::new(0.0, speed * 0.5, -speed * 0.5);
    }

    // Normalize horizontal direction
    let dir_x = dx / horizontal_dist;
    let dir_z = dz / horizontal_dist;

    // We need to calculate launch angle to clear the net.
    // Net is at z=0. Find what fraction of horizontal distance the net is at.
    // from.z is positive (player side), to.z is negative (opponent side)
    let net_fraction = if dz.abs() > 0.1 {
        (0.0 - from.z) / dz
    } else {
        0.5
    };
    let net_fraction = net_fraction.clamp(0.0, 1.0);

    // Required clearance height at net (add margin)
    let net_clearance = NET_HEIGHT_CENTER + 0.5;

    // Use projectile motion to find launch angle.
    // At fraction t along horizontal, height = launch_height + vy*th - 0.5*g*th^2
    // where th = net_fraction * horizontal_dist / horizontal_speed
    // We want height >= net_clearance at that point.
    //
    // Split speed into horizontal and vertical components:
    // speed^2 = vh^2 + vy^2
    // vh = speed * cos(angle), vy = speed * sin(angle)
    //
    // Simplified approach: use a good default angle and adjust.
    // For tennis groundstrokes, launch angle is typically 5-15 degrees above horizontal.

    // Calculate minimum launch angle to clear net
    let gravity = 9.81;

    // Time to reach net = net_fraction * horizontal_dist / vh
    // Height at net = LAUNCH_HEIGHT + vy * t_net - 0.5 * g * t_net^2
    // We want this >= net_clearance

    // Try a range of angles and pick the lowest that clears
    let mut best_angle = 0.2_f32; // ~11 degrees default

    for angle_deg in 5..45 {
        let angle = (angle_deg as f32).to_radians();
        let vh = speed * angle.cos();
        let vy = speed * angle.sin();

        if vh < 0.1 {
            continue;
        }

        let t_net = net_fraction * horizontal_dist / vh;
        let height_at_net = LAUNCH_HEIGHT + vy * t_net - 0.5 * gravity * t_net * t_net;

        if height_at_net >= net_clearance {
            best_angle = angle;
            break;
        }
    }

    let vh = speed * best_angle.cos();
    let vy = speed * best_angle.sin();

    Vec3::new(dir_x * vh, vy, dir_z * vh)
}

/// Generate a random point within a circle of given radius (uniform distribution).
fn random_in_circle(radius: f32) -> Vec2 {
    // Use rejection sampling for uniform distribution
    loop {
        let x = (rand::random::<f32>() * 2.0 - 1.0) * radius;
        let y = (rand::random::<f32>() * 2.0 - 1.0) * radius;
        if x * x + y * y <= radius * radius {
            return Vec2::new(x, y);
        }
    }
}
