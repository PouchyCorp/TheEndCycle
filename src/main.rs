use std::f32::consts::PI;
use bevy::{
    prelude::*, transform
};
use bevy_dev_tools::fps_overlay;

const ITERATIONS_COUNT : u32 = 15;

#[derive(Resource)]
struct MyCursor{
    position : Vec2
}

#[derive(Component)]
struct TargetBall;

struct LimbSegmentConfig {
    mesh: Mesh2d,
    material: MeshMaterial2d<ColorMaterial>,
    length: f32, // Add length field
}

struct JointConfig {
    mesh: Mesh2d,
    material: MeshMaterial2d<ColorMaterial>
}

#[derive(Component)]
struct Bone{
    length : f32
}
#[derive(Component)]
struct Root;

#[derive(Component)]
struct Hand;

#[derive(Component)]
struct Joint{
    target_dir : Vec2
}

impl Root {
    fn new(mut commands: Commands, origin: Vec3, segments: Vec<LimbSegmentConfig>, joint_config : JointConfig) {
        let root: Entity = commands.spawn((
            Transform::from_xyz(origin.x, origin.y, origin.z),
            joint_config.mesh.clone(),
            joint_config.material.clone(),
            Root))
            .id();

        let mut last_entity = root;
        let mut last_bone_length = 0.0; // theoric lenght of the root

        for segment_info in segments {
            
            let joint : Entity = commands.spawn((
                Transform::from_xyz(0.0, last_bone_length / 2.0, 0.0),
                joint_config.mesh.clone(),
                joint_config.material.clone(),
                Joint{target_dir : vec2(1.0, 1.0)}
            )).id();
            commands.entity(last_entity).add_child(joint);
            last_entity = joint;
            
            let bone: Entity = commands.spawn((
                Transform::from_xyz(0.0, segment_info.length / 2.0, 0.0),
                segment_info.mesh,
                segment_info.material,
                Bone{length : segment_info.length}
            )).id();
            commands.entity(last_entity).add_child(bone);
            last_entity = bone;
            last_bone_length = segment_info.length;
        }

        let hand : Entity = commands.spawn((
                Transform::from_xyz(0.0, last_bone_length / 2.0, 0.0),
                Hand
            )).id();
        commands.entity(last_entity).add_child(hand);
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins,
        fps_overlay::FpsOverlayPlugin {
                config: fps_overlay::FpsOverlayConfig {
                    text_config: TextFont {
                        font_size: 10.0,
                        font: default(),
                        ..default()
                    },
                    text_color: Color::srgb(255., 255., 255.),
                    refresh_interval: core::time::Duration::from_millis(100),
                    enabled: true,
                }
        }))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, cursor_system)
        .add_systems(FixedUpdate, (update_target_ball, ik_on_arm))
        .run();
}

fn cursor_system(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut cursor_ressource : ResMut<MyCursor>
) {
    let window = windows.single().expect("huh");
    let (camera, camera_transform) = camera_q.single().expect("huh");

    if let Some(cursor_pixel_position) =  window.cursor_position(){
        let world_pos = camera.viewport_to_world_2d(camera_transform, cursor_pixel_position);
        match world_pos{
            Ok(pos) => {cursor_ressource.position = pos},
            Err(_) => println!("error getting cursor pos")
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d::default());

    commands.insert_resource(MyCursor{position : vec2(0.,0.)});

    let segments = vec![
        LimbSegmentConfig {
            mesh: Mesh2d(meshes.add(Rectangle::new(70.0, 200.0))),
            material: MeshMaterial2d(materials.add(Color::srgb(255.0, 0.0, 0.0))),
            length: 200.0,
        },
        LimbSegmentConfig {
            mesh: Mesh2d(meshes.add(Rectangle::new(50.0, 100.0))),
            material: MeshMaterial2d(materials.add(Color::srgb(0.0, 255.0, 0.0))),
            length: 100.0,
        },
        LimbSegmentConfig {
            mesh: Mesh2d(meshes.add(Rectangle::new(50.0, 150.0))),
            material: MeshMaterial2d(materials.add(Color::srgb(255.0, 255.0, 0.0))),
            length: 150.0,
        },
    ];
    
    commands.spawn((
        Transform::from_xyz(0., 0., 0.),
        TargetBall,
        Mesh2d(meshes.add(Circle::new(10.0))),
        MeshMaterial2d(materials.add(Color::srgb(255.0, 255.0, 255.0)))
    ));


    let joint_config = JointConfig{
        mesh: Mesh2d(meshes.add(Circle::new(30.0))),
        material: MeshMaterial2d(materials.add(Color::srgb(0.0, 0.0, 255.0)))
    };
    Root::new(commands, vec3(0.0, 0.0, 0.0), segments, joint_config);

}

fn update_arm_transforms(mut transforms_joints: Query<(&mut Transform, &Joint)>){
    for (mut transform, joint) in transforms_joints.iter_mut() {
        let quat_dir = transform.rotation * Vec3::Y;
        let quat_dir_2d = Vec2::new(quat_dir.x, quat_dir.y);

        let angle = joint.target_dir.angle_to(quat_dir_2d);
        transform.rotate_z(angle);
    }
}

fn update_target_ball(
    mut ball_transform_query: Query<&mut Transform, With<TargetBall>>,
    my_cursor : Res<MyCursor>
){
    let mut ball_transform = ball_transform_query.single_mut().expect("no ball found");
    
    ball_transform.translation = vec3(my_cursor.position.x, my_cursor.position.y, 0.)
}

fn ik_on_arm(
    root_query: Query<Entity, With<Root>>,
    children: Query<&Children>,
    target: Query<&Transform, (With<TargetBall>, Without<Hand>, Without<Joint>)>,
    mut joint_query: Query<(&GlobalTransform, &mut Transform), (With<Joint>, Without<Hand>)>,
    mut bone_query: Query<(&Bone, &GlobalTransform, &mut Transform)>,
    hand_query: Query<&GlobalTransform, (Without<Joint>, With<Hand>)>
) {
    const ITERATIONS: usize = 15;
    const IK_EPSILON: f32 = 0.5;

    for root_entity in root_query {
        // Gather all entities in the arm chain
        let all_children: Vec<Entity> = children.iter_descendants(root_entity).collect();

        // Collect joint and bone entities in order
        let mut joint_entities = Vec::new();
        let mut bone_entities = Vec::new();
        for &child in &all_children {
            if joint_query.get_mut(child).is_ok() {
                joint_entities.push(child);
            }
            if bone_query.get_mut(child).is_ok() {
                bone_entities.push(child);
            }
        }
        let hand_entity = *all_children.last().expect("the arm does not have a hand");

        // Collect positions and lengths
        let mut positions = Vec::new();
        let mut lengths = Vec::new();

        // Root position
        let root_transform = joint_query.get(root_entity)
            .map(|(gt, _)| gt)
            .unwrap_or_else(|_| panic!("Root entity missing transform"));
        positions.push(root_transform.translation().truncate());

        // For each joint, get its position
        for &joint in &joint_entities {
            let (global, _) = joint_query.get_mut(joint).unwrap();
            positions.push(global.translation().truncate());
        }
        // For each bone, get its length
        for &bone in &bone_entities {
            let (bone_comp, _, _) = bone_query.get_mut(bone).unwrap();
            lengths.push(bone_comp.length);
        }

        // Add hand position
        let hand_transform = hand_query.get(hand_entity).unwrap();
        positions.push(hand_transform.translation().truncate());

        // If the arm is not initialized properly, skip
        if positions.len() < 2 || lengths.len() != positions.len() - 1 {
            continue;
        }

        // FABRIK algorithm
        let target_pos = target.single().expect("no target").translation.truncate();
        let root_pos = positions[0];
        let total_length: f32 = lengths.iter().sum();
        let dist_to_target = (target_pos - root_pos).length();

        // If unreachable, stretch towards target
        if dist_to_target > total_length {
            for i in 0..positions.len() - 1 {
                let dir = (target_pos - positions[i]).normalize();
                positions[i + 1] = positions[i] + dir * lengths[i];
            }
        } else {
            for _ in 0..ITERATIONS {
                // Forward reaching
                positions[positions.len() - 1] = target_pos;
                for i in (0..positions.len() - 1).rev() {
                    let dir = (positions[i] - positions[i + 1]).normalize();
                    positions[i] = positions[i + 1] + dir * lengths[i];
                }
                // Backward reaching
                positions[0] = root_pos;
                for i in 0..positions.len() - 1 {
                    let dir = (positions[i + 1] - positions[i]).normalize();
                    positions[i + 1] = positions[i] + dir * lengths[i];
                }
                // Stop if close enough
                if (positions[positions.len() - 1] - target_pos).length() < IK_EPSILON {
                    break;
                }
            }
        }

        // Update joint rotations to match new positions
        // Skip root (positions[0]), update each joint to point to next position
        for (i, &joint) in joint_entities.iter().enumerate() {
            let (_, mut local_transform) = joint_query.get_mut(joint).unwrap();
            let from = positions[i];
            let to = positions[i + 1];
            let dir = (to - from).normalize();
            let angle = dir.y.atan2(dir.x) - PI / 2.0;
            local_transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}