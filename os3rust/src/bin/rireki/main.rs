// りれきしょ

use bevy::shader::ShaderRef;
use bevy::{prelude::*, render::render_resource::AsBindGroup};
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};
use os3rust::bevy_connect::video::VideoMaterial;
use os3rust::bevy_connect::{
    transform::{AdvTransform, AdvTransformItem, AdvTransformOption, system_adv_transform},
    video::{
        VideoResource, VideoSequence, VideoSequenceConfig,
        initialize_ffmpeg, play_video, system_cleanup_video, system_video_sequence,
    },
    window::{WindowMetricsResource, system_window_resize},
};

#[derive(Component)]
pub struct MainVideo {}

#[derive(Component)]
pub struct TextVideo {}

// This is the struct that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CustomMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(2)]
    pub time: f32,
}

const SHADER_ASSET_PATH: &str = "shaders/custom_material_2d.wgsl";
impl Material2d for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Material2dPlugin::<CustomMaterial>::default(),
            Material2dPlugin::<VideoMaterial>::default(),
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
        .add_systems(Update, system_video_shaders)
        .run();
}


fn system_video_shaders (
    com: Commands,
    q_main: Query<(&MainVideo, Entity, Option<&Children>)>,
    q_text: Query<(&TextVideo, Entity, Option<&Children>)>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    m2d: Query<&MeshMaterial2d<CustomMaterial>>,
    time: ResMut<Time>
) {
//    let all_videos
//                            materials
//                                .get_mut(m2d.get(entity).unwrap().0.id())
//                                .unwrap()
//                                .time += time.delta_secs();
//
//                            MeshMaterial2d(materials.add(CustomMaterial {
//                                color_texture: Some(video_player.image_handle.clone()),
//                                time: 0.0,
//                            })),
}

fn init_ui(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    commands.spawn(VideoSequence {
        config: vec![
            VideoSequenceConfig {
                path: "assets/movies/text_1.webm".to_string(),
                fps: 24.0,
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
                        AdvTransformItem {
                            set_z: Some(1.0),
                            ..default()
                        },
                    ],
                },
            },
            VideoSequenceConfig {
                path: "assets/movies/text_2.webm".to_string(),
                fps: 24.0,
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
                        AdvTransformItem {
                            set_z: Some(1.0),
                            ..default()
                        },
                    ],
                },
            },
        ],
        ..default()
    }).insert(TextVideo {});

    commands.spawn(VideoSequence {
        config: vec![
            VideoSequenceConfig {
                path: "assets/movies/1.webm".to_string(),
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
            },
            VideoSequenceConfig {
                path: "assets/movies/2.webm".to_string(),
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
            },
            VideoSequenceConfig {
                path: "assets/movies/3.webm".to_string(),
                fps: 60.0,
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
            },
            VideoSequenceConfig {
                path: "assets/movies/4.webm".to_string(),
                fps: 60.0,
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
            },
            VideoSequenceConfig {
                path: "assets/movies/5.webm".to_string(),
                fps: 60.0,
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
            },
            VideoSequenceConfig {
                path: "assets/movies/6.webm".to_string(),
                fps: 60.0,
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
            },
        ],
        current: 0,
        ..default()
    }).insert(MainVideo {});
}
