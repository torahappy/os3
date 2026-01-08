// りれきしょ

use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, TextureDimension, TextureFormat, TextureUsages};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};
use std::collections::HashMap;

use ffmpeg_next as ffmpeg;

use ffmpeg::format::{input, Pixel};
use ffmpeg::frame::Video;
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};

// This is the struct that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[uniform(0)]
    color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
}

const SHADER_ASSET_PATH: &str = "shaders/custom_material_2d.wgsl";
impl Material2d for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Mask(0.5)
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Material2dPlugin::<CustomMaterial>::default(),
        ))
        .init_non_send_resource::<VideoResource>()
        .add_systems(Startup, init_ui)
        .add_systems(Startup, initialize_ffmpeg)
        .add_systems(Update, play_video)
        .run();
}

fn init_ui(
    mut commands: Commands,
    images: ResMut<Assets<Image>>,
    mut video_resource: NonSendMut<VideoResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let (video_player, video_player_non_send) =
        VideoPlayer::new("../assets/1.webm", images).unwrap();

    commands.spawn(Camera2d::default());
    let e = commands
        .spawn((
            Mesh2d(meshes.add(Rectangle::default())),
            MeshMaterial2d(materials.add(CustomMaterial {
                color: LinearRgba::BLUE,
                color_texture: Some(video_player.image_handle.clone()),
            })),
            Transform::default().with_scale(Vec3::splat(128.)),
            video_player
        ))
        .id();
    video_resource
        .video_players
        .insert(e, video_player_non_send);
}

fn initialize_ffmpeg() {
    ffmpeg::init().unwrap();
}

// workaround non-send data not being allowed in components by using non-send resource instead
#[derive(Default)]
struct VideoResource {
    video_players: HashMap<Entity, VideoPlayerNonSendData>,
}

struct VideoPlayerNonSendData {
    decoder: ffmpeg::decoder::Video,
    input_context: ffmpeg::format::context::Input,
    scaler_context: Context,
}

#[derive(Component)]
struct VideoPlayer {
    image_handle: Handle<Image>,
    video_stream_index: usize,
}

impl VideoPlayer {
    fn new<'a, P>(
        path: P,
        mut images: ResMut<Assets<Image>>,
    ) -> Result<(VideoPlayer, VideoPlayerNonSendData), ffmpeg::Error>
    where
        P: AsRef<Path>,
    {
        let input_context = input(&path)?;

        // initialize decoder
        let input_stream = input_context
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        let video_stream_index = input_stream.index();

        let context_decoder =
            ffmpeg::codec::context::Context::from_parameters(input_stream.parameters())?;
        let decoder = context_decoder.decoder().video()?;

        // initialize scaler
        let scaler_context = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGBA,
            decoder.width(),
            decoder.height(),
            Flags::BILINEAR,
        )?;

        // create image texture
        let mut image = Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width: decoder.width(),
                height: decoder.height(),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;

        let image_handle = images.add(image);

        Ok((
            VideoPlayer {
                image_handle,
                video_stream_index,
            },
            VideoPlayerNonSendData {
                decoder,
                input_context,
                scaler_context,
            },
        ))
    }
}

fn play_video(
    mut video_player_query: Query<(&mut VideoPlayer, Entity)>,
    mut video_resource: NonSendMut<VideoResource>,
    mut images: ResMut<Assets<Image>>,
) {
    for (video_player, entity) in video_player_query.iter_mut() {
        let video_player_non_send = video_resource.video_players.get_mut(&entity).unwrap();
        // read packets from stream until complete frame received
        while let Some((stream, packet)) = video_player_non_send.input_context.packets().next() {
            // check if packets is for the selected video stream
            if stream.index() == video_player.video_stream_index {
                // pass packet to decoder
                video_player_non_send.decoder.send_packet(&packet).unwrap();
                let mut decoded = Video::empty();
                // check if complete frame was received
                if let Ok(()) = video_player_non_send.decoder.receive_frame(&mut decoded) {
                    let mut rgb_frame = Video::empty();
                    // run frame through scaler for color space conversion
                    video_player_non_send
                        .scaler_context
                        .run(&decoded, &mut rgb_frame)
                        .unwrap();
                    // update data of image texture
                    let image = images.get_mut(&video_player.image_handle).unwrap();
                    image.data = Some(rgb_frame.data(0).to_vec());
                    return;
                }
            }
        }
        // no frame received
        // signal end of playback to decoder
        match video_player_non_send.decoder.send_eof() {
            Err(ffmpeg::Error::Eof) => {}
            other => other.unwrap(),
        }
    }
}
