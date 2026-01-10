// りれきしょ

use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};
use bevy::{prelude::*, render::render_resource::AsBindGroup};
use os3rust::bevy_connect::video::{VideoMaterial, VideoPlayer};
use os3rust::bevy_connect::{
    transform::{AdvTransform, AdvTransformItem, AdvTransformOption, system_adv_transform},
    video::{
        VideoResource, VideoSequence, VideoSequenceConfig, initialize_ffmpeg, play_video,
        system_cleanup_video, system_video_sequence,
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
    #[uniform(3)]
    pub alpha_green: f32,
    #[uniform(4)]
    pub alpha_white: f32,
    #[uniform(5)]
    pub t_1: f32,
    #[uniform(6)]
    pub t_2: f32,
    #[uniform(7)]
    pub t_3: f32,
    #[uniform(8)]
    pub t_4: f32,
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

fn system_video_shaders(
    mut com: Commands,
    q_main: Query<(&MainVideo, Entity, Option<&Children>)>,
    q_text: Query<(&TextVideo, Entity, Option<&Children>)>,
    q_vp: Query<(&VideoPlayer, Option<&MeshMaterial2d<CustomMaterial>>)>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    time: ResMut<Time>,
) {
    let q_all = {
        let mut q_main_ec = q_main.iter().map(|(x, y, z)| (y, z, 0)).collect::<Vec<_>>();
        let q_text_ec = q_text.iter().map(|(x, y, z)| (y, z, 1));
        q_main_ec.extend(q_text_ec);
        q_main_ec
    };

    q_all.iter().for_each(|(e, c, i)| {
        if let Some(c_in) = c {
            let c_vp = c_in.get(0).unwrap();
            let (vp, custom_material) = q_vp.get(*c_vp).unwrap();
            if let Some(cm) = custom_material {
                materials.get_mut(cm.0.id()).unwrap().time += time.delta_secs();
            } else {
                if (*i == 0) {
                    info!("main start");
                    com.entity(*c_vp)
                        .insert(MeshMaterial2d(materials.add(CustomMaterial {
                            color_texture: Some(vp.image_handle.clone()),
                            time: 0.0,
                            alpha_green: 0.0,
                            alpha_white: 1.0,
                            t_1: 0.03,
                            t_2: 0.05,
                            t_3: 0.99,
                            t_4: 0.9,
                        })));
                } else if (*i == 1) {
                    info!("text start");
                    com.entity(*c_vp)
                        .insert(MeshMaterial2d(materials.add(CustomMaterial {
                            color_texture: Some(vp.image_handle.clone()),
                            time: 0.0,
                            alpha_green: 1.0,
                            alpha_white: 0.0,
                            t_1: 0.03,
                            t_2: 0.05,
                            t_3: 0.99,
                            t_4: 0.9,
                        })));
                }
            }
        }
    });
}

fn init_ui(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    commands
        .spawn(VideoSequence {
            custom_material: true,
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
        })
        .insert(TextVideo {});

    commands
        .spawn(VideoSequence {
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
        })
        .insert(MainVideo {});
}
