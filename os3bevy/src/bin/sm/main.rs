// nanikano map
//
use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
    camera::RenderTarget,
    color::palettes::{
        css::{BLACK, WHITE},
        tailwind::PINK_200,
    },
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use bevy_mod_audio::ModAudioPlugins;
use bevy_tweening::TweeningPlugin;
use ffmpeg_next::ffi::daddr_t;
use os3bevy::bevy_connect::voice_analysis::{
    VoiceAnalysisConfig, VoicePacketData, system_microphone, system_voice_history,
};
use rand::{Rng, seq::SliceRandom};
use serde_json::de;

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
}

#[derive(Resource)]
struct GameConfig {
    textbox_w: f32,
    textbox_h: f32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            textbox_w: 1024.,
            textbox_h: 512.,
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
    ))
    .init_asset::<CourtDatabase>()
    .init_asset_loader::<CourtDatabaseLoader>()
    .insert_resource(ClearColor(Color::WHITE))
    .insert_resource(Time::<Fixed>::from_hz(120.0))
    .insert_resource(GameData::default())
    .insert_resource(GameConfig::default())
    .insert_resource(VoiceAnalysisConfig::default())
    .init_resource::<VoicePacketData>()
    .add_systems(Startup, init_game)
    .add_systems(Update, display_name)
    .add_systems(Update, system_voice_history)
    .add_systems(FixedUpdate, system_microphone);

    app.run();
}

fn init_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut gd: ResMut<GameData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut images: ResMut<Assets<Image>>,
    config: Res<GameConfig>,
) {
    commands.spawn(Camera2d::default());

    let database_handle: Handle<CourtDatabase> = asset_server.load("database/combined.json");
    gd.judges_database = Some(database_handle);

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

    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

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
            BackgroundColor(PINK_200.into()),
            UiTargetCamera(texture_camera),
        ))
        .with_children(|com| {
            com.spawn((
                Text::new("今崎幸彦\n安浪亮介\n中村愼"),
                TextFont {
                    font: asset_server.load("fonts/ZenOldMincho-Medium.ttf").into(),
                    font_size: FontSize::Px(config.textbox_h / 3.5),
                    ..default()
                },
                TextColor::BLACK,
            ));
        });

    let mesh_handle = meshes.add(Rectangle::default());

    // This material has the texture that has been rendered.
    let material_handle = materials.add(ColorMaterial {
        texture: Some(image_handle),
        ..default()
    });

    commands.spawn((
        Mesh2d(mesh_handle),
        MeshMaterial2d(material_handle),
        Transform::from_xyz(0.0, 0.0, 1.).with_scale(Vec3::new(
            config.textbox_w,
            config.textbox_h,
            1.,
        )),
    ));
}

fn display_name(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut gd: ResMut<GameData>,
    court_database_asset: Res<Assets<CourtDatabase>>,
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

    if let Some(shuffled) = &gd.judges_shuffled {}
}
