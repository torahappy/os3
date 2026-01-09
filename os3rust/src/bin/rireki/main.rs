// りれきしょ

use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use os3rust::bevy_connect::{
    transform::{AdvTransform, AdvTransformItem, AdvTransformOption, system_adv_transform},
    video::{
        CustomMaterial, VideoPlayer, VideoResource, cleanup_video, initialize_ffmpeg, play_video,
    },
    window::{WindowMetricsResource, system_window_resize},
};

#[derive(Component)]
pub struct VideoSequence {
    pub paths: Vec<String>,
    pub current: usize
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Material2dPlugin::<CustomMaterial>::default(),
        ))
        .init_resource::<WindowMetricsResource>()
        .init_non_send_resource::<VideoResource>()
        .add_systems(Startup, init_ui)
        .add_systems(Startup, initialize_ffmpeg)
        .add_systems(Update, play_video)
        .add_systems(Update, system_adv_transform)
        .add_systems(Update, system_window_resize)
        .add_systems(Update, cleanup_video)
        .run();
}

fn init_ui(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time>,
    mut video_resource: NonSendMut<VideoResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    let (video_player, video_player_non_send) =
        VideoPlayer::new("../assets/1.webm", &mut images, 48.0).unwrap();

    commands.spawn(Camera2d::default());
    let e = commands
        .spawn((
            Mesh2d(meshes.add(Rectangle::default())),
            MeshMaterial2d(materials.add(CustomMaterial {
                color_texture: Some(video_player.image_handle.clone()),
                time: 0.0,
            })),
            Transform::default().with_scale(Vec3::splat(1000.)),
            video_player,
            AdvTransform {
                contents: vec![
                    AdvTransformItem {
                        fullscreen_ratio: Some(2.0),
                        fullscreen_option: Some(AdvTransformOption::Contain),
                        ..default()
                    },
                    AdvTransformItem {
                        scale_mult: Some((1.0, 1.0)),
                        ..default()
                    },
                ],
            },
        ))
        .id();
    video_resource
        .video_players
        .insert(e, video_player_non_send);
}
