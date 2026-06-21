#![windows_subsystem = "windows"]

mod app;
mod config;
mod constants;
mod input;
mod lang;
mod state;
mod tray;

use app::BeerClickerApp;
use constants::{WND_H, WND_W};
use lang::{t, Lang};

use std::ptr::null_mut;
use winapi::um::winuser::{MessageBoxW, MB_OK, MB_ICONERROR};

fn main() {
    let icon_data = load_icon();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([WND_W as f32, WND_H as f32])
        .with_title("酒要点点");
    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        "beer_mouse_clicker",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(BeerClickerApp::new(cc)))
        }),
    ) {
        let lang = Lang::detect_system();
        let msg = format!("{}: {}", t(lang, "启动失败", "Startup failed"), e);
        let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
        let title_str = format!("{}\0", t(lang, "错误", "Error"));
        let title: Vec<u16> = title_str.encode_utf16().collect();
        unsafe {
            MessageBoxW(null_mut(), wide.as_ptr(), title.as_ptr(),
                MB_OK | MB_ICONERROR);
        }
    }
}

fn load_icon() -> Option<std::sync::Arc<egui::IconData>> {
    let ico = include_bytes!("../icon.ico");
    let img = image::load_from_memory(ico).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some(std::sync::Arc::new(egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    }))
}
