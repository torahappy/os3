use crate::spatial_audio::SpatialAudioSink;
use bevy::app::App;
use bevy::prelude::{Resource, Vec3};

pub struct AudioOutputPlugin;

impl bevy::prelude::Plugin for AudioOutputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AudioOutput::new());
    }
}

#[derive(Resource)]
pub struct AudioOutput {
    stream_handle: Option<rodio::MixerDeviceSink>,
}

impl AudioOutput {
    fn new() -> Self {
        if let Ok((stream_handle)) = rodio::DeviceSinkBuilder::open_default_sink() {
            // We leak `OutputStream` to prevent the audio from stopping.
            Self {
                stream_handle: Some(stream_handle),
            }
        } else {
            bevy::log::warn!("No audio output device found.");
            Self {
                stream_handle: None,
            }
        }
    }
    pub fn new_sink(&self) -> Option<SpatialAudioSink> {
        Some(SpatialAudioSink {
            sink: rodio::SpatialPlayer::connect_new(
                self.get()?.mixer(),
                [0.0, 0.0, 0.0],
                (Vec3::X * 4.0 / -2.0).to_array(),
                (Vec3::X * 4.0 / 2.0).to_array(),
            )
        })
    }
    pub fn get(&self) -> Option<&rodio::MixerDeviceSink> {
        self.stream_handle.as_ref()
    }
}
