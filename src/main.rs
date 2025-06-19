use std::f32::consts::PI;
use bevy::{
    log::tracing_subscriber::filter::targets, prelude::*, render::render_resource::encase::private::Length
};
use bevy_dev_tools::fps_overlay;
use std::collections::HashMap;

// Maximum number of iterations for the FABRIK algorithm
const MAX_FABRIK_ITER_COUNT : u32 = 10;

// Resource to store the current cursor position
#[derive(Resource)]
struct MyCursor{
    position : Vec2
}

// Marker component for the root joint
#[derive(Component)]
struct Root;

// Component representing a joint in the arm
#[derive(Component, Clone)]
struct Joint{
    length : f32,              // Length of the joint segment
    motion_range_min : f32,    // Minimum allowed angle (not used yet)
    motion_range_max : f32     // Maximum allowed angle (not used yet)
}
// Default implementation for Joint
impl Default for Joint {
    fn default() -> Self {
        Joint { 
            length: 100.0,
            motion_range_min : 180.0,
            motion_range_max : 180.0}
    }
}

// Resource to store all arms, mapping root entity to a list of joint entities
#[derive(Resource)]
struct Arms{
    hashmap : HashMap<Entity, Vec<Entity>>
}
impl Arms {
    // Method to create a new arm with a given joint list and root position
    fn new(
        &mut self,
        commands : &mut Commands,
        joint_list : Vec<Joint>,
        root_position : Vec3
    ) {
        let mut arm_vec : Vec<Entity> = Vec::new();

        // Spawn the root entity
        let root = commands.spawn((
            Root,
            Joint{length: 0. , motion_range_max : 180. , motion_range_min : 180.},
            Transform::from_xyz(root_position.x, root_position.y, root_position.z)
        )).id();
        arm_vec.push(root);

        // Prepare joint parameters for IK
        let mut ik_param_vec: Vec<(Joint, Vec3)> = Vec::new();
        ik_param_vec.push((
            Joint{length: 0. , motion_range_max : 180. , motion_range_min : 180.},
            vec3(root_position.x, root_position.y, root_position.z)
        ));
        for joint in &joint_list{
            ik_param_vec.push((joint.clone(), vec3(root_position.x, root_position.y, root_position.z)));
        }
        // Attempt to solve IK using FABRIK
        let ik_result = solve_fabrik(&root_position, &ik_param_vec, &Vec3 { x: 200. , y: 200., z: 200. });

        // updating joint positions after IK
        let mut updated_joint_list: Vec<(Joint, Vec3)> = Vec::new();
        match ik_result{
            Ok(vec) => {
                for pair in vec{
                    updated_joint_list.push((
                        pair.0,
                        pair.1
                    ));
                }
            }
            Err(UnreachableError) => {}
        }

        // Spawn each joint entity at the root position
        for (joint, pos) in updated_joint_list{
            let joint = commands.spawn((
                joint,
                Transform::from_xyz(pos.x, pos.y, pos.z)
            )).id();

            arm_vec.push(joint);
        }

        // Store the arm in the resource
        self.hashmap.insert(
            root, 
            arm_vec
        );
    }
}

impl Default for Arms {
    fn default() -> Self {
        Arms { hashmap: HashMap::new()}
    }
}

// Marker component for the target ball
#[derive(Component)]
struct TargetBall;




// Main entry point
fn main() {
    App::new()
        // Add default plugins and FPS overlay
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
        // Register startup and update systems
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, (
            cursor_system,
            update_target_ball,
            draw_arm_gizmos
        ))
        .run();
}

// System to update the cursor position resource with the current mouse position
fn cursor_system(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut cursor_ressource : ResMut<MyCursor>
) {
    let window = windows.single().expect("huh");
    let (camera, camera_transform) = camera_q.single().expect("huh");

    // Get the cursor position in world coordinates
    if let Some(cursor_pixel_position) =  window.cursor_position(){
        let world_pos = camera.viewport_to_world_2d(camera_transform, cursor_pixel_position);
        match world_pos{
            Ok(pos) => {cursor_ressource.position = pos},
            Err(_) => println!("error getting cursor pos")
        }
    }
}

// Setup system to initialize the scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Spawn the 2D camera
    commands.spawn(Camera2d::default());
    // Insert the cursor resource
    commands.insert_resource(MyCursor{position : vec2(0.,0.)});

    // Define the arm segments
    let segments: Vec<Joint> = vec![
        Joint::default(),
        Joint::default(),
        Joint::default(),
        Joint::default()
    ];
    
    let mut arms = Arms::default();
    arms.new(&mut commands, segments, vec3(0., 0., 0.));
    commands.insert_resource(arms);

    // Spawn the target ball entity
    commands.spawn((
        Transform::from_xyz(0., 0., 0.),
        TargetBall,
        Mesh2d(meshes.add(Circle::new(10.0))),
        MeshMaterial2d(materials.add(Color::srgb(255.0, 255.0, 255.0)))
    ));

    

}

// System to update the target ball's position to follow the cursor
fn update_target_ball(
    mut ball_transform_query: Query<&mut Transform, With<TargetBall>>,
    my_cursor : Res<MyCursor>
){
    let mut ball_transform = ball_transform_query.single_mut().expect("no ball found");
    
    // Set the ball's translation to the cursor position
    ball_transform.translation = vec3(my_cursor.position.x, my_cursor.position.y, 0.)
}


// Error type for unreachable IK targets
#[derive(Debug, Clone)]
struct UnreachableError;

// FABRIK algorithm implementation for inverse kinematics
fn solve_fabrik(
    root: &Vec3,
    joints: &Vec<(Joint, Vec3)>,
    target: &Vec3
) -> Result<Vec<(Joint, Vec3)>, UnreachableError> {
    let joint_count = joints.len() as u32;
    let mut joints: Vec<(Joint, Vec3)> = joints.clone(); // No need for to_vec() after clone()
    
    // Check if target is reachable
    let mut total_length = 0.0;
    for (joint, _) in &joints {
        total_length += joint.length;
    }
    
    if total_length < root.distance(*target) {
        // Target is unreachable
        return Err(UnreachableError);
    }
    
    let tolerance = 0.01; // Define appropriate tolerance
    let mut iter_count: u32 = 0;
    
    // Iterate until the end effector is close enough to the target or max iterations reached
    while joints.last().unwrap().1.distance(*target) > tolerance && iter_count < MAX_FABRIK_ITER_COUNT {
        // Forward pass
        // Set the end effector to the target position
        let (_, hand_pos) = joints.last_mut().unwrap();
        *hand_pos = *target;
        
        // Work backward from end to root
        for i in (1..joint_count).rev() {
            let current_idx = i as usize;
            let anterior_idx = (i-1) as usize;
            
            let direction = (joints[anterior_idx].1 - joints[current_idx].1).normalize();
            joints[anterior_idx].1 = joints[current_idx].1 + direction * joints[current_idx].0.length;
        }
        
        // Backward pass
        // Set the root to its original position
        let (_, root_pos) = joints.first_mut().unwrap();
        *root_pos = *root;
        
        // Work forward from root to end
        for i in 0..(joint_count-1) {
            let current_idx = i as usize;
            let posterior_idx = (i+1) as usize;
            
            let direction = (joints[posterior_idx].1 - joints[current_idx].1).normalize();
            joints[posterior_idx].1 = joints[current_idx].1 + direction * joints[posterior_idx].0.length;
        }
        
        iter_count += 1;
    }
    
    Ok(joints)
}

// System to draw gizmos for all arms and their joints
fn draw_arm_gizmos(
    arms: Res<Arms>,
    transforms: Query<&Transform, With<Joint>>,
    mut gizmos: Gizmos,
) {
    // For each arm in the hashmap
    for arm in arms.hashmap.values() {
        // Draw lines between consecutive joints
        for window in arm.windows(2) {
            if let (Some(a), Some(b)) = (transforms.get(window[0]).ok(), transforms.get(window[1]).ok()) {
                gizmos.line(
                    a.translation,
                    b.translation,
                    Color::WHITE,
                );
            }
        }
    }
}
