use std::{cmp::Reverse, collections::BinaryHeap};

// nanikano map
//
use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader}, camera::RenderTarget, color::palettes::css::{BLACK, WHITE}, platform::collections::HashMap, prelude::*, render::render_resource::{
        AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureUsages,
    }, shader::ShaderRef, sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};
use bevy_mod_audio::ModAudioPlugins;
use bevy_tweening::TweeningPlugin;
use os3bevy::bevy_connect::{
    transform::{AdvTransform, AdvTransformItem, AdvTransformOption, system_adv_transform},
    voice_analysis::{
        VoiceAnalysisConfig, VoicePacketData, system_microphone, system_voice_history,
    },
    window::{WindowMetricsResource, system_window_resize},
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

#[derive(Resource)]
struct GameConfig {
    textbox_w: f32,
    textbox_h: f32,
    num_textbox: usize,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            textbox_w: 200.,
            textbox_h: 100.,
            num_textbox: 10,
        }
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
    ))
    .init_asset::<CourtDatabase>()
    .init_asset_loader::<CourtDatabaseLoader>()
    .insert_resource(ClearColor(Color::WHITE))
    .insert_resource(Time::<Fixed>::from_hz(120.0))
    .insert_resource(GameData::default())
    .insert_resource(WindowMetricsResource::default())
    .insert_resource(GameConfig::default())
    .insert_resource(VoiceAnalysisConfig::default())
    .init_resource::<VoicePacketData>()
    .add_systems(Startup, init_game)
    .add_systems(Update, system_animate_name)
    .add_systems(Update, system_window_resize)
    .add_systems(Update, system_voice_history)
    .add_systems(Update, system_adv_transform)
    .add_systems(FixedUpdate, system_microphone);

    app.run();
}

fn init_game(mut commands: Commands, asset_server: Res<AssetServer>, mut gd: ResMut<GameData>) {
    commands.spawn(Camera2d::default());

    let database_handle: Handle<CourtDatabase> = asset_server.load("database/combined.json");
    gd.judges_database = Some(database_handle);
}

fn most_frequent<T>(array: &[T], k: usize) -> Vec<(usize, &T)>
where
    T: std::hash::Hash + Eq + Ord,
{
    let mut map = HashMap::new();
    for x in array {
        *map.entry(x).or_default() += 1;
    }

    let mut heap = BinaryHeap::with_capacity(k + 1);
    for (x, count) in map.into_iter() {
        heap.push(Reverse((count, x)));
        if heap.len() > k {
            heap.pop();
        }
    }
    heap.into_sorted_vec().into_iter().map(|r| r.0).collect()
}

fn system_animate_name(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut gd: ResMut<GameData>,
    court_database_asset: Res<Assets<CourtDatabase>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut name_materials: ResMut<Assets<NameMaterial>>,
    mut images: ResMut<Assets<Image>>,
    config: Res<GameConfig>,
    vpd: Res<VoicePacketData>,
) {
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

    if gd.init_done {
        let mean_all: f64 = vpd
            .history
            .iter()
            .map(|x| if x.0.is_nan() { 0.0 } else { x.0 })
            .sum::<f64>()
            / vpd.history.len() as f64;
        if let Some(last) = vpd.history.last() {
            let mean_ratio = (mean_all as f32 / last.0 as f32).log10();
            let mr_max = 7.0;
            let mr_processed = (mean_ratio.min(mr_max) / mr_max * 0.7).max(0.0);
        }
        let mut top_data = vpd
            .history
            .iter()
            .map(|x| x.1.get(3).cloned())
            .flatten()
            .map(|x| (x / 7, x))
            .collect::<Vec<_>>();
        top_data.sort();
        let s = top_data.iter().map(|x|x.0).collect::<Vec<_>>();
        let a = most_frequent(s.as_slice(), 3);
        info!("{:?}", a);
    }

    if let Some(shuffled) = &gd.judges_shuffled
        && !gd.init_done
    {
        let mut rng = rand::rng();
        let choice = shuffled
            .choose_multiple(&mut rng, 3 * config.num_textbox)
            .cloned()
            .collect::<Vec<_>>()
            .chunks(3)
            .map(|x| x.to_vec())
            .collect::<Vec<_>>();

        for arr in choice {
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
                    for i in arr {
                        com.spawn((
                            Text::new(i.1.clone()),
                            TextFont {
                                font: asset_server.load("fonts/ZenOldMincho-Medium.ttf").into(),
                                font_size: FontSize::Px(config.textbox_h / 3.5),
                                ..default()
                            },
                            TextColor::BLACK,
                        ));
                    }
                });

            commands.spawn((
                Mesh2d(meshes.add(Rectangle::default())),
                MeshMaterial2d(name_materials.add(NameMaterial {
                    color_texture: Some(image_handle),
                })),
                Transform::from_xyz(0.0, 0.0, 10.0 + rand::random_range(0.0..1.0))
                    .with_scale(Vec3::new(1., 1., 1.)),
                AdvTransform {
                    contents: vec![AdvTransformItem {
                        fullscreen_ratio: Some(config.textbox_w / config.textbox_h),
                        fullscreen_option: Some(AdvTransformOption::FitHeight),
                        ..default()
                    }],
                },
            ));
        }

        gd.init_done = true;
    }
}
