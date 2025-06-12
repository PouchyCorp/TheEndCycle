use std::f32::consts::PI;

use bevy::{math::VectorSpace, prelude::*};

struct LimbSegment {
    mesh: Mesh2d,
    material: MeshMaterial2d<ColorMaterial>,
    length: f32, // Add length field
}

#[derive(Component)]
struct LimbPart;

#[derive(Component)]
struct ArmRoot;

impl ArmRoot {
    fn new(mut commands: Commands, origin: Vec3, segments: Vec<LimbSegment>) {
        let root: Entity = commands.spawn((
            Transform::from_xyz(origin.x, origin.y, origin.z),
            ArmRoot))
            .id();

        let mut last_entity = root;
        let mut current_offset = 0.0;

        for segment in segments {
            // Offset by half the segment's length along y
            let limb_segment = commands.spawn((
                Transform::from_xyz(0.0, segment.length / 2.0, 0.0),
                segment.mesh,
                segment.material,
            )).id();
            commands.entity(last_entity).add_child(limb_segment);
            last_entity = limb_segment;
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, rotate_root_segments)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d::default());

    let segments = vec![
        LimbSegment {
            mesh: Mesh2d(meshes.add(Rectangle::new(70.0, 200.0))),
            material: MeshMaterial2d(materials.add(Color::srgb(255.0, 0.0, 0.0))),
            length: 200.0,
        },
        LimbSegment {
            mesh: Mesh2d(meshes.add(Rectangle::new(50.0, 100.0))),
            material: MeshMaterial2d(materials.add(Color::srgb(0.0, 255.0, 0.0))),
            length: 100.0,
        },
    ];

    ArmRoot::new(commands, Vec3::ZERO, segments);
}

fn rotate_root_segments(root_transforms : Query<&mut Transform>){
    for mut transform in root_transforms{
        let axis = Dir3::from_xyz(0.0, 0.0, 1.0);
        match axis{
            Ok(axis) => {transform.rotate_axis(axis, PI/120.0);}
            Err(_) => {println!("err")}
        }
    }
}