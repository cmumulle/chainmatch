use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use crate::systems::aiming::AimTarget;
use crate::systems::ball_physics::{Ball, BallState, ValidBounce, NetFault, OutOfBounds};
use crate::systems::movement::Player;
use crate::resources::court::{
    HALF_COURT_WIDTH, SERVICE_BOX_DEPTH, NET_HEIGHT_CENTER,
};

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

/// Which side to serve from (alternates each point).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServeSide {
    #[default]
    Deuce,
    Ad,
}

impl ServeSide {
    pub fn toggle(self) -> Self {
        match self {
            ServeSide::Deuce => ServeSide::Ad,
            ServeSide::Ad => ServeSide::Deuce,
        }
    }

    /// Returns the target service box bounds (min_x, max_x, min_z, max_z) on the far side.
    /// Deuce: server on right, serves diagonally to opponent's left box (x < 0).
    /// Ad: server on left, serves diagonally to opponent's right box (x > 0).
    pub fn service_box_bounds(self) -> (f32, f32, f32, f32) {
        match self {
            ServeSide::Deuce => (-HALF_COURT_WIDTH, 0.0, -SERVICE_BOX_DEPTH, 0.0),
            ServeSide::Ad => (0.0, HALF_COURT_WIDTH, -SERVICE_BOX_DEPTH, 0.0),
        }
    }

    /// Returns the center of the target service box.
    pub fn service_box_center(self) -> Vec3 {
        let (min_x, max_x, min_z, max_z) = self.service_box_bounds();
        Vec3::new((min_x + max_x) / 2.0, 0.0, (min_z + max_z) / 2.0)
    }
}

/// Serve type affecting spin and trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServeType {
    #[default]
    Flat,
    Slice,
    Kick,
}

impl ServeType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ServeType::Flat => "FLAT",
            ServeType::Slice => "SLICE",
            ServeType::Kick => "KICK",
        }
    }

    fn cycle_next(self) -> Self {
        match self {
            ServeType::Flat => ServeType::Slice,
            ServeType::Slice => ServeType::Kick,
            ServeType::Kick => ServeType::Flat,
        }
    }

    /// Returns (angular_velocity, speed_multiplier) for this serve type.
    /// Slice curves away from receiver; kick has heavy topspin + slight sidespin.
    fn spin_params(self, serve_side: ServeSide) -> (Vec3, f32) {
        let side_sign = match serve_side {
            ServeSide::Deuce => 1.0_f32,
            ServeSide::Ad => -1.0,
        };
        match self {
            ServeType::Flat => (Vec3::ZERO, 1.0),
            ServeType::Slice => (Vec3::new(0.0, side_sign * 40.0, 0.0), 0.95),
            ServeType::Kick => (Vec3::new(80.0, side_sign * 15.0, 0.0), 0.85),
        }
    }
}

/// Resource for active serve type (scroll wheel during serve).
#[derive(Resource, Default)]
pub struct ActiveServeType(pub ServeType);

/// Resource tracking serve state.
#[derive(Resource, Default)]
pub struct ServeState {
    pub phase: ServePhase,
    /// Ball Y at the moment of click (for power/control calculation).
    pub contact_height: f32,
    /// Whether a serve is pending (ball needs to be positioned for serve).
    pub serve_pending: bool,
    /// Which service box to target.
    pub serve_side: ServeSide,
    /// Fault count for current service game (0 = first serve, 1 = second serve).
    pub fault_count: u8,
    /// True when a served ball is in flight (waiting for bounce to validate).
    pub serve_in_flight: bool,
}

/// Event: double fault occurred (two consecutive faults).
#[derive(Event, Debug)]
pub struct DoubleFault;

/// Event: serve fault (net, out, or missed toss).
#[derive(Event, Debug)]
pub struct ServeFault {
    pub fault_number: u8,
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

/// System: Scroll wheel cycles serve type when serve is pending.
pub fn serve_type_cycle_system(
    mut scroll_events: EventReader<MouseWheel>,
    serve_state: Res<ServeState>,
    mut serve_type: ResMut<ActiveServeType>,
) {
    if !serve_state.serve_pending {
        return;
    }
    for event in scroll_events.read() {
        if event.y.abs() > 0.0 || event.x.abs() > 0.0 {
            serve_type.0 = serve_type.0.cycle_next();
            info!("Serve type: {}", serve_type.0.display_name());
        }
    }
}

/// System: Spacebar triggers ball toss during idle serve state.
pub fn serve_toss_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut serve_state: ResMut<ServeState>,
    mut ball_query: Query<(&mut Transform, &mut BallState), With<Ball>>,
    player_query: Query<&Transform, (With<Player>, Without<Ball>)>,
) {
    if serve_state.phase != ServePhase::Idle || !serve_state.serve_pending || serve_state.serve_in_flight {
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
    let serve_num = if serve_state.fault_count == 0 { "1st" } else { "2nd" };
    info!(
        "Serve toss ({} serve, {:?} side)!",
        serve_num, serve_state.serve_side
    );
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
    serve_type: Res<ActiveServeType>,
    mut ball_query: Query<(&mut Transform, &mut BallState), With<Ball>>,
    player_query: Query<&Transform, (With<Player>, Without<Ball>)>,
    mut fault_events: EventWriter<ServeFault>,
    mut double_fault_events: EventWriter<DoubleFault>,
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
            serve_state.fault_count += 1;
            let fault_num = serve_state.fault_count;
            info!("FAULT #{}: Missed the toss (ball too low)", fault_num);
            fault_events.send(ServeFault { fault_number: fault_num });

            if serve_state.fault_count >= 2 {
                info!("DOUBLE FAULT! Point to receiver.");
                double_fault_events.send(DoubleFault);
                serve_state.fault_count = 0;
                serve_state.serve_side = serve_state.serve_side.toggle();
                serve_state.serve_pending = false;
            }

            serve_state.phase = ServePhase::Idle;
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
        serve_state.serve_in_flight = true;

        // Compute power/precision based on contact height
        let height_factor = ((ball_y - MIN_CONTACT_HEIGHT)
            / (MAX_CONTACT_HEIGHT - MIN_CONTACT_HEIGHT))
            .clamp(0.0, 1.0);

        let base_speed = SERVE_MIN_SPEED + height_factor * (SERVE_MAX_SPEED - SERVE_MIN_SPEED);
        let precision =
            SERVE_MIN_PRECISION + height_factor * (SERVE_MAX_PRECISION - SERVE_MIN_PRECISION);

        // Constrain target to correct service box
        let (box_min_x, box_max_x, box_min_z, box_max_z) =
            serve_state.serve_side.service_box_bounds();
        let box_center = serve_state.serve_side.service_box_center();

        let target = match aim_target.position {
            Some(pos) => Vec3::new(
                pos.x.clamp(box_min_x, box_max_x),
                0.0,
                pos.z.clamp(box_min_z, box_max_z),
            ),
            None => box_center,
        };

        // Apply scatter based on precision
        let scatter = random_in_circle(precision);
        let actual_target = Vec3::new(target.x + scatter.x, 0.0, target.z + scatter.y);

        // Get serve type spin and speed multiplier
        let (spin, speed_mult) = serve_type.0.spin_params(serve_state.serve_side);
        let speed = base_speed * speed_mult;

        // Compute serve velocity
        let velocity = compute_serve_velocity(player_pos, actual_target, speed);

        ball_transform.translation = Vec3::new(player_pos.x, ball_y, player_pos.z);
        ball_state.velocity = velocity;
        ball_state.angular_velocity = spin;
        ball_state.grounded = false;

        let serve_num = if serve_state.fault_count == 0 {
            "1st"
        } else {
            "2nd"
        };
        info!(
            "{} SERVE ({})! {:?} side, height={:.2}m, speed={:.1}m/s, target=({:.1}, {:.1})",
            serve_type.0.display_name(),
            serve_num,
            serve_state.serve_side,
            ball_y,
            speed,
            actual_target.x,
            actual_target.z
        );
    }
}

/// System: Check where the served ball lands and determine fault/good serve.
pub fn serve_landing_system(
    mut serve_state: ResMut<ServeState>,
    mut valid_bounces: EventReader<ValidBounce>,
    mut net_faults: EventReader<NetFault>,
    mut out_of_bounds: EventReader<OutOfBounds>,
    mut fault_events: EventWriter<ServeFault>,
    mut double_fault_events: EventWriter<DoubleFault>,
) {
    if !serve_state.serve_in_flight {
        return;
    }

    // Check for net fault during serve
    for _event in net_faults.read() {
        handle_serve_fault(
            &mut serve_state,
            &mut fault_events,
            &mut double_fault_events,
            "net",
        );
        return;
    }

    // Check for out of bounds during serve
    for _event in out_of_bounds.read() {
        handle_serve_fault(
            &mut serve_state,
            &mut fault_events,
            &mut double_fault_events,
            "long/wide",
        );
        return;
    }

    // Check valid bounces — was it in the correct service box?
    for bounce in valid_bounces.read() {
        let pos = bounce.position;
        let (min_x, max_x, min_z, max_z) = serve_state.serve_side.service_box_bounds();

        if pos.x >= min_x && pos.x <= max_x && pos.z >= min_z && pos.z <= max_z {
            // Good serve!
            info!(
                "Serve IN! Ball landed at ({:.1}, {:.1}) in {:?} box",
                pos.x, pos.z, serve_state.serve_side
            );
            serve_state.serve_in_flight = false;
            serve_state.serve_pending = false;
            serve_state.fault_count = 0;
        } else {
            // Landed inside court but outside service box = fault
            handle_serve_fault(
                &mut serve_state,
                &mut fault_events,
                &mut double_fault_events,
                "out of service box",
            );
        }
        return;
    }
}

fn handle_serve_fault(
    serve_state: &mut ResMut<ServeState>,
    fault_events: &mut EventWriter<ServeFault>,
    double_fault_events: &mut EventWriter<DoubleFault>,
    reason: &str,
) {
    serve_state.serve_in_flight = false;
    serve_state.fault_count += 1;
    let fault_num = serve_state.fault_count;

    info!("FAULT #{} ({})", fault_num, reason);
    fault_events.send(ServeFault {
        fault_number: fault_num,
    });

    if serve_state.fault_count >= 2 {
        info!("DOUBLE FAULT! Point to receiver.");
        double_fault_events.send(DoubleFault);
        serve_state.fault_count = 0;
        serve_state.serve_side = serve_state.serve_side.toggle();
        serve_state.serve_pending = false;
    }
    // Otherwise fault_count == 1: second serve. serve_pending stays true.
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
