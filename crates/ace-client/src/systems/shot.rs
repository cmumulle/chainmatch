use bevy::prelude::*;
use ace_shared::types::ShotModifier;
use crate::systems::aiming::AimTarget;
use crate::systems::ball_physics::{Ball, BallState};
use crate::systems::input::{ActiveShotModifier, ActiveShotType, SmashAvailable, ShotType};
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

/// Ideal hit distance from player (the "sweet spot").
const IDEAL_HIT_DISTANCE: f32 = 1.0;

// --- Shot Quality / Timing ---

/// Timing quality rating for shots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShotQuality {
    Perfect,
    Good,
    #[default]
    Ok,
    Late,
    Miss,
}

impl ShotQuality {
    /// Map absolute time error (seconds) to quality rating.
    pub fn from_time_error(time_error: f32) -> Self {
        let abs_error = time_error.abs();
        if abs_error < 0.05 {
            ShotQuality::Perfect
        } else if abs_error < 0.12 {
            ShotQuality::Good
        } else if abs_error < 0.20 {
            ShotQuality::Ok
        } else if abs_error < 0.35 {
            ShotQuality::Late
        } else {
            ShotQuality::Miss
        }
    }

    /// Power efficiency multiplier.
    pub fn power_multiplier(&self) -> f32 {
        match self {
            ShotQuality::Perfect => 1.0,
            ShotQuality::Good => 0.9,
            ShotQuality::Ok => 0.75,
            ShotQuality::Late => 0.5,
            ShotQuality::Miss => 0.2,
        }
    }

    /// Precision scatter multiplier (lower = more accurate).
    pub fn precision_multiplier(&self) -> f32 {
        match self {
            ShotQuality::Perfect => 0.5,
            ShotQuality::Good => 0.8,
            ShotQuality::Ok => 1.0,
            ShotQuality::Late => 1.5,
            ShotQuality::Miss => 3.0,
        }
    }

    /// Reticle scale factor (visual preview of precision).
    pub fn reticle_scale(&self) -> f32 {
        match self {
            ShotQuality::Perfect => 0.5,
            ShotQuality::Good => 0.8,
            ShotQuality::Ok => 1.0,
            ShotQuality::Late => 1.5,
            ShotQuality::Miss => 2.0,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ShotQuality::Perfect => "PERFECT!",
            ShotQuality::Good => "GOOD",
            ShotQuality::Ok => "OK",
            ShotQuality::Late => "LATE",
            ShotQuality::Miss => "MISS",
        }
    }
}

/// Resource tracking ball approach for timing quality.
#[derive(Resource)]
pub struct ShotTimingState {
    /// Whether ball is currently within the hit zone.
    pub in_zone: bool,
    /// Time (elapsed_secs) when ball entered the hit zone.
    pub zone_entry_time: f32,
    /// Ball approach speed at zone entry (m/s toward player).
    pub approach_speed: f32,
    /// Current quality preview (what quality you'd get if you hit now).
    pub quality_preview: ShotQuality,
}

impl Default for ShotTimingState {
    fn default() -> Self {
        Self {
            in_zone: false,
            zone_entry_time: 0.0,
            approach_speed: 15.0,
            quality_preview: ShotQuality::Ok,
        }
    }
}

/// Event: shot quality feedback for HUD display.
#[derive(Event, Debug)]
pub struct ShotQualityEvent {
    pub quality: ShotQuality,
}

/// System that tracks ball approach to player and computes timing quality preview.
pub fn ball_approach_tracking_system(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<Ball>)>,
    ball_query: Query<(&Transform, &BallState), (With<Ball>, Without<Player>)>,
    mut timing: ResMut<ShotTimingState>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let now = time.elapsed_secs();

    for (ball_transform, ball_state) in ball_query.iter() {
        if ball_state.grounded {
            timing.in_zone = false;
            timing.quality_preview = ShotQuality::Ok;
            continue;
        }

        let ball_pos = ball_transform.translation;
        let dist = (ball_pos - player_pos).length();

        if dist > HIT_RANGE {
            if timing.in_zone {
                timing.in_zone = false;
                timing.quality_preview = ShotQuality::Miss;
            }
            continue;
        }

        // Ball is within HIT_RANGE
        if !timing.in_zone {
            // Just entered hit zone
            timing.in_zone = true;
            timing.zone_entry_time = now;
            // Compute approach speed (component of velocity toward player)
            let to_player = (player_pos - ball_pos).normalize_or_zero();
            timing.approach_speed = ball_state.velocity.dot(to_player).max(5.0);
        }

        // Compute timing: optimal time = entry + time for ball to travel from
        // zone boundary (HIT_RANGE) to ideal hit distance
        let optimal_delay =
            (HIT_RANGE - IDEAL_HIT_DISTANCE) / timing.approach_speed;
        let optimal_time = timing.zone_entry_time + optimal_delay;
        let time_error = now - optimal_time;

        timing.quality_preview = ShotQuality::from_time_error(time_error);
    }
}

// --- Existing shot systems ---

/// Compute angular velocity (spin) for the given shot modifier.
/// Returns (angular_velocity, speed_multiplier).
fn modifier_spin(modifier: ShotModifier) -> (Vec3, f32) {
    match modifier {
        ShotModifier::Flat => {
            (Vec3::ZERO, 1.0)
        }
        ShotModifier::Topspin => {
            (Vec3::new(80.0, 0.0, 0.0), 0.9)
        }
        ShotModifier::Slice => {
            (Vec3::new(-40.0, 0.0, 0.0), 0.85)
        }
    }
}

/// Minimum ball height above player for smash to be available.
const SMASH_HEIGHT_THRESHOLD: f32 = 2.5;

/// Horizontal distance within which ball must be to player for smash detection.
const SMASH_HORIZONTAL_RANGE: f32 = 3.0;

/// Returns (launch angle offset in degrees, power multiplier, precision_radius) for a shot type.
fn shot_type_params(shot_type: ShotType) -> (f32, f32, f32) {
    match shot_type {
        ShotType::Groundstroke => (0.0, 1.0, BASE_PRECISION_RADIUS),
        ShotType::Lob => (25.0, 0.6, BASE_PRECISION_RADIUS),
        ShotType::DropShot => (-3.0, 0.25, BASE_PRECISION_RADIUS * 0.7),
        ShotType::Smash => (-15.0, 1.5, BASE_PRECISION_RADIUS * 2.0),
    }
}

/// System that detects whether smash is available (ball above player at threshold height).
pub fn smash_detection_system(
    player_query: Query<&Transform, (With<Player>, Without<Ball>)>,
    ball_query: Query<&Transform, (With<Ball>, Without<Player>)>,
    mut smash: ResMut<SmashAvailable>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        smash.0 = false;
        return;
    };

    let player_pos = player_transform.translation;
    let mut available = false;

    for ball_transform in ball_query.iter() {
        let ball_pos = ball_transform.translation;
        let horizontal_dist = Vec2::new(
            ball_pos.x - player_pos.x,
            ball_pos.z - player_pos.z,
        )
        .length();

        if ball_pos.y > SMASH_HEIGHT_THRESHOLD && horizontal_dist < SMASH_HORIZONTAL_RANGE {
            available = true;
            break;
        }
    }

    smash.0 = available;
}

/// System that executes a shot when ShotCharged fires and ball is near player.
/// Applies timing quality multipliers to power and precision.
pub fn shot_execution_system(
    time: Res<Time>,
    mut shot_events: EventReader<ShotCharged>,
    aim_target: Res<AimTarget>,
    active_modifier: Res<ActiveShotModifier>,
    active_shot_type: Res<ActiveShotType>,
    timing: Res<ShotTimingState>,
    player_query: Query<&Transform, (With<Player>, Without<Ball>)>,
    mut ball_query: Query<(&mut Transform, &mut BallState), With<Ball>>,
    mut quality_events: EventWriter<ShotQualityEvent>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    let now = time.elapsed_secs();

    for event in shot_events.read() {
        let Some(target_pos) = aim_target.position else {
            info!("Shot fired but no aim target on opponent's court");
            continue;
        };

        let player_pos = player_transform.translation;
        let modifier = active_modifier.0;
        let shot_type = active_shot_type.0;
        let (spin, speed_mult) = modifier_spin(modifier);
        let (angle_offset, type_power_mult, base_precision) = shot_type_params(shot_type);

        for (mut ball_transform, mut ball_state) in ball_query.iter_mut() {
            let ball_pos = ball_transform.translation;
            let dist = (ball_pos - player_pos).length();

            if dist > HIT_RANGE {
                info!(
                    "Ball too far from player ({:.1}m > {:.1}m)",
                    dist, HIT_RANGE
                );
                continue;
            }

            // Compute timing quality
            let quality = if !timing.in_zone {
                ShotQuality::Miss
            } else {
                let optimal_delay =
                    (HIT_RANGE - IDEAL_HIT_DISTANCE) / timing.approach_speed;
                let optimal_time = timing.zone_entry_time + optimal_delay;
                let time_error = now - optimal_time;
                ShotQuality::from_time_error(time_error)
            };

            quality_events.send(ShotQualityEvent { quality });

            // If overcharged, ball goes wild (ignores timing quality)
            if event.overcharged {
                let wild_scatter = random_in_circle(5.0);
                let wild_target = Vec3::new(
                    target_pos.x + wild_scatter.x,
                    0.0,
                    target_pos.z + wild_scatter.y,
                );
                let speed = BASE_MAX_SPEED * event.power * speed_mult * type_power_mult;
                let velocity = compute_launch_velocity(player_pos, wild_target, speed, 0.0);
                ball_transform.translation =
                    Vec3::new(player_pos.x, LAUNCH_HEIGHT, player_pos.z);
                ball_state.velocity = velocity;
                ball_state.angular_velocity = Vec3::ZERO;
                ball_state.grounded = false;
                info!("OVERCHARGED shot! Ball goes wild toward {:.1}", wild_target);
                continue;
            }

            // Miss quality: shanked shot (random direction, very weak)
            if quality == ShotQuality::Miss {
                let wild_scatter = random_in_circle(4.0);
                let miss_target = Vec3::new(
                    target_pos.x + wild_scatter.x,
                    0.0,
                    target_pos.z + wild_scatter.y,
                );
                let speed =
                    BASE_MAX_SPEED * event.power * 0.2 * speed_mult * type_power_mult;
                let velocity = compute_launch_velocity(player_pos, miss_target, speed, 0.0);
                ball_transform.translation =
                    Vec3::new(player_pos.x, LAUNCH_HEIGHT, player_pos.z);
                ball_state.velocity = velocity;
                ball_state.angular_velocity = Vec3::ZERO;
                ball_state.grounded = false;
                info!("MISS! Shanked shot (bad timing)");
                continue;
            }

            // Apply timing quality multipliers
            let precision_radius = base_precision * quality.precision_multiplier();
            let scatter = random_in_circle(precision_radius);
            let actual_target = Vec3::new(
                target_pos.x + scatter.x,
                0.0,
                target_pos.z + scatter.y,
            );

            let speed = BASE_MAX_SPEED
                * event.power
                * speed_mult
                * type_power_mult
                * quality.power_multiplier();

            let velocity =
                compute_launch_velocity(player_pos, actual_target, speed, angle_offset);

            ball_transform.translation =
                Vec3::new(player_pos.x, LAUNCH_HEIGHT, player_pos.z);
            ball_state.velocity = velocity;
            ball_state.angular_velocity = spin;
            ball_state.grounded = false;

            info!(
                "{} {} {:?} power={:.0}%, speed={:.1}m/s, target=({:.1}, {:.1})",
                quality.display_name(),
                shot_type.display_name(),
                modifier,
                event.power * quality.power_multiplier() * 100.0,
                speed,
                actual_target.x,
                actual_target.z
            );
        }
    }
}

/// Compute launch velocity to send ball from origin toward target at given speed,
/// with enough arc to clear the net (at z=0, height = NET_HEIGHT_CENTER).
/// `angle_offset_deg` adds extra degrees to the launch angle (positive = higher arc).
fn compute_launch_velocity(from: Vec3, to: Vec3, speed: f32, angle_offset_deg: f32) -> Vec3 {
    let dx = to.x - from.x;
    let dz = to.z - from.z;
    let horizontal_dist = (dx * dx + dz * dz).sqrt();

    if horizontal_dist < 0.1 {
        return Vec3::new(0.0, speed * 0.5, -speed * 0.5);
    }

    let dir_x = dx / horizontal_dist;
    let dir_z = dz / horizontal_dist;

    let net_fraction = if dz.abs() > 0.1 {
        (0.0 - from.z) / dz
    } else {
        0.5
    };
    let net_fraction = net_fraction.clamp(0.0, 1.0);

    let net_clearance = NET_HEIGHT_CENTER + 0.5;
    let gravity = 9.81;

    let mut best_angle = 0.2_f32;

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

    let final_angle = (best_angle + angle_offset_deg.to_radians()).clamp(0.05, 1.2);

    let vh = speed * final_angle.cos();
    let vy = speed * final_angle.sin();

    Vec3::new(dir_x * vh, vy, dir_z * vh)
}

/// Generate a random point within a circle of given radius (uniform distribution).
fn random_in_circle(radius: f32) -> Vec2 {
    loop {
        let x = (rand::random::<f32>() * 2.0 - 1.0) * radius;
        let y = (rand::random::<f32>() * 2.0 - 1.0) * radius;
        if x * x + y * y <= radius * radius {
            return Vec2::new(x, y);
        }
    }
}
