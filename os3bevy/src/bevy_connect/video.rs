use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::math::VectorSpace;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, TextureDimension, TextureFormat, TextureUsages};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d};
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use ffmpeg::format::{Pixel, input};
#[cfg(not(target_arch = "wasm32"))]
use ffmpeg::frame::Video;
#[cfg(not(target_arch = "wasm32"))]
use ffmpeg::media::Type;
#[cfg(not(target_arch = "wasm32"))]
use ffmpeg::software::scaling::{context::Context, flag::Flags};
#[cfg(not(target_arch = "wasm32"))]
use ffmpeg_next::{self as ffmpeg};

use crate::bevy_connect::transform::{AdvTransform, AdvTransformItem, AdvTransformOption};

pub fn initialize_ffmpeg() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        ffmpeg::init().unwrap();
    }
}

// workaround non-send data not being allowed in components by using non-send resource instead
#[derive(Default)]
pub struct VideoResource {
    pub video_players: HashMap<Entity, VideoPlayerNonSendData>,
}

pub struct VideoPlayerNonSendData {
    #[cfg(not(target_arch = "wasm32"))]
    pub decoder: ffmpeg::decoder::Video,
    #[cfg(not(target_arch = "wasm32"))]
    pub input_context: ffmpeg::format::context::Input,
    #[cfg(not(target_arch = "wasm32"))]
    pub scaler_context: Context,
}

#[derive(Component)]
pub struct VideoPlayer {
    pub image_handle: Handle<Image>,
    pub video_stream_index: usize,
    pub fps: f64,
    pub elapsed: f64,
    pub last_sync: f64,
    pub video_end: bool,
}

impl VideoPlayer {
    pub fn new<'a, P>(
        path: P,
        images: &mut ResMut<Assets<Image>>,
        fps: f64,
    ) -> Result<(VideoPlayer, VideoPlayerNonSendData), anyhow::Error>
    where
        P: AsRef<Path>,
    {
        #[cfg(not(target_arch = "wasm32"))]
        let ret = {
            info!("Sprite FPS: {}", fps);
            let input_context = input(&path)?;

            // initialize decoder
            let input_stream = input_context
                .streams()
                .best(Type::Video)
                .ok_or(ffmpeg::Error::StreamNotFound)?;
            let orig_file_rate: f64 = input_stream.rate().into();
            info!("File FPS: {}", orig_file_rate);
            let video_stream_index = input_stream.index();
            let param = input_stream.parameters();
            let context_decoder = ffmpeg::codec::context::Context::from_parameters(param)?;
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
                RenderAssetUsages::all(),
            );
            image.texture_descriptor.usage =
                TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;

            let image_handle = images.add(image);

            Ok((
                VideoPlayer {
                    image_handle,
                    video_stream_index,
                    fps,
                    last_sync: 0.0,
                    elapsed: 0.0,
                    video_end: false,
                },
                VideoPlayerNonSendData {
                    decoder,
                    input_context,
                    scaler_context,
                },
            ))
        };

        #[cfg(target_arch = "wasm32")]
        let ret = {
            let mut image = Image::new_fill(
                bevy::render::render_resource::Extent3d {
                    width: 100,  // TODO
                    height: 100, // TODO
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[255, 255, 255, 255],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::all(),
            );
            image.texture_descriptor.usage =
                TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;

            let image_handle = images.add(image);
            Ok((
                VideoPlayer {
                    image_handle,
                    video_stream_index: 0,
                    fps,
                    last_sync: 0.0,
                    elapsed: 0.0,
                    video_end: false,
                },
                VideoPlayerNonSendData {},
            ))
        };

        return ret;
    }
}

pub fn system_cleanup_video(
    mut com: Commands,
    video_player_query: Query<(&mut VideoPlayer, Entity)>,
    mut video_resource: NonSendMut<VideoResource>,
) {
    let to_clean = video_player_query
        .iter()
        .filter(|(vp, e)| vp.video_end)
        .map(|(vp, e)| e)
        .collect::<Vec<_>>();
    to_clean.iter().for_each(|x| {
        com.entity(*x).despawn();
        video_resource.video_players.remove(x);
    });
}

pub fn play_video(
    mut video_player_query: Query<(&mut VideoPlayer, Entity)>,
    mut video_resource: NonSendMut<VideoResource>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<VideoMaterial>>,
    m2d: Query<&MeshMaterial2d<VideoMaterial>>,
    time: Res<Time>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    for (mut video_player, entity) in video_player_query.iter_mut() {
        video_player.elapsed += time.delta_secs_f64();
        let window = 1.0 / video_player.fps;
        if video_player.elapsed - video_player.last_sync > window {
            let total_frames = video_player.elapsed / window;
            let frames_skipped = ((video_player.elapsed - video_player.last_sync) / window) as u64;
            if frames_skipped > 1 {
                info!("frame skipped: {} {}", entity, frames_skipped);
            }
            video_player.last_sync = total_frames * window;

            if let Some(video_player_non_send) = video_resource.video_players.get_mut(&entity) {
                // read packets from stream until complete frame received
                while frames_skipped > 0
                    && let Some((stream, packet)) =
                        video_player_non_send.input_context.packets().next()
                {
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

                            let frame = rgb_frame.data(0).to_vec();
                            image.data = Some(frame);
                            if m2d.contains(entity) {
                                let m = materials.get_mut(m2d.get(entity).unwrap().0.id());
                                if let Some(mm) = m {
                                    mm.time += time.delta_secs();
                                }
                            }
                            return;
                        }
                    }
                }
                // no frame received
                // signal end of playback to decoder
                match video_player_non_send.decoder.send_eof() {
                    Err(ffmpeg::Error::Eof) => {
                        info!("End of file: send cleanup signal for {}", entity);
                        video_player.video_end = true;
                    }
                    other => other.unwrap(),
                }
            }
        }
    }
}

// This is the struct that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct VideoMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(2)]
    pub time: Vec4,
}

const SHADER_ASSET_PATH: &str = "shaders/video_material_2d.wgsl";
impl Material2d for VideoMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

pub struct VideoSequenceConfig {
    pub path: String,
    pub fps: f64,
    pub init_adv_transform: AdvTransform,
}

#[derive(Component, Default)]
#[require(InheritedVisibility, GlobalTransform, Transform)]
pub struct VideoSequence {
    pub config: Vec<VideoSequenceConfig>,
    pub current: usize,
    pub has_children: bool,
    pub custom_material: bool,
}

pub fn system_video_sequence(
    mut qv: Query<(&mut VideoSequence, Option<&Children>, Entity)>,
    qvp: Query<&VideoPlayer>,
    mut com: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<VideoMaterial>>,
    mut video_resource: NonSendMut<VideoResource>,
) {
    qv.iter_mut().for_each(|(mut vs, c, e)| {
        let should_start: bool = match c {
            Some(_) => {
                vs.has_children = true;
                false
            }
            None => {
                if vs.has_children {
                    vs.current += 1;
                }
                vs.has_children = false;
                true
            }
        };
        if should_start {
            let config = vs.config.get(vs.current);
            if let Some(config_in) = config {
                let (video_player, video_player_non_send) =
                    VideoPlayer::new(config_in.path.clone(), &mut images, config_in.fps).unwrap();
                com.entity(e).with_children(|com2| {
                    let ih = video_player.image_handle.clone();
                    let mut com2_fork = com2.spawn((
                        Mesh2d(meshes.add(Rectangle::default())),
                        Transform::default().with_scale(Vec3::splat(1000.)),
                        video_player,
                        config_in.init_adv_transform.clone(),
                    ));
                    if vs.custom_material == false {
                        com2_fork.insert(MeshMaterial2d(materials.add(VideoMaterial {
                            color_texture: Some(ih),
                            time: Vec4::ZERO,
                        })));
                    }
                    let e2 = com2_fork.id();
                    video_resource
                        .video_players
                        .insert(e2, video_player_non_send);
                });
            }
        }
    });
}
