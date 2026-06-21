//! 颜色常量 + 主题设置 + 中文字体加载

use egui::{Color32, Stroke, Vec2};

// ── Catppuccin Mocha → egui Color32 ────────────────────────

pub const C_BG: Color32       = Color32::from_rgb(30, 30, 46);
pub const C_SURFACE0: Color32 = Color32::from_rgb(62, 64, 85);
pub const C_SURFACE1: Color32 = Color32::from_rgb(83, 86, 110);
pub const C_SURFACE2: Color32 = Color32::from_rgb(105, 108, 135);
pub const C_TEXT: Color32     = Color32::from_rgb(209, 206, 210);
pub const C_SUBTEXT: Color32  = Color32::from_rgb(179, 176, 189);
pub const C_GREEN: Color32    = Color32::from_rgb(166, 227, 161);
pub const C_BLUE: Color32     = Color32::from_rgb(137, 180, 250);
pub const C_PEACH: Color32    = Color32::from_rgb(250, 179, 135);
pub const C_GREEN2: Color32   = Color32::from_rgb(148, 226, 178);
pub const C_MAUVE: Color32    = Color32::from_rgb(203, 166, 247);
pub const C_TEAL: Color32     = Color32::from_rgb(148, 190, 212);
pub const C_RED: Color32      = Color32::from_rgb(243, 139, 168);
pub const C_YELLOW: Color32   = Color32::from_rgb(249, 226, 175);
pub const C_PINK: Color32     = Color32::from_rgb(245, 194, 231);
pub const C_WHITE_SOFT: Color32 = Color32::from_rgb(238, 240, 244);
pub const C_LAVENDER: Color32  = Color32::from_rgb(180, 190, 254);
pub const C_ORANGE: Color32   = Color32::from_rgb(250, 160, 80);
pub const C_CYAN: Color32     = Color32::from_rgb(137, 220, 235);
pub const C_HOVER: Color32 = Color32::from_rgb(75, 77, 100);

// ── 字体 ──────────────────────────────────────────────────

fn load_chinese_font() -> Option<Vec<u8>> {
    let font_paths = [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ];
    for path in &font_paths {
        if let Ok(data) = std::fs::read(path) {
            return Some(data);
        }
    }
    None
}

pub fn setup_chinese_font(ctx: &egui::Context) {
    if let Some(font_data) = load_chinese_font() {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "CJK".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(font_data)),
        );
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families
                .entry(family)
                .or_default()
                .insert(0, "CJK".to_owned());
        }
        ctx.set_fonts(fonts);
    }
}

// ── 主题 ──────────────────────────────────────────────────

pub fn setup_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.panel_fill = C_BG;
    style.visuals.window_fill = C_BG;
    style.visuals.extreme_bg_color = Color32::from_rgb(55, 57, 72);
    style.visuals.faint_bg_color = Color32::from_rgb(35, 37, 50);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(95, 98, 125);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(70, 72, 92);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(65, 67, 88));
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(115, 119, 148);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(78, 80, 102));
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(130, 134, 165);
    style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(60, 62, 80);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, C_TEXT);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, C_TEXT);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, C_WHITE_SOFT);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, C_WHITE_SOFT);
    style.visuals.selection.bg_fill = C_BLUE.gamma_multiply(0.5);
    style.visuals.override_text_color = Some(C_TEXT);
    style.spacing.item_spacing = Vec2::new(8.0, 4.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    ctx.set_style(style);
}
