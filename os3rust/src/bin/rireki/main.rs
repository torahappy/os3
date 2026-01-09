// りれきしょ

use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use os3rust::bevy_connect::{
    transform::{AdvTransform, AdvTransformItem, AdvTransformOption, system_adv_transform},
    video::{
        CustomMaterial, VideoPlayer, VideoResource, VideoSequence, VideoSequenceConfig,
        system_cleanup_video, initialize_ffmpeg, play_video, system_video_sequence,
    },
    window::{WindowMetricsResource, system_window_resize},
};

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
        .add_systems(Update, system_cleanup_video)
        .add_systems(Update, system_video_sequence)
        .run();
}

fn init_ui(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    commands.spawn(VideoSequence {
        config: vec![VideoSequenceConfig {
            path: "../assets/1.webm".to_string(),
            fps: 48.0,
            init_adv_transform: AdvTransform {
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
        }],
        current: 0,
        ..default()
    });
}
