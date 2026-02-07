// りれきしょ

use bevy::math::VectorSpace;
use bevy::tasks::futures::check_ready;
use bevy::window::{CursorOptions, WindowResolution};
use bevy_mod_audio::ModAudioPlugins;
use bevy_mod_audio::audio_output::AudioOutput;
use bevy_mod_audio::microphone::MicrophoneAudio;
#[cfg(target_arch = "wasm32")]
use bevy_web_video::{EventListenerAppExt, WebVideoPlugin};
#[cfg(target_arch = "wasm32")]
use os3bevy::bevy_connect::video::wasm_video_termination;
use core::slice;
use futures_lite::future;
use num_complex::ComplexFloat;
use os3bevy::math::wave;
use rand::distr::uniform::SampleRange;
use std::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};
use bevy::tasks::{AsyncComputeTaskPool, IoTaskPool, Task, TaskPool, block_on};
use bevy::{prelude::*, render::render_resource::AsBindGroup};
use bevy_tweening::lens::TransformPositionLens;
use bevy_tweening::{Tween, TweenAnim, TweeningPlugin};
use os3bevy::bevy_connect::transform::{Lifetime, apply_adv_transform, system_lifetime};
use os3bevy::bevy_connect::video::{VideoMaterial, VideoPlayer};
use os3bevy::bevy_connect::{
    transform::{AdvTransform, AdvTransformItem, AdvTransformOption, system_adv_transform},
    video::{
        VideoResource, VideoSequence, VideoSequenceConfig, initialize_ffmpeg, play_video,
        system_cleanup_video, system_video_sequence,
    },
    window::{WindowMetricsResource, system_window_resize},
};
use rand::{prelude::*, rng};

#[derive(Component, Default)]
pub struct VoiceSphere {
    id: usize,
    category: u64,
}

#[derive(Resource, Default)]
pub struct VoicePacketData {
    tasks: Vec<Task<(f64, Vec<u64>)>>,
    history: Vec<(f64, Vec<u64>)>,
}

#[derive(Resource, Default)]
pub struct VoiceGameData {
    cat1_atari_timer: f64,
}

#[derive(Component)]
pub struct MainVideo {}

#[derive(Component)]
pub struct TextVideo {}

#[derive(Component)]
pub struct DrawingBack {}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct VoiceSphereMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(2)]
    pub time: Vec4,
    #[uniform(3)]
    pub category_level_p3_p4: Vec4,
}

const SHADER_ASSET_PATH_VOICESPHERE: &str = "shaders/voicesphere.wgsl";
impl Material2d for VoiceSphereMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH_VOICESPHERE.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

// This is the struct that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct DrawingMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(2)]
    pub time: Vec4,
}

const SHADER_ASSET_PATH_DRAWING: &str = "shaders/drawing_2d.wgsl";
impl Material2d for DrawingMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH_DRAWING.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

// This is the struct that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct RirekiVideoMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(2)]
    pub time: Vec4,
    #[uniform(3)]
    pub green_white_p3_p4: Vec4,
}

const SHADER_ASSET_PATH: &str = "shaders/custom_material_2d.wgsl";
impl Material2d for RirekiVideoMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                mode: bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                ..Default::default()
            }),
            ..default()
        }),
        ModAudioPlugins,
        TweeningPlugin,
        Material2dPlugin::<RirekiVideoMaterial>::default(),
        Material2dPlugin::<VoiceSphereMaterial>::default(),
        Material2dPlugin::<VideoMaterial>::default(),
        Material2dPlugin::<DrawingMaterial>::default(),
        #[cfg(target_arch = "wasm32")]
        WebVideoPlugin,
    ))
    .insert_resource(ClearColor(Color::srgb(0., 0., 0.)))
    .insert_resource(Time::<Fixed>::from_hz(120.0))
    .init_resource::<WindowMetricsResource>()
    .init_resource::<VoicePacketData>()
    .init_resource::<VoiceGameData>()
    .init_non_send_resource::<VideoResource>()
    .add_systems(Startup, init_ui)
    .add_systems(Startup, init_voice_sphere)
    .add_systems(Startup, initialize_ffmpeg)
    .add_systems(FixedUpdate, play_video)
    .add_systems(Update, system_adv_transform)
    .add_systems(Update, system_window_resize)
    .add_systems(Update, system_cleanup_video)
    .add_systems(Update, system_video_sequence)
    .add_systems(Update, system_lifetime)
    .add_systems(Update, system_video_shaders)
    .add_systems(Update, system_spawn_images)
    .add_systems(FixedUpdate, system_voice_queue)
    .add_systems(Update, system_voice_history)
    .add_systems(Update, system_voice_history_calc)
    .add_systems(Update, system_end_condition)
    .add_systems(Update, hide_mouse)
    .add_systems(FixedUpdate, system_microphone);

    app.run();
}

fn system_microphone(mic: ResMut<MicrophoneAudio>, mut vpd: ResMut<VoicePacketData>) {
    let ms = 30.0;
    let samples = ((mic.config.sample_rate as f64) / 1000.0 * ms) as usize;

    let mut mic_in = mic.try_iter().collect::<Vec<Vec<_>>>().concat();

    if mic_in.len() > samples {
        // info!("sent async compute task {} {}", mic_in.len(), samples);
        let task_pool = AsyncComputeTaskPool::get();
        mic_in.truncate(samples);
        let mut slice_left = mic_in.clone();
        let task = task_pool.spawn(async move {
            let max = slice_left
                .iter()
                .map(|x| x.abs())
                .fold(0.0 / 0.0, |a, b| b.max(a));
            slice_left.iter_mut().for_each(|x| *x /= max);
            wave::pre_emphasis_in_place(&mut slice_left, 0.97);
            wave::apply_hamming_in_place(&mut slice_left);
            let result = wave::my_levinson(&slice_left, 32);
            let fft_result = wave::compute_freqz(&result.1, result.0.as_slice(), samples);
            let log_abs = fft_result
                .iter()
                .map(|x| x.abs().log10() * 20.0)
                .collect::<Vec<_>>();
            let mut pf = find_peaks::PeakFinder::new(&log_abs);
            pf.with_min_prominence(10.0);
            let mut peaks = pf
                .find_peaks()
                .iter()
                .map(|x| x.position.start as u64)
                .collect::<Vec<_>>();
            peaks.sort();
            return (result.1 as f64, peaks);
        });
        vpd.tasks.push(task);
    }
}

fn system_end_condition(q: Query<(&VideoSequence, &TextVideo)>, mut ae: MessageWriter<AppExit>) {
    q.iter().for_each(|(vs, _)| {
        if vs.current == 1 {
            ae.write(AppExit::Success);
        }
    });
}

fn hide_mouse(mut cursor_options: Single<&mut CursorOptions>) {
    cursor_options.visible = false;
}

fn init_voice_sphere(
    mut com: Commands,
    q_drawing: Query<&DrawingBack>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<VoiceSphereMaterial>>,
    asset_server: Res<AssetServer>,
) {
    for i in 0..20 {
        com.spawn((
            Mesh2d(meshes.add(Circle::default())),
            MeshMaterial2d(materials.add(VoiceSphereMaterial {
                color_texture: Some(asset_server.load("pictures/voicesphere2.webp")),
                ..default()
            })),
            VoiceSphere { id: i, category: 2 },
            Transform::default()
                .with_scale(Vec3::splat(100.0))
                .with_translation(Vec3 {
                    x: 0.,
                    y: 0.,
                    z: 0.95 + (0.000001 * rng().random_range(0.0..1.0)),
                }),
        ));
    }
    for i in 0..20 {
        com.spawn((
            Mesh2d(meshes.add(Circle::default())),
            MeshMaterial2d(materials.add(VoiceSphereMaterial {
                color_texture: Some(asset_server.load("pictures/voicesphere2.webp")),
                ..default()
            })),
            VoiceSphere { id: i, category: 1 },
            Transform::default()
                .with_scale(Vec3::splat(100.0))
                .with_translation(Vec3 {
                    x: 0.,
                    y: 0.,
                    z: 0.9 + (0.000001 * rng().random_range(0.0..1.0)),
                }),
        ));
    }
}

fn system_voice_history_calc(
    data: Res<VoicePacketData>,
    mut g_data: ResMut<VoiceGameData>,
    mut cc: ResMut<ClearColor>,
    mut q_sphere: Query<(
        &VoiceSphere,
        &mut Transform,
        &MeshMaterial2d<VoiceSphereMaterial>,
    )>,
    wm: Res<WindowMetricsResource>,
    mut materials: ResMut<Assets<VoiceSphereMaterial>>,
    time: Res<Time>,
) {
    let dev_all: f64 =
        data.history.iter().map(|x| x.0 * x.0).sum::<f64>() / data.history.len() as f64;
    let mean_all: f64 = data
        .history
        .iter()
        .map(|x| if x.0.is_nan() { 0.0 } else { x.0 })
        .sum::<f64>()
        / data.history.len() as f64;
    if let Some(last) = data.history.last() {
        let mean_ratio = (mean_all as f32 / last.0 as f32).log10();
        // last.0
        let mr_max = 7.0;
        let mr_processed = (mean_ratio.min(mr_max) / mr_max * 0.7).max(0.0);
        cc.0 = Color::srgb(0.3 + mr_processed, 0.05 + mr_processed, 0.12 + mr_processed);

        // info!("{} {} {}", data.history.len(), last.0 / mean_all, dev_all);
    }

    let mut atari = 0;

    q_sphere.iter_mut().for_each(|(v_data, mut t, matref)| {
        let i = v_data.id;
        if data.history.len() < i + 1 {
            return;
        }
        let item = data.history.get(data.history.len() - i - 1);
        if let Some(h) = item {
            t.translation = Vec3::default();

            let (idx_a, idx_b) = (
                (v_data.category * 2 - 2) as usize,
                (v_data.category * 2 - 1) as usize,
            );

            if h.1.get(idx_a).is_some() && h.1.get(idx_b).is_some() {
                let a = *h.1.get(idx_a).unwrap() as f32;
                let a_large = a / 1200. - 0.5;
                let a_small = a / 100. - 0.5;

                let b = *h.1.get(idx_b).unwrap() as f32;
                let b_large = b / 1200. - 0.5;
                let b_small = b / 200. - 0.5;

                let (a_final, b_final, level) = {
                    if (a_large < -0.4 && b_large < -0.4) {
                        (a_small, b_small, 1)
                    } else {
                        (a_large, b_large, 2)
                    }
                };

                if (v_data.category == 1
                    && level == 1
                    && -0.20 < a_final
                    && a_final < -0.10
                    && -0.05 < b_final
                    && b_final < 0.05)
                {
                    atari += 1;
                }

                let mm = materials.get_mut(matref.id()).unwrap();
                mm.category_level_p3_p4 = Vec4::new(v_data.category as f32, level as f32, 0.0, 0.0);

                apply_adv_transform(
                    &AdvTransform {
                        contents: vec![
                            AdvTransformItem {
                                translate_rel_window: Some((a_final, b_final)),
                                ..default()
                            },
                            AdvTransformItem {
                                scale_mult_rel_window_width: Some((0.053, 0.053)),
                                ..default()
                            },
                        ],
                    },
                    t.into_inner(),
                    &wm,
                );
            }
        }
    });
    if (atari > 3) {
        g_data.cat1_atari_timer += time.delta_secs_f64();
        info!("special!! {}", g_data.cat1_atari_timer);
    } else {
        g_data.cat1_atari_timer = 0.;
    }
}

fn system_voice_history(mut data: ResMut<VoicePacketData>) {
    let mut vectors = Vec::new();
    data.tasks.retain_mut(|x| {
        let status = check_ready(x);
        if let Some(v) = status {
            vectors.push(v);
            // info!("recv");
            return false;
        } else {
            return true;
        }
    });
    data.history.append(&mut vectors);
    if data.history.len() > 1000 {
        let x = data.history[500..]
            .iter()
            .map(|x| x.clone())
            .collect::<Vec<_>>();
        data.history = x;
    }
}

fn system_voice_queue(mut data: ResMut<VoicePacketData>, time: Res<Time>) {
    //    if time.elapsed_secs_f64() - data.last_run > 1.0 / 24.0 {
    //        data.last_run = time.elapsed_secs_f64();
    //        let task_pool = IoTaskPool::get();
    //        let task = task_pool.spawn(async move {
    //            let res = reqwest::blocking::get("http://127.0.0.1:8000/readlines");
    //            match res {
    //                Ok(x) => match x.json() {
    //                    Ok(x) => x,
    //                    Err(e) => {
    //                        info!("Request parse failed ! {}", e);
    //                        Vec::new()
    //                    }
    //                },
    //                Err(e) => {
    //                    info!("{}", e);
    //                    Vec::new()
    //                }
    //            }
    //        });
    //        let i = data.last_task_id;
    //        data.tasks.insert(i, task);
    //        data.last_task_id += 1;
    //    }
}

fn system_spawn_images(
    mut com: Commands,
    q_drawing: Query<&DrawingBack>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<DrawingMaterial>>,
    asset_server: Res<AssetServer>,
    wm: Res<WindowMetricsResource>,
    time: Res<Time>,
) {
    let picture_urls = [
        "pictures/a2000.webp",
        "pictures/b2000.webp",
        "pictures/c2000.webp",
    ];
    let picture_url = picture_urls.choose(&mut rng()).unwrap();
    if (q_drawing.iter().len() == 0) {
        info!("spawn picture");
        let mut com_fork = com.spawn((
            Mesh2d(meshes.add(Rectangle::default())),
            MeshMaterial2d(materials.add(DrawingMaterial {
                color_texture: Some(asset_server.load(*picture_url)),
                time: Vec4::ZERO,
            })),
            DrawingBack {},
        ));

        {
            let advt_start = AdvTransform {
                contents: vec![
                    AdvTransformItem {
                        fullscreen_ratio: Some(1.442380404405721),
                        fullscreen_option: Some(AdvTransformOption::Cover),
                        ..default()
                    },
                    AdvTransformItem {
                        scale_mult: Some((1.0, 1.0)),
                        ..default()
                    },
                    AdvTransformItem {
                        set_z: Some(-1.0),
                        ..default()
                    },
                    AdvTransformItem {
                        translate_mult: Some((0.0, 0.9)),
                        ..default()
                    },
                ],
            };
            let mut t_start = Transform::default();
            apply_adv_transform(&advt_start, &mut t_start, &wm.clone());

            let advt_end = AdvTransform {
                contents: vec![
                    AdvTransformItem {
                        fullscreen_ratio: Some(1.442380404405721),
                        fullscreen_option: Some(AdvTransformOption::Cover),
                        ..default()
                    },
                    AdvTransformItem {
                        scale_mult: Some((1.0, 1.0)),
                        ..default()
                    },
                    AdvTransformItem {
                        set_z: Some(-1.0),
                        ..default()
                    },
                    AdvTransformItem {
                        translate_mult: Some((0.0, -0.9)),
                        ..default()
                    },
                ],
            };
            let mut t_end = Transform::default();
            apply_adv_transform(&advt_end, &mut t_end, &wm.clone());

            info!("{:?} {:?}", t_start, t_end);

            let len = 17.0 + rand::rng().random::<f32>() * 25.0;

            com_fork.insert((
                t_start,
                TweenAnim::new(Tween::new(
                    EaseFunction::QuadraticInOut,
                    // It takes 1 second to go from start to end points.
                    Duration::from_secs_f32(len),
                    // The lens gives access to the Transform component of the Entity,
                    // for the TweenAnimator to animate it. It also contains the start and
                    // end values respectively associated with the progress ratios 0. and 1.
                    TransformPositionLens {
                        start: t_start.translation,
                        end: t_end.translation,
                    },
                )),
                Lifetime {
                    destruct_on: time.elapsed_secs_f64() + ((len * 1.5) as f64),
                },
            ));
        }
    }
}

fn system_video_shaders(
    mut com: Commands,
    q_main: Query<(&MainVideo, Entity, Option<&Children>)>,
    q_text: Query<(&TextVideo, Entity, Option<&Children>)>,
    q_vp: Query<(&VideoPlayer, Option<&MeshMaterial2d<RirekiVideoMaterial>>)>,
    mut materials: ResMut<Assets<RirekiVideoMaterial>>,
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
                        .insert(MeshMaterial2d(materials.add(RirekiVideoMaterial {
                            color_texture: Some(vp.image_handle.clone()),
                            time: Vec4::ZERO,
                            green_white_p3_p4: Vec4::new(0.0, 1.0, 0.0, 0.0),
                        })));
                } else if (*i == 1) {
                    info!("text start");
                    com.entity(*c_vp)
                        .insert(MeshMaterial2d(materials.add(RirekiVideoMaterial {
                            color_texture: Some(vp.image_handle.clone()),
                            time: Vec4::ZERO,
                            green_white_p3_p4: Vec4::new(1.0, 0.0, 0.0, 0.0),
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
            config: vec![VideoSequenceConfig {
                path: "assets/movies/concat_text.webm".to_string(),
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
            }],
            ..default()
        })
        .insert(TextVideo {});

    commands
        .spawn(VideoSequence {
            custom_material: true,
            config: vec![VideoSequenceConfig {
                path: "assets/movies/concat_main.webm".to_string(),
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
                    ],
                },
            }],
            current: 0,
            ..default()
        })
        .insert(MainVideo {});
}
