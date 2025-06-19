use std::f32::consts::PI;
use bevy::{
    prelude::*
};
use bevy_dev_tools::fps_overlay;
use std::collections::HashMap;

#[derive(Resource)]
struct MyCursor{
    position : Vec2
}

#[derive(Component)]
struct Root;

#[derive(Component)]
struct Joint{
    length : f32,
    motion_range_min : f32,
    motion_range_max : f32
}
impl Default for Joint {
    fn default() -> Self {
        Joint { 
            length: 100.0,
            motion_range_min : 180.0,
            motion_range_max : 180.0}
    }
}

#[derive(Resource)]
struct Arms{
    list : HashMap<Entity, Vec<Entity>>
}
impl Arms {
    fn new(
        mut self,
        mut commands : Commands,
        joint_list : Vec<Joint>,
        root_position : Vec3
    ) {
        let root = commands.spawn((
            Root,
            Transform::from_xyz(root_position.x, root_position.y, root_position.z)
        )).id();

        for joint in joint_list{
            let joint = commands.spawn((
                joint,
                Transform::from_xyz(root_position.x, root_position.y, root_position.z) //starts at the roots position
            )).id();

            commands.entity(root).add_child(joint);
        }
    }
}

#[derive(Component)]
struct TargetBall;




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
        .add_systems(FixedUpdate, (cursor_system, update_target_ball))
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

    let segments: Vec<Joint> = vec![
        Joint::default(),
        Joint::default(),
        Joint::default(),
        Joint::default()
    ];
    
    commands.spawn((
        Transform::from_xyz(0., 0., 0.),
        TargetBall,
        Mesh2d(meshes.add(Circle::new(10.0))),
        MeshMaterial2d(materials.add(Color::srgb(255.0, 255.0, 255.0)))
    ));

    //Root::new(commands, vec3(0.0, 0.0, 0.0), segments, joint_config);

}

fn update_target_ball(
    mut ball_transform_query: Query<&mut Transform, With<TargetBall>>,
    my_cursor : Res<MyCursor>
){
    let mut ball_transform = ball_transform_query.single_mut().expect("no ball found");
    
    ball_transform.translation = vec3(my_cursor.position.x, my_cursor.position.y, 0.)
}