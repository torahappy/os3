// りれきしょ

use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use os3rust::bevy_connect::{
    video::{CustomMaterial, VideoPlayer, VideoResource, cleanup_video, initialize_ffmpeg, play_video},
    window::{WindowMetricsResource, system_window_resize},
};

#[derive(Clone, Copy)]
pub enum MakeFullscreenOption {
    Cover,
    FitWidth,
    FitHeight,
}

#[derive(Component)]
pub struct MakeFullscreen {
    /// (width / height) ratio
    pub ratio: Option<f32>,
    pub option: Option<MakeFullscreenOption>,
}

pub fn system_make_fullscreen(
    mut p: Query<(&MakeFullscreen, &mut Transform)>,
    wm: Res<WindowMetricsResource>,
) {
    p.iter_mut().for_each(|(mf, mut t)| {
        if let Some(ratio) = mf.ratio
            && let Some(opt) = mf.option
        {
            match opt {
                MakeFullscreenOption::Cover => {
                    let window_ratio = wm.window_width / wm.window_height;
                    if (window_ratio>ratio) {
                        t.scale.x = wm.window_width;
                        t.scale.y = wm.window_width / ratio;
                    } else {
                        t.scale.y = wm.window_height;
                        t.scale.x = wm.window_height * ratio;
                    }
                }
                MakeFullscreenOption::FitWidth => {
                    t.scale.x = wm.window_width;
                    t.scale.y = wm.window_width / ratio;
                }
                MakeFullscreenOption::FitHeight => {
                    t.scale.y = wm.window_height;
                    t.scale.x = wm.window_height * ratio;
                }
            }
        } else {
            t.scale.x = wm.window_width;
            t.scale.y = wm.window_height;
        }
    });
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
        .add_systems(Update, system_make_fullscreen)
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
            MakeFullscreen {
                ratio: Some(1.0),
                option: Some(MakeFullscreenOption::Cover),
            }
        ))
        .id();
    video_resource
        .video_players
        .insert(e, video_player_non_send);
}
