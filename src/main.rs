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
    hand_query: Query<&GlobalTransform, (Without<Joint>, With<Hand>)>
) {
    const MAX_ROTATION_PER_ITER: f32 = 10.0 * PI / 180.0; // 10 degrees in radians
    const IK_EPSILON: f32 = 0.5; // Minimum distance to consider as "close enough"

    for root_entity in root_query{
        let all_children: Vec<Entity> = children.iter_descendants(root_entity).collect();

        let mut joints: Vec<Entity> = Vec::new();
        for &child in &all_children {
            if joint_query.get_mut(child).is_ok() {
                joints.push(child);
            }
        }
        let hand: Entity = *all_children.last().expect("the arm does not have a hand");

        let target_transform: &Transform = target.single().expect("no target");
        // do Cyclic Corrdinate IK
        for _ in 0..ITERATIONS_COUNT{
            // do IK
            for joint in joints.iter(){
                let (global_joint_transform, mut joint_transform) =  joint_query.get_mut(*joint).expect("");
                
                // Re-fetch hand transform after each joint rotation for up-to-date position
                let hand_transform = hand_query.get(hand).expect("the arm does not have a hand");
                let joint_pos = global_joint_transform.translation().truncate();
                let hand_pos = hand_transform.translation().truncate();
                let target_pos = target_transform.translation.truncate();

                let vector_joint_hand = hand_pos - joint_pos;
                let vector_joint_target = target_pos - joint_pos;

                // Only rotate if the hand is not close enough to the target
                if vector_joint_hand.length() > 1e-6 && vector_joint_target.length() > 1e-6 && (hand_pos - target_pos).length() > IK_EPSILON {
                    let vector_joint_hand_norm = vector_joint_hand.normalize();
                    let vector_joint_target_norm = vector_joint_target.normalize();
                    let angle = vector_joint_hand_norm.angle_to(vector_joint_target_norm);
                    // Determine the sign of the angle using cross product
                    let cross = vector_joint_hand_norm.perp_dot(vector_joint_target_norm);
                    let signed_angle = if cross < 0.0 { -angle } else { angle };

                    // Clamp the rotation to avoid flailing
                    let clamped_angle = signed_angle.clamp(-MAX_ROTATION_PER_ITER, MAX_ROTATION_PER_ITER);

                    joint_transform.rotation = joint_transform.rotation * Quat::from_rotation_z(clamped_angle);
                }
            }
        }
    }
}