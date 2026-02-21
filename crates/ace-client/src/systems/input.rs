use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use ace_shared::types::ShotModifier;
use crate::systems::serve::ServeState;

/// Resource tracking the currently selected shot modifier.
#[derive(Resource)]
pub struct ActiveShotModifier(pub ShotModifier);

impl Default for ActiveShotModifier {
    fn default() -> Self {
        Self(ShotModifier::Flat)
    }
}

impl ActiveShotModifier {
    pub fn display_name(&self) -> &'static str {
        match self.0 {
            ShotModifier::Flat => "FLAT",
            ShotModifier::Topspin => "TOPSPIN",
            ShotModifier::Slice => "SLICE",
        }
    }

    fn cycle_next(&mut self) {
        self.0 = match self.0 {
            ShotModifier::Flat => ShotModifier::Topspin,
            ShotModifier::Topspin => ShotModifier::Slice,
            ShotModifier::Slice => ShotModifier::Flat,
        };
    }
}

/// Active shot type (groundstroke / lob / drop shot / smash).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShotType {
    #[default]
    Groundstroke,
    Lob,
    DropShot,
    Smash,
}

impl ShotType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ShotType::Groundstroke => "GROUND",
            ShotType::Lob => "LOB",
            ShotType::DropShot => "DROP",
            ShotType::Smash => "SMASH",
        }
    }
}

/// Resource tracking whether smash is currently available.
#[derive(Resource, Default)]
pub struct SmashAvailable(pub bool);

/// Resource tracking the currently selected shot type.
#[derive(Resource, Default)]
pub struct ActiveShotType(pub ShotType);

/// System that cycles shot modifier on scroll wheel input.
/// Skips when serve is pending (scroll wheel cycles serve type instead).
pub fn shot_modifier_cycle_system(
    mut scroll_events: EventReader<MouseWheel>,
    mut modifier: ResMut<ActiveShotModifier>,
    serve_state: Res<ServeState>,
) {
    if serve_state.serve_pending {
        return;
    }
    for event in scroll_events.read() {
        if event.y.abs() > 0.0 || event.x.abs() > 0.0 {
            modifier.cycle_next();
            info!("Shot modifier: {}", modifier.display_name());
        }
    }
}

/// System that sets shot type via Q/E keys. Releases revert to Groundstroke.
/// Auto-switches to Smash when smash is available (overrides Q/E).
pub fn shot_type_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    smash: Res<SmashAvailable>,
    mut shot_type: ResMut<ActiveShotType>,
) {
    let prev = shot_type.0;

    if smash.0 {
        shot_type.0 = ShotType::Smash;
    } else if keyboard.pressed(KeyCode::KeyQ) {
        shot_type.0 = ShotType::Lob;
    } else if keyboard.pressed(KeyCode::KeyE) {
        shot_type.0 = ShotType::DropShot;
    } else {
        shot_type.0 = ShotType::Groundstroke;
    }

    if shot_type.0 != prev {
        info!("Shot type: {}", shot_type.0.display_name());
    }
}
