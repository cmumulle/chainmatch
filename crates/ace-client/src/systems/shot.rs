use bevy::prelude::*;

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
