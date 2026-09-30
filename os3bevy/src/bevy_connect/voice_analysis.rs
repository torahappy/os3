use bevy::tasks::futures::check_ready;
use bevy_mod_audio::microphone::MicrophoneAudio;
use num_complex::ComplexFloat;
use crate::math::wave;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::{prelude::*};

#[derive(Resource, Default)]
pub struct VoicePacketData {
    pub tasks: Vec<Task<(f64, Vec<u64>)>>,
    pub history: Vec<(f64, Vec<u64>)>,
}

pub fn system_microphone(mic: ResMut<MicrophoneAudio>, mut vpd: ResMut<VoicePacketData>) {
    // TODO: ! device-dependent ! ADJUST HERE IF Audio Processing went wrong!!
    let ms = 10.0;
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

pub fn system_voice_history(mut data: ResMut<VoicePacketData>) {
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

