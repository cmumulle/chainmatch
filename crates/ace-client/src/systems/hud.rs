use bevy::prelude::*;
use crate::systems::shot::ShotChargeState;
use crate::systems::input::{ActiveShotModifier, ActiveShotType, ShotType};
use crate::systems::movement::Player;
use crate::resources::court::CourtEntity;

/// Marker for the shot modifier HUD text.
#[derive(Component)]
pub struct ModifierLabel;

/// Marker for the shot type HUD text.
#[derive(Component)]
pub struct ShotTypeLabel;

/// Marker for the power bar background.
#[derive(Component)]
pub struct PowerBarBg;

/// Marker for the power bar fill.
#[derive(Component)]
pub struct PowerBarFill;

/// Power bar dimensions.
const BAR_WIDTH: f32 = 0.15;
const BAR_HEIGHT: f32 = 1.5;
const BAR_OFFSET_X: f32 = 0.8; // Offset from player to the right
const BAR_OFFSET_Y: f32 = 0.5; // Vertical offset from player base

/// Spawns all HUD elements: power bar + modifier label.
pub fn spawn_hud(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Modifier label (2D UI text, bottom-right corner)
    commands.spawn((
        Text::new("FLAT"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            right: Val::Px(20.0),
            ..default()
        },
        ModifierLabel,
        CourtEntity,
    ));

    // Shot type label (2D UI text, above modifier label)
    commands.spawn((
        Text::new("GROUND"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(48.0),
            right: Val::Px(20.0),
            ..default()
        },
        ShotTypeLabel,
        CourtEntity,
    ));

    let bg_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.1, 0.1, 0.7),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    let fill_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.8, 0.2, 0.9),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // Background bar (always same size)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(BAR_WIDTH, BAR_HEIGHT, 0.02))),
        MeshMaterial3d(bg_mat),
        Transform::from_xyz(0.0, -10.0, 0.0), // Hidden initially
        Visibility::Hidden,
        PowerBarBg,
        CourtEntity,
    ));

    // Fill bar (scales with charge)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(BAR_WIDTH - 0.02, BAR_HEIGHT - 0.02, 0.03))),
        MeshMaterial3d(fill_mat),
        Transform::from_xyz(0.0, -10.0, 0.0), // Hidden initially
        Visibility::Hidden,
        PowerBarFill,
        CourtEntity,
    ));
}

/// System that updates the power bar position and fill based on charge state.
pub fn update_power_bar(
    charge_state: Res<ShotChargeState>,
    player_query: Query<&Transform, (With<Player>, Without<PowerBarBg>, Without<PowerBarFill>)>,
    mut bg_query: Query<
        (&mut Transform, &mut Visibility),
        (With<PowerBarBg>, Without<PowerBarFill>, Without<Player>),
    >,
    mut fill_query: Query<
        (&mut Transform, &mut Visibility, &MeshMaterial3d<StandardMaterial>),
        (With<PowerBarFill>, Without<PowerBarBg>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    let player_pos = player_transform.translation;

    if !charge_state.charging {
        // Hide power bar
        for (_, mut vis) in bg_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
        for (_, mut vis, _) in fill_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let power = charge_state.power().min(1.0);
    let overcharged = charge_state.overcharged;

    // Position bar next to player
    let bar_pos = Vec3::new(
        player_pos.x + BAR_OFFSET_X,
        player_pos.y + BAR_OFFSET_Y + BAR_HEIGHT / 2.0,
        player_pos.z,
    );

    // Update background bar
    for (mut transform, mut vis) in bg_query.iter_mut() {
        transform.translation = bar_pos;
        *vis = Visibility::Visible;
    }

    // Update fill bar: scale Y by power, position anchored at bottom
    let fill_height = (BAR_HEIGHT - 0.02) * power;
    let fill_y = bar_pos.y - (BAR_HEIGHT - 0.02) / 2.0 + fill_height / 2.0;

    for (mut transform, mut vis, mat_handle) in fill_query.iter_mut() {
        transform.translation = Vec3::new(bar_pos.x, fill_y, bar_pos.z + 0.01);
        transform.scale = Vec3::new(1.0, power.max(0.01), 1.0);
        *vis = Visibility::Visible;

        // Change color: green → yellow → red (overcharge)
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            if overcharged {
                mat.base_color = Color::srgba(0.9, 0.1, 0.1, 0.9);
            } else if power > 0.8 {
                mat.base_color = Color::srgba(0.9, 0.9, 0.0, 0.9);
            } else {
                mat.base_color = Color::srgba(0.0, 0.8, 0.2, 0.9);
            }
        }
    }
}

/// System that updates the modifier label text.
pub fn update_modifier_label(
    modifier: Res<ActiveShotModifier>,
    mut query: Query<(&mut Text, &mut TextColor), With<ModifierLabel>>,
) {
    if !modifier.is_changed() {
        return;
    }

    for (mut text, mut color) in query.iter_mut() {
        **text = modifier.display_name().to_string();

        // Color-code modifier
        *color = match modifier.0 {
            ace_shared::types::ShotModifier::Flat => TextColor(Color::WHITE),
            ace_shared::types::ShotModifier::Topspin => TextColor(Color::srgb(0.2, 0.9, 0.2)),
            ace_shared::types::ShotModifier::Slice => TextColor(Color::srgb(0.4, 0.7, 1.0)),
        };
    }
}

/// System that updates the shot type label text.
pub fn update_shot_type_label(
    shot_type: Res<ActiveShotType>,
    mut query: Query<(&mut Text, &mut TextColor), With<ShotTypeLabel>>,
) {
    if !shot_type.is_changed() {
        return;
    }

    for (mut text, mut color) in query.iter_mut() {
        **text = shot_type.0.display_name().to_string();

        *color = match shot_type.0 {
            ShotType::Groundstroke => TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ShotType::Lob => TextColor(Color::srgb(1.0, 0.8, 0.2)),
            ShotType::DropShot => TextColor(Color::srgb(0.9, 0.4, 0.9)),
            ShotType::Smash => TextColor(Color::srgb(1.0, 0.3, 0.1)),
        };
    }
}
