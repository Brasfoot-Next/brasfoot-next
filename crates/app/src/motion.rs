//! The modern touch over the game's screens: short eased transitions for hover, selection,
//! dialogs and content that changes. Animations ask for the frames they need and nothing more.

use eframe::egui::{self, Color32, Id};

/// How long the transitions take, in seconds.
pub const HOVER: f32 = 0.12;
pub const SELECT: f32 = 0.18;
pub const DIALOG: f32 = 0.16;
pub const CONTENT: f32 = 0.22;

pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// 0 to 1 towards `on`, eased.
pub fn toward(ctx: &egui::Context, id: Id, on: bool, seconds: f32) -> f32 {
    ctx.animate_bool_with_time_and_easing(id, on, seconds, ease_out)
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()), f(a.a(), b.a()))
}

/// 0 to 1 since `key` last changed under `id`: content that was swapped fades in.
pub fn since_change(ctx: &egui::Context, id: Id, key: u64, seconds: f32) -> f32 {
    let now = ctx.input(|i| i.time);
    let started = ctx.data_mut(|d| {
        let entry = d.get_temp_mut_or_insert_with(id, || (key, f64::NEG_INFINITY));
        if entry.0 != key {
            *entry = (key, now);
        }
        entry.1
    });
    let t = ((now - started) as f32 / seconds).clamp(0.0, 1.0);
    if t < 1.0 {
        ctx.request_repaint();
    }
    ease_out(t)
}

/// A modal dialog of the game (the original's are modal `JDialog`s): it fades and grows in
/// over a dimmed screen, and fades out after `open` turns false. Esc closes it.
pub fn dialog(ctx: &egui::Context, name: &str, open: &mut bool, size: egui::Vec2, frame: egui::Frame, content: impl FnOnce(&mut egui::Ui, &mut bool)) {
    let id = Id::unique(("dialog", name));
    let t = toward(ctx, id, *open, DIALOG);
    if t <= 0.0 {
        return;
    }
    let screen = ctx.content_rect();
    // a window smaller than the dialog gets a dialog that fits
    let size = size.min(screen.size() - egui::vec2(24.0, 24.0));
    // the backdrop takes the clicks the main screen would get
    egui::Area::new(id.with("backdrop")).order(egui::Order::Middle).fixed_pos(screen.min).interactable(true).show(ctx, |ui| {
        let (rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::click_and_drag());
        ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha((110.0 * t) as u8));
    });
    let area = egui::Area::new(id).order(egui::Order::Foreground).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).interactable(*open);
    let mut still_open = *open;
    // every dialog the same shape: rounded, with the theme's border
    let frame = frame.corner_radius(12).stroke(egui::Stroke::new(1.0, crate::theme::BORDER));
    let response = area.show(ctx, |ui| {
        ui.multiply_opacity(t);
        frame.shadow(egui::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(140) }).show(ui, |ui| {
            ui.set_min_size(size);
            ui.set_max_size(size);
            content(ui, &mut still_open);
        });
    });
    *open = still_open;
    // grow from 96% around the dialog's center
    let rect = response.response.rect;
    let scale = 0.96 + 0.04 * t;
    let center = rect.center().to_vec2();
    let transform = egui::emath::TSTransform::from_translation(center) * egui::emath::TSTransform::from_scaling(scale) * egui::emath::TSTransform::from_translation(-center);
    ctx.set_transform_layer(response.response.layer_id, transform);
    if *open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        *open = false;
    }
}
