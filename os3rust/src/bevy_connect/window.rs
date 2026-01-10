use bevy::{prelude::*, window::WindowResized};

#[derive(Resource, Default, Clone)]
pub struct WindowMetricsResource {
    pub window_width: f32,
    pub window_height: f32,
}

pub fn system_window_resize(
    resize_event: Res<Events<WindowResized>>,
    mut system_data: ResMut<WindowMetricsResource>,
) {
    let mut reader = resize_event.get_cursor();
    for e in reader.read(&resize_event) {
        system_data.window_width = e.width;
        system_data.window_height = e.height;
    }
}

pub fn system_focus_window(mut windows: Query<&mut Window>) {
    let w = windows.single_mut();
    if (w.is_ok()) {
        w.unwrap().focused = true;
    }
}
