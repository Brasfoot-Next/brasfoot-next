//! The look: the mod's modern theme (`bfmod.Visual`, `bfmod.Paineis`) with the Inter font, and
//! the game's club colors on the club panels. Colors live here only.

use std::sync::Arc;

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, Visuals};

/// `Visual.FUNDO`
pub const BACKGROUND: Color32 = Color32::from_rgb(0x12, 0x1a, 0x2b);
/// The window behind the game's panels (`Fundo`).
pub const OUTER: Color32 = Color32::from_rgb(0x0b, 0x11, 0x1e);
/// `Visual.PAINEL`
pub const PANEL: Color32 = Color32::from_rgb(0x17, 0x21, 0x36);
/// `Visual.LINHA_ALT`
pub const ROW_ALT: Color32 = Color32::from_rgb(0x1b, 0x26, 0x3e);
/// `Visual.CABECALHO`
pub const HEADER: Color32 = Color32::from_rgb(0x0f, 0x16, 0x25);
/// `Visual.BORDA`
pub const BORDER: Color32 = Color32::from_rgb(0x26, 0x32, 0x4a);
/// `Visual.TEXTO`
pub const TEXT: Color32 = Color32::from_rgb(0xdc, 0xe2, 0xec);
/// `Visual.TEXTO_SUAVE`
pub const MUTED: Color32 = Color32::from_rgb(0x8e, 0x9a, 0xb3);
/// `Visual.DESTAQUE`
pub const ACCENT: Color32 = Color32::from_rgb(0x3b, 0x82, 0xf6);
/// `Visual.SELECAO`
pub const SELECTION: Color32 = Color32::from_rgb(0x27, 0x4a, 0x86);
/// The grid lines of the table headers.
pub const GRID: Color32 = Color32::from_rgb(0x3a, 0x47, 0x63);
/// A row under the mouse (the modern touch; the game has no hover).
pub const HOVER: Color32 = Color32::from_rgb(0x24, 0x34, 0x55);
/// A table card under the mouse.
pub const CARD_HOVER: Color32 = Color32::from_rgb(0x2b, 0x3b, 0x5e);
/// `Paineis.PAINEL_ALTO`
pub const PANEL_HIGH: Color32 = Color32::from_rgb(0x22, 0x30, 0x4d);
/// `Paineis.CARTAO`
pub const CARD: Color32 = Color32::from_rgb(0x1f, 0x2b, 0x45);
/// The game's dark buttons ("Escalar Time").
pub const BUTTON: Color32 = Color32::from_rgb(0x27, 0x2f, 0x3e);
/// `Visual.ENERGIA_ALTA`, `ENERGIA_MEDIA`, `ENERGIA_BAIXA`
pub const ENERGY_HIGH: Color32 = Color32::from_rgb(0x3f, 0xa6, 0x6b);
pub const ENERGY_MID: Color32 = Color32::from_rgb(0xc9, 0xa2, 0x27);
pub const ENERGY_LOW: Color32 = Color32::from_rgb(0xd0, 0x52, 0x52);
/// `Ref.colunaRating`, dark theme.
pub const RATING_HIGH: Color32 = Color32::from_rgb(0x4c, 0xc3, 0x8a);
pub const RATING_MID: Color32 = Color32::from_rgb(0xc9, 0xa2, 0x27);
pub const RATING_LOW: Color32 = Color32::from_rgb(0xe6, 0x67, 0x67);
/// The game's navy and red texts after `Visual.texto` (age column, contract warnings).
pub const TEXT_BLUE: Color32 = Color32::from_rgb(0x8c, 0xb0, 0xff);
pub const TEXT_RED: Color32 = Color32::from_rgb(0xff, 0x8c, 0x8c);
pub const TEXT_GREEN: Color32 = Color32::from_rgb(0x8c, 0xff, 0x9b);
/// The tables' bands (`/aiconsc/`): neutral, promotion, relegation and the other places.
pub const BAND_NEUTRAL: Color32 = Color32::from_rgb(0xc4, 0xc5, 0xc6);
pub const BAND_GREEN: Color32 = Color32::from_rgb(0x2e, 0xa0, 0x43);
pub const BAND_RED: Color32 = Color32::from_rgb(0xbf, 0x02, 0x01);
pub const BAND_BLUE: Color32 = Color32::from_rgb(0x2f, 0x6f, 0xd6);
pub const BAND_YELLOW: Color32 = Color32::from_rgb(0xe0, 0xb8, 0x1d);
pub const STAR: Color32 = Color32::from_rgb(0xf5, 0xc5, 0x42);
pub const BAD: Color32 = Color32::from_rgb(0xe5, 0x48, 0x4d);
pub const GOOD: Color32 = Color32::from_rgb(0x4c, 0xc3, 0x8a);
pub const WARN: Color32 = Color32::from_rgb(0xc9, 0xa2, 0x27);
pub const INFO: Color32 = ACCENT;

pub fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

pub fn icons_family() -> FontFamily {
    FontFamily::Name("icons".into())
}

/// The font of the icons (`glyphs`).
pub fn icon_font(size: f32) -> FontId {
    FontId::new(size, icons_family())
}

pub fn regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, bold_family())
}

pub fn apply(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("inter".into(), Arc::new(FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf"))));
    fonts.font_data.insert("inter-bold".into(), Arc::new(FontData::from_static(include_bytes!("../assets/fonts/Inter-Bold.ttf"))));
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "inter".into());
    let mut bold_fonts: Vec<String> = vec!["inter-bold".into()];
    bold_fonts.extend(fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
    fonts.families.insert(bold_family(), bold_fonts);
    // the icons (`glyphs`): their own family, and a fallback of the text fonts
    fonts.font_data.insert("phosphor".into(), Arc::new(FontData::from_static(include_bytes!("../assets/fonts/Phosphor-Fill.ttf"))));
    fonts.families.insert(icons_family(), vec!["phosphor".into()]);
    for family in [FontFamily::Proportional, bold_family()] {
        fonts.families.entry(family).or_default().push("phosphor".into());
    }
    ctx.set_fonts(fonts);

    let mut v = Visuals::dark();
    v.panel_fill = BACKGROUND;
    v.window_fill = BACKGROUND;
    v.extreme_bg_color = HEADER;
    v.faint_bg_color = ROW_ALT;
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = SELECTION;
    v.selection.stroke = Stroke::new(1.0, TEXT);
    v.hyperlink_color = ACCENT;
    v.window_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.inactive.bg_fill = CARD;
    v.widgets.inactive.weak_bg_fill = CARD;
    v.widgets.hovered.bg_fill = PANEL_HIGH;
    v.widgets.hovered.weak_bg_fill = PANEL_HIGH;
    v.widgets.active.bg_fill = SELECTION;
    v.widgets.active.weak_bg_fill = SELECTION;
    let radius = CornerRadius::same(3);
    v.widgets.inactive.corner_radius = radius;
    v.widgets.hovered.corner_radius = radius;
    v.widgets.active.corner_radius = radius;
    // every window and popup rounded like the dialogs
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, v);
    ctx.all_styles_mut(|s| {
        // texts are not selectable, so a click on a table row's text picks the row
        s.interaction.selectable_labels = false;
        s.animation_time = 0.12;
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(8.0, 4.0);
        s.text_styles.insert(egui::TextStyle::Body, regular(12.0));
        s.text_styles.insert(egui::TextStyle::Button, regular(12.0));
        s.text_styles.insert(egui::TextStyle::Heading, bold(16.0));
        s.text_styles.insert(egui::TextStyle::Small, regular(10.0));
    });
}

/// A club color from the save (`#rrggbb`), or the accent.
pub fn club_color(hex: &str) -> Color32 {
    let h = hex.trim_start_matches('#');
    if h.len() == 6
        && let Ok(v) = u32::from_str_radix(h, 16)
    {
        return Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    }
    ACCENT
}

/// A panel of the game: navy with the palette's border.
pub fn card() -> egui::Frame {
    egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER)).inner_margin(10)
}

/// A card header: the game's black strip with bold text (`a.ay` statistics cards).
pub fn header_strip(ui: &mut egui::Ui, text: &str, columns: &[&str]) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, Color32::BLACK);
    ui.painter().text(rect.left_center() + egui::vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, text, bold(12.0), TEXT);
    let mut x = rect.right() - 6.0;
    for c in columns.iter().rev() {
        ui.painter().text(egui::pos2(x - 12.0, rect.center().y), egui::Align2::CENTER_CENTER, *c, bold(12.0), TEXT);
        x -= 36.0;
    }
}

/// Energy colors by how much is left (`Visual.barraNaTabela`).
pub fn energy_color(value: i32) -> Color32 {
    if value >= 80 {
        ENERGY_HIGH
    } else if value >= 50 {
        ENERGY_MID
    } else {
        ENERGY_LOW
    }
}

/// `Ref.colunaRating`
pub fn rating_color(value: i32) -> Color32 {
    if value >= 75 {
        RATING_HIGH
    } else if value >= 65 {
        TEXT
    } else if value >= 55 {
        RATING_MID
    } else {
        RATING_LOW
    }
}

/// The colors of the positions on the pitch and in the lists: goalkeeper, defenders (backs and
/// centre backs), midfielders, forwards.
pub const POSITION_GOALKEEPER: Color32 = Color32::from_rgb(0xd9, 0x9a, 0x1e);
pub const POSITION_DEFENDER: Color32 = Color32::from_rgb(0x2f, 0x6f, 0xd6);
pub const POSITION_MIDFIELDER: Color32 = Color32::from_rgb(0x1f, 0x9d, 0x63);
pub const POSITION_FORWARD: Color32 = Color32::from_rgb(0xd0, 0x45, 0x4a);
/// The pitch drawn by the app: two greens for the stripes and the lines.
pub const PITCH_DARK: Color32 = Color32::from_rgb(0x1d, 0x5c, 0x37);
pub const PITCH_LIGHT: Color32 = Color32::from_rgb(0x22, 0x68, 0x3e);

/// A position's color (0 goalkeeper, 1 back, 2 centre back, 3 midfielder, 4 forward).
pub fn position_color(position: i32) -> Color32 {
    match position {
        0 => POSITION_GOALKEEPER,
        1 | 2 => POSITION_DEFENDER,
        3 => POSITION_MIDFIELDER,
        _ => POSITION_FORWARD,
    }
}
