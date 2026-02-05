use bevy::prelude::*;


#[derive(Component, Default, Clone)]
pub struct Lifetime {
    pub destruct_on: f64
}

pub fn system_lifetime (q: Query<(Entity, &Lifetime)>, mut com: Commands, t: Res<Time>) {
    q.iter().for_each(|(e, l)| {
        if (t.elapsed_secs_f64() > l.destruct_on) {
            com.entity(e).despawn();
        }
    });
}

use crate::bevy_connect::window::WindowMetricsResource;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AdvTransformOption {
    Cover,
    Contain,
    FitWidth,
    FitHeight,
    SameAsWindow,
}

#[derive(Clone, Default)]
pub struct AdvTransformItem {
    /// (width / height) ratio
    pub fullscreen_ratio: Option<f32>,
    pub fullscreen_option: Option<AdvTransformOption>,
    pub translate_mult: Option<(f32, f32)>,
    pub translate_rel_window: Option<(f32, f32)>,
    pub scale_mult: Option<(f32, f32)>,
    pub scale_mult_rel_window_width: Option<(f32, f32)>,
    pub rotate: Option<f32>,
    pub set_z: Option<f32>,
}

#[derive(Component, Default, Clone)]
pub struct AdvTransform {
    pub contents: Vec<AdvTransformItem>,
}

pub fn apply_adv_transform(mfs: &AdvTransform, t: &mut Transform, wm: &WindowMetricsResource) {
    mfs.contents.iter().for_each(|mf| {
        if let Some(ratio) = mf.fullscreen_ratio
            && let Some(opt) = mf.fullscreen_option
        {
            match opt {
                AdvTransformOption::Cover => {
                    let window_ratio = wm.window_width / wm.window_height;
                    if window_ratio > ratio {
                        t.scale.x = wm.window_width;
                        t.scale.y = wm.window_width / ratio;
                    } else {
                        t.scale.y = wm.window_height;
                        t.scale.x = wm.window_height * ratio;
                    }
                }
                AdvTransformOption::FitWidth => {
                    t.scale.x = wm.window_width;
                    t.scale.y = wm.window_width / ratio;
                }
                AdvTransformOption::FitHeight => {
                    t.scale.y = wm.window_height;
                    t.scale.x = wm.window_height * ratio;
                }
                AdvTransformOption::Contain => {
                    let window_ratio = wm.window_width / wm.window_height;
                    if window_ratio < ratio {
                        t.scale.x = wm.window_width;
                        t.scale.y = wm.window_width / ratio;
                    } else {
                        t.scale.y = wm.window_height;
                        t.scale.x = wm.window_height * ratio;
                    }
                }
                AdvTransformOption::SameAsWindow => {
                    warn!("invalid argument for positioning");
                }
            }
        } else if mf.fullscreen_option == Some(AdvTransformOption::SameAsWindow) {
            t.scale.x = wm.window_width;
            t.scale.y = wm.window_height;
        } else if let Some((x, y)) = mf.translate_mult {
            t.translation.x = t.scale.x * x;
            t.translation.y = t.scale.y * y;
        } else if let Some((x, y)) = mf.scale_mult {
            t.scale.x = t.scale.x * x;
            t.scale.y = t.scale.y * y;
        } else if let Some((x, y)) = mf.scale_mult_rel_window_width {
            t.scale.x = wm.window_width * x;
            t.scale.y = wm.window_width * y;
        } else if let Some(x) = mf.rotate {
            t.rotate_z(x);
        } else if let Some(x) = mf.set_z {
            t.translation.z = x;
        } else if let Some((x, y)) = mf.translate_rel_window {
            t.translation.x += wm.window_width * x;
            t.translation.y += wm.window_height * y;
        }
    });
}

pub fn system_adv_transform(
    mut p: Query<(&AdvTransform, &mut Transform)>,
    wm: Res<WindowMetricsResource>,
) {
    p.iter_mut().for_each(|(mfs, mut t)| {
        apply_adv_transform(mfs, t.as_mut(), &wm.clone());
    });
}
