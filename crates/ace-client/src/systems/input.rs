use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use ace_shared::types::ShotModifier;

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

/// System that cycles shot modifier on scroll wheel input.
pub fn shot_modifier_cycle_system(
    mut scroll_events: EventReader<MouseWheel>,
    mut modifier: ResMut<ActiveShotModifier>,
) {
    for event in scroll_events.read() {
        // Any scroll direction cycles the modifier
        if event.y.abs() > 0.0 || event.x.abs() > 0.0 {
            modifier.cycle_next();
            info!("Shot modifier: {}", modifier.display_name());
        }
    }
}
