#![feature(slice_shift)]
use std::{cmp::Reverse, collections::BinaryHeap};

// nanikano map
//
use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
    camera::RenderTarget,
    color::palettes::css::{BLACK, WHITE},
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::{
        AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureUsages,
    },
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};
use bevy_mod_audio::ModAudioPlugins;
use bevy_tweening::TweeningPlugin;
use num_traits::Pow;
use os3bevy::{
    bevy_connect::{
        transform::{AdvTransform, AdvTransformItem, AdvTransformOption, system_adv_transform},
        voice_analysis::{
            VoiceAnalysisConfig, VoicePacketData, system_microphone, system_voice_history,
        },
        window::{WindowMetricsResource, system_window_resize},
    },
    math::misc::most_frequent,
};
use rand::{
    Rng,
    seq::{IndexedRandom, SliceRandom},
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct NameMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(2)]
    pub time_x_x_x: Vec4,
}

const SHADER_ASSET_PATH: &str = "shaders/name.wgsl";
impl Material2d for NameMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        let alpha_mode2d = AlphaMode2d::Blend;
        alpha_mode2d
    }
}

#[derive(Asset, Reflect, Debug, serde::Deserialize, serde::Serialize)]
struct Court {
    court_name: String,
    judges: Vec<String>,
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
enum CourtDatabaseLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON](ron) Error
    #[error("Could not parse JSON: {0}")]
    ParseFailError(#[from] serde_json::Error),
}

#[derive(Default, Reflect)]
struct CourtDatabaseLoader;

#[derive(Asset, Reflect, Debug, serde::Deserialize, serde::Serialize)]
struct CourtDatabase {
    data: Vec<Court>,
}

impl AssetLoader for CourtDatabaseLoader {
    type Asset = CourtDatabase;
    type Settings = ();
    type Error = CourtDatabaseLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let custom_asset = serde_json::from_slice::<CourtDatabase>(&bytes)?;
        Ok(custom_asset)
    }

    fn extensions(&self) -> &[&str] {
        &["custom"]
    }
}

#[derive(Default, Resource)]
struct GameData {
    judges_database: Option<Handle<CourtDatabase>>,
    /// vector of (所属, 氏名)
    judges_shuffled: Option<Vec<(String, String)>>,
    init_done: bool,
}

#[derive(Default, Component)]
struct TextBox {
    id: usize,
}

#[derive(Default, Component)]
struct OrigUIText {
    textbox_id: usize,
    line_id: usize,
}

#[derive(Default, Component)]
struct HideBoxUpper {}

#[derive(Default, Component)]
struct HideBoxLower {}

#[derive(Default, Resource)]
struct NamePhysics {
    prev: Vec<Option<u32>>,
    current: Vec<u32>,
    pos: Vec<f64>,
    speed: Vec<f64>,
    force: Vec<f64>,
}

#[derive(Resource)]
struct GameConfig {
    textbox_w: f32,
    textbox_h: f32,
    num_textbox: usize,
    num_lines: usize,
    num_sai: usize,
    len_1d_analysis: usize,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            textbox_w: 200.,
            textbox_h: 100.,
            num_textbox: 10,
            num_sai: 20,
            num_lines: 5,
            len_1d_analysis: 50,
        }
    }
}

#[derive(Component, Default)]
pub struct Sai {
    id: usize,
    category: u64,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct SaiMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(2)]
    pub category_id_freq1_freq2: Vec4,
    #[uniform(3)]
    pub color: Vec4,
}

const SHADER_ASSET_PATH_VOICESPHERE: &str = "shaders/sai.wgsl";
impl Material2d for SaiMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH_VOICESPHERE.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

fn main() {
    let mut app = App::new();

    let win: Window = Default::default();

    // #[cfg(target_arch = "wasm32")]
    // let win: Window = Default::default();
    // #[cfg(not(target_arch = "wasm32"))]
    // let win: Window = Window {
    //    mode: bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Current),
    //    ..Default::default()
    //};

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(win),
            ..default()
        }),
        ModAudioPlugins,
        TweeningPlugin,
        Material2dPlugin::<NameMaterial>::default(),
        Material2dPlugin::<SaiMaterial>::default(),
    ))
    .init_asset::<CourtDatabase>()
    .init_asset_loader::<CourtDatabaseLoader>()
    .insert_resource(ClearColor(Color::WHITE))
    .insert_resource(Time::<Fixed>::from_hz(120.0))
    .insert_resource(GameData::default())
    .insert_resource(WindowMetricsResource::default())
    .insert_resource(GameConfig::default())
    .insert_resource(VoiceAnalysisConfig::default())
    .insert_resource(NamePhysics::default())
    .init_resource::<VoicePacketData>()
    .add_systems(Startup, init_game)
    .add_systems(Update, system_animate_sai)
    .add_systems(Update, system_animate_name)
    .add_systems(Update, system_window_resize)
    .add_systems(Update, system_voice_history)
    .add_systems(Update, system_adv_transform)
    .add_systems(Update, system_1d_analysis)
    .add_systems(FixedUpdate, system_apply_physics)
    .add_systems(FixedUpdate, system_microphone);

    app.run();
}

fn init_game(
    mut com: Commands,
    asset_server: Res<AssetServer>,
    mut gd: ResMut<GameData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sai_materials: ResMut<Assets<SaiMaterial>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    config: Res<GameConfig>,
    mut phy: ResMut<NamePhysics>,
) {
    phy.prev = (0..config.num_textbox).map(|_| Some(0)).collect();
    phy.current = (0..config.num_textbox).map(|_| 0).collect();
    phy.pos = (0..config.num_textbox).map(|_| 0.).collect();
    phy.speed = phy.pos.clone();
    phy.force = phy.pos.clone();

    com.spawn(Camera2d::default());

    let database_handle: Handle<CourtDatabase> = asset_server.load("database/combined.json");
    gd.judges_database = Some(database_handle);

    for i in 0..config.num_sai {
        com.spawn((
            Mesh2d(meshes.add(Rectangle::default())),
            MeshMaterial2d(sai_materials.add(SaiMaterial {
                color_texture: Some(asset_server.load("pictures/sai.png")),
                ..default()
            })),
            Sai { id: i, category: 2 },
            Transform::default()
                .with_scale(Vec3::splat(20.0))
                .with_translation(Vec3 {
                    x: 0.,
                    y: 0.,
                    z: 20.0 + (0.000001 * rand::rng().random_range(0.0..1.0)),
                }),
        ));
    }
    for i in 0..config.num_sai {
        com.spawn((
            Mesh2d(meshes.add(Rectangle::default())),
            MeshMaterial2d(sai_materials.add(SaiMaterial {
                color_texture: Some(asset_server.load("pictures/sai.png")),
                ..default()
            })),
            Sai { id: i, category: 1 },
            Transform::default()
                .with_scale(Vec3::splat(20.0))
                .with_translation(Vec3 {
                    x: 0.,
                    y: 0.,
                    z: 21.0 + (0.000001 * rand::rng().random_range(0.0..1.0)),
                }),
        ));
    }
    com.spawn((
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(color_materials.add(ColorMaterial::from_color(Color::WHITE))),
        Transform::from_xyz(0.0, 0.0, 19.0),
        HideBoxUpper {},
    ));
    com.spawn((
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(color_materials.add(ColorMaterial::from_color(Color::WHITE))),
        Transform::from_xyz(0.0, 0.0, 19.0),
        HideBoxLower {},
    ));
}

fn system_apply_physics(
    conf: Res<GameConfig>,
    mut phy: ResMut<NamePhysics>,
    mut q_textbox: Query<(&mut Transform, &TextBox)>,
    time: Res<Time>,
    wm: Res<WindowMetricsResource>,
    mut text: Query<(&mut Text, &OrigUIText)>,
    mut name_materials: ResMut<Assets<NameMaterial>>,
    config: Res<GameConfig>,
    gd: Res<GameData>,
) {
    let mut rng = rand::rng();
    if (!gd.init_done) {
        return;
    };
    let n = phy.pos.len();
    let current = phy.current.clone();
    let prev = phy.prev.clone();
    for i in 0..n {
        if let Some(prev) = prev.get(i).unwrap() {
            let diff = (*current.get(i).unwrap() as i32) - (*prev as i32);
            *phy.force.get_mut(i).unwrap() += (diff as f64).signum() * (diff as f64).abs().pow(0.2);
        }
    }
    let speed = phy.speed.clone();
    let force = phy.force.clone();

    let mut text = text.iter_mut().collect::<Vec<_>>();
    text.sort_by_key(|x| (x.1.textbox_id, x.1.line_id));
    let text_data_clone = text.iter().map(|x|x.0.0.clone()).collect::<Vec<_>>();
    let text_data_clone = text_data_clone.chunks(conf.num_lines).collect::<Vec<_>>();

    for i in 0..n {
        *phy.pos.get_mut(i).unwrap() += speed.get(i).unwrap();
        *phy.speed.get_mut(i).unwrap() += force.get(i).unwrap();
        *phy.speed.get_mut(i).unwrap() *= 0.99;
        *phy.force.get_mut(i).unwrap() = 0.0;
        *phy.prev.get_mut(i).unwrap() = Some(*current.get(i).unwrap());
    }
    for (i, (mut tr, tb)) in q_textbox.iter_mut().enumerate() {
        let c = *phy.pos.get(tb.id).unwrap();
        let restrict = wm.window_height as f64 / (conf.num_lines as f64);
        let orig_y = tr.translation.y as f64;
        if restrict < orig_y || orig_y < -restrict {
            let mut text_array = text_data_clone
                .get(i)
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            let chosen = gd
                .judges_shuffled
                .as_ref()
                .unwrap()
                .choose(&mut rng)
                .unwrap()
                .1
                .clone();
            if restrict < orig_y {
                text_array.shift_left([chosen]);
            } else if orig_y < -restrict {
                text_array.shift_right([chosen]);
            }
            for j in 0..conf.num_lines {
                let x = text.get_mut(conf.num_textbox * i + j);
                if let Some((t, id_info)) = x {
                    assert!(id_info.textbox_id == i);
                    assert!(id_info.line_id == j);
                    t.0 = text_array.get(j).unwrap().clone();
                }
            }
        }

        tr.translation.y = ((c + restrict) % (restrict * 2.0) - restrict) as f32;
    }
}

fn system_1d_analysis(
    mut phy: ResMut<NamePhysics>,
    vpd: Res<VoicePacketData>,
    config: Res<GameConfig>,
) {
    if vpd.history.len() < config.len_1d_analysis {
        return;
    }
    let vec_top = (0..config.num_textbox / 2)
        .map(|i| {
            let recent_data = vpd
                .history
                .split_at(vpd.history.len() - config.len_1d_analysis)
                .1
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            let mut top_data = recent_data
                .iter()
                .map(|x| x.1.get(i).cloned())
                .flatten()
                .map(|x| x)
                .collect::<Vec<_>>();
            top_data.sort();
            most_frequent(top_data.as_slice(), 1)
                .get(0)
                .cloned()
                .map(|x| (x.0, x.1.clone()))
        })
        .collect::<Vec<_>>();
    for (i, top) in vec_top.iter().enumerate() {
        if let Some(top) = top {
            *phy.current.get_mut(i * 2).unwrap() = top.0 as u32;
            *phy.current.get_mut(i * 2 + 1).unwrap() = top.1 as u32;
        }
    }
}

fn system_animate_sai(
    mut commands: Commands,
    mut gd: ResMut<GameData>,
    mut sai_materials: ResMut<Assets<SaiMaterial>>,
    config: Res<GameConfig>,
    vpd: Res<VoicePacketData>,
    mut q_sai: Query<(&Sai, &mut Transform, &MeshMaterial2d<SaiMaterial>)>,
) {
    if vpd.history.len() < 100 {
        return;
    }
    let mean_all: f64 = vpd
        .history
        .iter()
        .map(|x| if x.0.is_nan() { 0.0 } else { x.0 })
        .sum::<f64>()
        / vpd.history.len() as f64;
    if let Some(last) = vpd.history.last() {
        let mean_ratio = (mean_all as f32 / last.0 as f32).log10();
        let mr_max = 7.0;
        let mr_coeff = 0.7;
        let mr_processed = (mean_ratio.min(mr_max) / mr_max * mr_coeff).max(0.0);
    }
    q_sai.iter_mut().for_each(|(sai, mut trans, mat_ref)| {
        let id = sai.id;
        let category = sai.category;

        // get the recent data
        let item = vpd.history.get(vpd.history.len() - id - 1);
    });
}

fn system_animate_name(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut gd: ResMut<GameData>,
    court_database_asset: Res<Assets<CourtDatabase>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut name_materials: ResMut<Assets<NameMaterial>>,
    mut hide_box_upper: Query<&mut Transform, (With<HideBoxUpper>, Without<HideBoxLower>)>,
    mut hide_box_lower: Query<&mut Transform, (With<HideBoxLower>, Without<HideBoxUpper>)>,
    mut images: ResMut<Assets<Image>>,
    config: Res<GameConfig>,
    vpd: Res<VoicePacketData>,
    wm: Res<WindowMetricsResource>,
) {
    let textbox_ratio = config.textbox_w / config.textbox_h;
    let shown_lines = config.num_lines - 2;

    // when FitHeight:
    // let fit_mode = AdvTransformOption::FitHeight;
    // let textbox_scale = (config.num_lines as f32) / (shown_lines as f32);
    // let actual_line_height = wm.window_height / shown_lines as f32;

    // when FitWidth:
    let fit_mode = AdvTransformOption::FitWidth;
    let textbox_scale = 1.0;
    let actual_line_height = wm.window_width / textbox_ratio / config.num_lines as f32;

    {
        let top_y1 = wm.window_height / 2.0;
        let top_y2 = actual_line_height * shown_lines as f32 / 2.0;
        if top_y1 > top_y2 {
            let bottom_y1 = -top_y2;
            let bottom_y2 = -top_y1;
            hide_box_upper.single_mut().unwrap().scale.x = wm.window_width;
            hide_box_upper.single_mut().unwrap().scale.y = bottom_y1 - bottom_y2;
            hide_box_lower.single_mut().unwrap().scale.x = wm.window_width;
            hide_box_lower.single_mut().unwrap().scale.y = bottom_y1 - bottom_y2;
            hide_box_upper.single_mut().unwrap().translation.y = (top_y1 + top_y2) / 2.0;
            hide_box_lower.single_mut().unwrap().translation.y = (bottom_y1 + bottom_y2) / 2.0;
        } else {
            hide_box_upper.single_mut().unwrap().scale = Vec3::splat(0.0);
            hide_box_lower.single_mut().unwrap().scale = Vec3::splat(0.0);
        }
    }

    if let Some(h) = &gd.judges_database {
        if let Some(court_database) = court_database_asset.get(h) {
            if gd.judges_shuffled.is_none() {
                info!("Court Count: {}", court_database.data.len());
                let mut judges = court_database
                    .data
                    .iter()
                    .map(|x| x.judges.iter().map(|y| (x.court_name.clone(), y.clone())))
                    .flatten()
                    .collect::<Vec<_>>();
                let mut rng = rand::rng();
                judges.shuffle(&mut rng);
                info!("Judge Count: {}", judges.len());
                gd.judges_shuffled = Some(judges);
            }
        }
    }

    if gd.init_done {}

    if let Some(shuffled) = &gd.judges_shuffled
        && !gd.init_done
    {
        let mut rng = rand::rng();
        let choice = shuffled
            .choose_multiple(&mut rng, config.num_lines * config.num_textbox)
            .cloned()
            .collect::<Vec<_>>()
            .chunks(config.num_lines)
            .map(|x| x.to_vec())
            .collect::<Vec<_>>();

        for (i, arr) in choice.iter().enumerate() {
            let mut image = Image::new_fill(
                Extent3d {
                    width: config.textbox_w as u32,
                    height: config.textbox_h as u32,
                    ..default()
                },
                TextureDimension::D2,
                &[0, 0, 0, 0],
                TextureFormat::Bgra8UnormSrgb,
                RenderAssetUsages::default(),
            );

            image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT;

            let image_handle = images.add(image);
            let texture_camera = commands
                .spawn((
                    Camera2d,
                    Camera {
                        // render before the "main pass" camera
                        order: -1,
                        ..default()
                    },
                    RenderTarget::Image(image_handle.clone().into()),
                ))
                .id();
            commands
                .spawn((
                    Node {
                        // Cover the whole image
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::WHITE.into()),
                    UiTargetCamera(texture_camera),
                ))
                .with_children(|com| {
                    for (j, place_and_name) in arr.iter().enumerate() {
                        com.spawn((
                            Text::new(place_and_name.1.clone()),
                            TextFont {
                                font: asset_server.load("fonts/ZenOldMincho-Medium.ttf").into(),
                                font_size: FontSize::Px(
                                    config.textbox_h / (config.num_lines as f32 * 1.166666666),
                                ),
                                ..default()
                            },
                            TextColor::BLACK,
                            OrigUIText {
                                textbox_id: i,
                                line_id: j,
                            },
                        ));
                    }
                });

            commands.spawn((
                Mesh2d(meshes.add(Rectangle::default())),
                MeshMaterial2d(name_materials.add(NameMaterial {
                    color_texture: Some(image_handle),
                    time_x_x_x: Vec4::splat(0.0),
                })),
                Transform::from_xyz(0.0, 0.0, 10.0 + rand::random_range(0.0..1.0))
                    .with_scale(Vec3::new(1., 1., 1.)),
                TextBox { id: i },
                AdvTransform {
                    contents: vec![
                        AdvTransformItem {
                            fullscreen_ratio: Some(textbox_ratio),
                            fullscreen_option: Some(fit_mode),
                            ..default()
                        },
                        AdvTransformItem {
                            scale_mult: Some((textbox_scale, textbox_scale)),
                            ..default()
                        },
                    ],
                },
            ));
        }

        gd.init_done = true;
    }
}
