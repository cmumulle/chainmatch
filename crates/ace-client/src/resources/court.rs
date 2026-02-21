use bevy::prelude::*;

// Court dimensions in meters (1 unit = 1 meter)
pub const COURT_LENGTH: f32 = 23.77;
pub const COURT_WIDTH: f32 = 8.23; // singles
pub const HALF_COURT_LENGTH: f32 = COURT_LENGTH / 2.0;
pub const HALF_COURT_WIDTH: f32 = COURT_WIDTH / 2.0;

pub const SERVICE_BOX_DEPTH: f32 = 6.40;
pub const SERVICE_BOX_WIDTH: f32 = COURT_WIDTH / 2.0; // 4.115m

pub const NET_HEIGHT_CENTER: f32 = 0.914;
pub const NET_HEIGHT_POSTS: f32 = 1.067;

pub const BASELINE_RUNOFF: f32 = 6.0;
pub const SIDE_RUNOFF: f32 = 3.66;

// Total playable area (court + runoff)
pub const TOTAL_LENGTH: f32 = COURT_LENGTH + 2.0 * BASELINE_RUNOFF;
pub const TOTAL_WIDTH: f32 = COURT_WIDTH + 2.0 * SIDE_RUNOFF;

// Line rendering
const LINE_WIDTH: f32 = 0.05;
const LINE_HEIGHT: f32 = 0.005; // Slightly above court surface

// Colors
const COURT_COLOR: Color = Color::srgb(0.15, 0.35, 0.55); // Blue court
const RUNOFF_COLOR: Color = Color::srgb(0.12, 0.28, 0.45); // Slightly darker
const LINE_COLOR: Color = Color::srgb(1.0, 1.0, 1.0);
const NET_COLOR: Color = Color::srgb(0.85, 0.85, 0.85);

/// Marker component for court entities (for cleanup).
#[derive(Component)]
pub struct CourtEntity;

/// Spawns the full court geometry.
pub fn spawn_court(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let court_mat = materials.add(StandardMaterial {
        base_color: COURT_COLOR,
        perceptual_roughness: 0.9,
        ..default()
    });
    let runoff_mat = materials.add(StandardMaterial {
        base_color: RUNOFF_COLOR,
        perceptual_roughness: 0.9,
        ..default()
    });
    let line_mat = materials.add(StandardMaterial {
        base_color: LINE_COLOR,
        unlit: true,
        ..default()
    });
    let net_mat = materials.add(StandardMaterial {
        base_color: NET_COLOR,
        perceptual_roughness: 0.5,
        ..default()
    });

    // Court surface
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(COURT_WIDTH, COURT_LENGTH))),
        MeshMaterial3d(court_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
        CourtEntity,
    ));

    // Runoff area (larger plane underneath)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(TOTAL_WIDTH, TOTAL_LENGTH))),
        MeshMaterial3d(runoff_mat),
        Transform::from_xyz(0.0, -0.001, 0.0),
        CourtEntity,
    ));

    // --- Court lines ---

    // Baselines (two lines at each end along the width)
    spawn_line(commands, meshes, &line_mat, 0.0, HALF_COURT_LENGTH, COURT_WIDTH, LINE_WIDTH);
    spawn_line(commands, meshes, &line_mat, 0.0, -HALF_COURT_LENGTH, COURT_WIDTH, LINE_WIDTH);

    // Singles sidelines (two lines along the length)
    spawn_line(commands, meshes, &line_mat, HALF_COURT_WIDTH, 0.0, LINE_WIDTH, COURT_LENGTH);
    spawn_line(commands, meshes, &line_mat, -HALF_COURT_WIDTH, 0.0, LINE_WIDTH, COURT_LENGTH);

    // Service lines (parallel to baselines, at service box depth from net)
    spawn_line(commands, meshes, &line_mat, 0.0, SERVICE_BOX_DEPTH, COURT_WIDTH, LINE_WIDTH);
    spawn_line(commands, meshes, &line_mat, 0.0, -SERVICE_BOX_DEPTH, COURT_WIDTH, LINE_WIDTH);

    // Center service line (divides service boxes)
    spawn_line(commands, meshes, &line_mat, 0.0, 0.0, LINE_WIDTH, SERVICE_BOX_DEPTH * 2.0);

    // Center marks on baselines (short perpendicular marks)
    spawn_line(commands, meshes, &line_mat, 0.0, HALF_COURT_LENGTH - 0.1, LINE_WIDTH, 0.2);
    spawn_line(commands, meshes, &line_mat, 0.0, -HALF_COURT_LENGTH + 0.1, LINE_WIDTH, 0.2);

    // --- Net ---
    // Net is a thin box at z=0, spanning the court width + a bit extra for posts
    let net_width = COURT_WIDTH + 0.6; // Slightly wider than court for posts
    let net_thickness = 0.03;

    // Main net (center height)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(net_width, NET_HEIGHT_CENTER, net_thickness))),
        MeshMaterial3d(net_mat.clone()),
        Transform::from_xyz(0.0, NET_HEIGHT_CENTER / 2.0, 0.0),
        CourtEntity,
    ));

    // Net posts (taller at the sides)
    let post_width = 0.08;
    let post_positions = [HALF_COURT_WIDTH + 0.3, -(HALF_COURT_WIDTH + 0.3)];
    for &x in &post_positions {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(post_width, NET_HEIGHT_POSTS, post_width))),
            MeshMaterial3d(net_mat.clone()),
            Transform::from_xyz(x, NET_HEIGHT_POSTS / 2.0, 0.0),
            CourtEntity,
        ));
    }

    // --- Lighting ---
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_6,
            0.0,
        )),
        CourtEntity,
    ));

    // Ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
    });

    // --- Camera ---
    // Behind player 0's baseline, looking toward the net
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 8.0, HALF_COURT_LENGTH + 6.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        CourtEntity,
    ));
}

/// Helper to spawn a line on the court surface.
fn spawn_line(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: &Handle<StandardMaterial>,
    x: f32,
    z: f32,
    width: f32,
    depth: f32,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(width, LINE_HEIGHT, depth))),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(x, LINE_HEIGHT / 2.0, z),
        CourtEntity,
    ));
}
