//! 添加任务弹窗：AddState 枚举 + draw_type_card + draw_add_popup

use crate::app::theme::*;
use crate::app::util::*;
use crate::constants::*;
use crate::lang::{t, tf};
use winapi::um::winuser::GetAsyncKeyState;
use egui::{Align2, CornerRadius, FontId, Vec2};
use image::ImageEncoder;

#[derive(Clone)]
pub enum AddState {
    None,
    PickType,
    ConfigDelay { ms_str: String },
    ConfigClick { button: i32, hold_ms: u64, hold_str: String, action_type: TaskActionType, jitter_str: String },
    ConfigKey { vk: i32, hold_ms: u64, hold_str: String, recording: bool, action_type: TaskActionType },
    ConfigWheel { up: bool },
    ConfigMove { preset: usize, secs_str: String, custom_x: String, custom_y: String, window_title: String, is_relative: bool },
    ConfigImageMatch { path: String, confidence_str: String, fail_msg: String, move_mouse: bool, ignore_failure: bool, window_title: String, thumbnail: Option<Vec<u8>>, full_image: Option<Vec<u8>>, show_preview: bool, preview_w: u32, preview_h: u32 },
    ConfigNotify { msg: String },
    ConfigRandomDelay { min_str: String, max_str: String },
    ConfigComboKey { preset: usize },
    ConfigWaitUntil { hour_str: String, minute_str: String },
    ConfigCopyText { msg: String },
    ConfigComment { msg: String, count_str: String },
    ConfigWaitKey { vk: i32, recording: bool, terminate: bool, popup: bool },
    ConfigWaitInput { msg: String, copy_input: bool, ignore_fail: bool, use_regex: bool },
    ConfigOpenProgram { msg: String },
}

use super::BeerClickerApp;

impl BeerClickerApp {
    /// 类型选择卡片
    pub fn draw_type_card(ui: &mut egui::Ui, label: &str, desc: &str, color: egui::Color32, w: f32, h: f32) {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), egui::Sense::click());
        let hovered = resp.hovered();
        let bg = if hovered { C_HOVER } else { C_SURFACE0 };
        ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
        let border_color = if hovered { brighten_color(C_SUBTEXT, 80) } else { C_SURFACE2 };
        ui.painter().rect_stroke(rect, CornerRadius::same(6), egui::Stroke::new(1.0, border_color), egui::StrokeKind::Inside);
        let bar = egui::Rect::from_min_size(rect.left_top(), egui::vec2(4.0, h));
        ui.painter().rect_filled(bar, CornerRadius::same(2), if hovered { brighten_color(color, 30) } else { color });
        let text_x = rect.left() + 14.0;
        let text_y = rect.top() + 8.0;
        ui.painter().text(
            egui::pos2(text_x, text_y),
            Align2::LEFT_TOP,
            label,
            FontId::proportional(14.0),
            if hovered { C_WHITE_SOFT } else { C_TEXT },
        );
        ui.painter().text(
            egui::pos2(text_x, text_y + 18.0),
            Align2::LEFT_TOP,
            desc,
            FontId::proportional(11.0),
            if hovered { brighten_color(C_SUBTEXT, 40) } else { C_SUBTEXT },
        );
        if resp.clicked() {
            ui.ctx().data_mut(|d| d.insert_temp::<String>(egui::Id::new("type_pick"), label.to_string()));
        }
    }

    pub fn draw_add_popup(&mut self, ctx: &egui::Context) {
        if matches!(self.add_state, AddState::None) { return; }

        let mut close = false;
        let mut done = false;
        let mut back = false;

        egui::Window::new(t(self.lang, "添加任务", "Add Task"))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                if !matches!(self.add_state, AddState::PickType) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        ui.add_space(ui.available_width() - 48.0);
                        let btn = egui::Button::new(egui::RichText::new(t(self.lang, "← 返回", "← Back")).size(12.0).color(C_SUBTEXT))
                            .min_size(Vec2::splat(0.0))
                            .small()
                            .frame(false);
                        if ui.add(btn).clicked() { back = true; }
                    });
                    ui.add_space(2.0);
                }
                match &mut self.add_state {
                    AddState::PickType => {
                        ui.add_space(4.0);
                        let types: [(&str, &str, egui::Color32); 22] = [
                            (t(self.lang, "点击", "Click"), t(self.lang, "单击鼠标左/右键", "Single left/right click"), C_BLUE),
                            (t(self.lang, "按下", "Hold"), t(self.lang, "按下鼠标不松开", "Hold mouse button down"), C_YELLOW),
                            (t(self.lang, "松开", "Release"), t(self.lang, "松开按下的鼠标键", "Release held mouse button"), C_RED),
                            (t(self.lang, "延迟", "Delay"), t(self.lang, "等待一段时间", "Wait for a duration"), C_PEACH),
                            (t(self.lang, "随机延迟", "Random Delay"), t(self.lang, "随机等待一段时间", "Wait a random duration"), C_ORANGE),
                            (t(self.lang, "整点延迟", "Wait Until"), t(self.lang, "等待到指定时间", "Wait until specific time"), C_ORANGE),
                            (t(self.lang, "按键", "Key Press"), t(self.lang, "按下并松开键盘按键", "Press and release a key"), C_GREEN2),
                            (t(self.lang, "按下按键", "Key Down"), t(self.lang, "按下键盘不松开", "Hold a key down"), C_GREEN),
                            (t(self.lang, "松开按键", "Key Up"), t(self.lang, "松开按下的键盘键", "Release a held key"), C_GREEN2),
                            (t(self.lang, "组合按键", "Combo Key"), t(self.lang, "同时按下多个键（如Win+D）", "Press multiple keys (e.g. Win+D)"), C_CYAN),
                            (t(self.lang, "滚轮", "Wheel"), t(self.lang, "鼠标滚轮滚动", "Mouse wheel scroll"), C_MAUVE),
                            (t(self.lang, "移动", "Move"), t(self.lang, "移动鼠标到目标位置", "Move mouse to target"), C_TEAL),
                            (t(self.lang, "识图", "Image Match"), t(self.lang, "屏幕找图并移动鼠标", "Find image on screen and move"), C_PINK),
                            (t(self.lang, "通知", "Notify"), t(self.lang, "弹出消息弹窗", "Show a message popup"), C_WHITE_SOFT),
                            (t(self.lang, "失去焦点", "Lost Focus"), t(self.lang, "最小化窗口释放焦点", "Minimize window to release focus"), C_LAVENDER),
                            (t(self.lang, "复制文本", "Copy Text"), t(self.lang, "复制自定义文本到剪贴板", "Copy custom text to clipboard"), C_CYAN),
                            (t(self.lang, "备注", "Comment"), t(self.lang, "仅用于备注说明，无任何操作", "For annotations only, no action"), C_SUBTEXT),
                            (t(self.lang, "等待按键", "Wait Key"), t(self.lang, "暂停等待指定按键后继续", "Pause until a key is pressed"), C_LAVENDER),
                            (t(self.lang, "等待输入", "Wait Input"), t(self.lang, "弹窗输入文本，匹配则继续", "Input dialog, match to continue"), C_LAVENDER),
                            (t(self.lang, "显示程序窗口", "Show Window"), t(self.lang, "将程序窗口显示出来", "Show the app window"), C_GREEN2),
                            (t(self.lang, "隐藏程序窗口", "Hide Window"), t(self.lang, "将程序窗口隐藏起来", "Hide the app window"), C_ORANGE),
                            (t(self.lang, "打开程序", "Open Program"), t(self.lang, "运行指定的外部程序", "Run an external program"), C_TEAL),
                        ];
                        let cell_w = 180.0;
                        let cell_h = 50.0;
                        let row_count = types.len().div_ceil(2);
                        let scroll_h = (row_count as f32 * cell_h + row_count as f32 * 4.0).min(360.0);
                        egui::ScrollArea::vertical().max_height(scroll_h).show(ui, |ui| {
                            for (col, _) in types.iter().enumerate() {
                                if col % 2 == 0 {
                                    ui.horizontal(|ui| {
                                        ui.set_height(cell_h);
                                        ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
                                        for (label2, desc2, color2) in types.iter().skip(col).take(2) {
                                            Self::draw_type_card(ui, label2, desc2, *color2, cell_w, cell_h);
                                        }
                                    });
                                }
                            }
                        });
                        if let Some(pick) = ui.ctx().data_mut(|d| d.remove_temp::<String>(egui::Id::new("type_pick"))) {
                            let s = pick.as_str();
                            if s == t(self.lang, "点击", "Click") { self.add_state = AddState::ConfigClick { button: 0, hold_ms: 0, hold_str: "0".into(), action_type: TaskActionType::MouseClick, jitter_str: "0".into() }; }
                            else if s == t(self.lang, "按下", "Hold") { self.add_state = AddState::ConfigClick { button: 0, hold_ms: 0, hold_str: "0".into(), action_type: TaskActionType::MouseDown, jitter_str: "0".into() }; }
                            else if s == t(self.lang, "松开", "Release") { self.add_state = AddState::ConfigClick { button: 0, hold_ms: 0, hold_str: "0".into(), action_type: TaskActionType::MouseUp, jitter_str: "0".into() }; }
                            else if s == t(self.lang, "延迟", "Delay") { self.add_state = AddState::ConfigDelay { ms_str: "100".into() }; }
                            else if s == t(self.lang, "随机延迟", "Random Delay") { self.add_state = AddState::ConfigRandomDelay { min_str: "100".into(), max_str: "1000".into() }; }
                            else if s == t(self.lang, "整点延迟", "Wait Until") { self.add_state = AddState::ConfigWaitUntil { hour_str: "12".into(), minute_str: "00".into() }; }
                            else if s == t(self.lang, "组合按键", "Combo Key") { self.add_state = AddState::ConfigComboKey { preset: 0 }; }
                            else if s == t(self.lang, "按键", "Key Press") { self.add_state = AddState::ConfigKey { vk: 0x20, hold_ms: 0, hold_str: "0".into(), recording: false, action_type: TaskActionType::KeyPress }; }
                            else if s == t(self.lang, "按下按键", "Key Down") { self.add_state = AddState::ConfigKey { vk: 0x20, hold_ms: 0, hold_str: "0".into(), recording: false, action_type: TaskActionType::KeyDown }; }
                            else if s == t(self.lang, "松开按键", "Key Up") { self.add_state = AddState::ConfigKey { vk: 0x20, hold_ms: 0, hold_str: "0".into(), recording: false, action_type: TaskActionType::KeyUp }; }
                            else if s == t(self.lang, "滚轮", "Wheel") { self.add_state = AddState::ConfigWheel { up: true }; }
                            else if s == t(self.lang, "移动", "Move") { self.add_state = AddState::ConfigMove { preset: 0, secs_str: "0.0".into(), custom_x: String::new(), custom_y: String::new(), window_title: String::new(), is_relative: false }; }
                            else if s == t(self.lang, "识图", "Image Match") { self.add_state = AddState::ConfigImageMatch { path: String::new(), confidence_str: "85".into(), fail_msg: String::new(), move_mouse: true, ignore_failure: false, window_title: String::new(), thumbnail: None, full_image: None, show_preview: false, preview_w: 0, preview_h: 0 }; }
                            else if s == t(self.lang, "通知", "Notify") { self.add_state = AddState::ConfigNotify { msg: String::new() }; }
                            else if s == t(self.lang, "复制文本", "Copy Text") { self.add_state = AddState::ConfigCopyText { msg: String::new() }; }
                            else if s == t(self.lang, "备注", "Comment") { self.add_state = AddState::ConfigComment { msg: String::new(), count_str: String::new() }; }
                            else if s == t(self.lang, "等待按键", "Wait Key") { self.add_state = AddState::ConfigWaitKey { vk: 0x20, recording: false, terminate: false, popup: false }; }
                            else if s == t(self.lang, "等待输入", "Wait Input") { self.add_state = AddState::ConfigWaitInput { msg: String::new(), copy_input: false, ignore_fail: false, use_regex: false }; }
                            else if s == t(self.lang, "失去焦点", "Lost Focus") {
                                self.state.tasks.push(TaskStep {
                                    action: TaskActionType::LostFocus,
                                    param: 0,
                                    extra: 0,
                                });
                                crate::config::save_tasks(&self.state.tasks);
                                close = true;
                            }
                            else if s == t(self.lang, "显示程序窗口", "Show Window") {
                                self.state.tasks.push(TaskStep {
                                    action: TaskActionType::ShowWindow,
                                    param: 0,
                                    extra: 0,
                                });
                                crate::config::save_tasks(&self.state.tasks);
                                close = true;
                            }
                            else if s == t(self.lang, "隐藏程序窗口", "Hide Window") {
                                self.state.tasks.push(TaskStep {
                                    action: TaskActionType::HideWindow,
                                    param: 0,
                                    extra: 0,
                                });
                                crate::config::save_tasks(&self.state.tasks);
                                close = true;
                            }
                            else if s == t(self.lang, "打开程序", "Open Program") { self.add_state = AddState::ConfigOpenProgram { msg: String::new() }; }
                        }
                        ui.add_space(8.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                        });
                    }
                    AddState::ConfigComboKey { preset } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "组合按键", "Combo Key")).size(15.0).color(C_CYAN));
                            ui.label(egui::RichText::new(t(self.lang, " — 自选常用快捷键", " — Choose a common shortcut")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        const COLS: usize = 2;
                        egui::Grid::new("add_combo_grid").show(ui, |ui| {
                            for (i, (name, keys, _, _)) in crate::constants::COMBO_PRESETS.iter().enumerate() {
                                if selectable_btn(ui, *preset == i, egui::RichText::new(format!("{}|{}", name, keys)).size(11.5)).clicked() { *preset = i; }
                                if (i + 1) % COLS == 0 { ui.end_row(); }
                            }
                        });
                        if *preset < crate::constants::COMBO_PRESETS.len() {
                            let (_, _, desc, vks) = crate::constants::COMBO_PRESETS[*preset];
                            ui.add_space(4.0);
                            let tmpl = tf(self.lang, "用途: {}", "Usage: {}");
                            ui.label(egui::RichText::new(tmpl.replace("{}", desc)).size(12.0).color(C_SUBTEXT));
                            ui.add_space(8.0);
                            let vks_save = vks.to_vec();
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                                if ui.button(t(self.lang, "添加", "Add")).clicked() { done = true; }
                            });
                            if done {
                                let param = pack_combo(&vks_save);
                                self.state.tasks.push(TaskStep { action: TaskActionType::ComboKey, param, extra: vks_save.len() as u32 });
                                crate::config::save_tasks(&self.state.tasks);
                            }
                        } else {
                            ui.add_space(8.0);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            });
                        }
                    }
                    AddState::ConfigWaitUntil { hour_str, minute_str } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "整点延迟", "Wait Until")).size(15.0).color(C_ORANGE));
                            ui.label(egui::RichText::new(t(self.lang, " — 等待到指定时间再继续", " — Wait until specific time")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "等待到:", "Wait until:"));
                            ui.add(egui::TextEdit::singleline(hour_str).desired_width(40.0).hint_text(t(self.lang, "时", "h")));
                            ui.label(t(self.lang, "时", "h"));
                            ui.add(egui::TextEdit::singleline(minute_str).desired_width(40.0).hint_text(t(self.lang, "分", "m")));
                            ui.label(t(self.lang, "分", "m"));
                        });
                        ui.add_space(8.0);
                        let h = hour_str.parse::<u32>().unwrap_or(0).min(23);
                        let m = minute_str.parse::<u32>().unwrap_or(0).min(59);
                        let param = (h * 60 + m) as u64;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() { done = true; }
                        });
                        if done {
                            self.state.tasks.push(TaskStep { action: TaskActionType::WaitUntil, param, extra: 0 });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigDelay { ms_str } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "延迟", "Delay")).size(15.0).color(C_PEACH));
                            ui.label(egui::RichText::new(t(self.lang, "  — 等待毫秒数", "  — Wait in milliseconds")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "时长:", "Duration:"));
                            ui.add(egui::TextEdit::singleline(ms_str).desired_width(80.0));
                            ui.label("ms");
                        });
                        ui.add_space(6.0);
                        let ms_parsed = ms_str.parse::<u64>().unwrap_or(100).clamp(1, 60000);
                        let mut ms_slider = ms_parsed as f64;
                        if ui.add(egui::Slider::new(&mut ms_slider, 1.0..=60000.0).text("")).changed() {
                            *ms_str = format!("{}", ms_slider as u64);
                        }
                        ui.add_space(8.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() { done = true; }
                        });
                        if done {
                            let ms_val = ms_str.parse::<u64>().unwrap_or(100).clamp(1, 60000);
                            self.state.tasks.push(TaskStep { action: TaskActionType::Delay, param: ms_val, extra: 0 });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigRandomDelay { min_str, max_str } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "随机延迟", "Random Delay")).size(15.0).color(C_ORANGE));
                            ui.label(egui::RichText::new(t(self.lang, " — 随机等待毫秒数", " — Wait random milliseconds")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "最小:", "Min:"));
                            ui.add(egui::TextEdit::singleline(min_str).desired_width(60.0));
                            ui.label(t(self.lang, "ms  最大:", "ms  Max:"));
                            ui.add(egui::TextEdit::singleline(max_str).desired_width(60.0));
                            ui.label("ms");
                        });
                        ui.add_space(8.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() { done = true; }
                        });
                        if done {
                            let min_val = min_str.parse::<u64>().unwrap_or(100).clamp(1, 60000);
                            let max_val = max_str.parse::<u64>().unwrap_or(1000).clamp(1, 60000);
                            let (lo, hi) = if min_val <= max_val { (min_val, max_val) } else { (max_val, min_val) };
                            let param = lo | (hi << 32);
                            self.state.tasks.push(TaskStep { action: TaskActionType::RandomDelay, param, extra: 0 });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigClick { button, hold_ms, hold_str, action_type, jitter_str } => {
                        let (title, title_color) = match *action_type {
                            TaskActionType::MouseClick => (t(self.lang, "点击", "Click"), C_BLUE),
                            TaskActionType::MouseDown => (t(self.lang, "按下", "Hold"), C_YELLOW),
                            TaskActionType::MouseUp => (t(self.lang, "松开", "Release"), C_RED),
                            _ => unreachable!(),
                        };
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(title).size(15.0).color(title_color));
                            let sub = match *action_type {
                                TaskActionType::MouseClick => t(self.lang, " — 单击或长按鼠标", " — Single click or hold"),
                                TaskActionType::MouseDown => t(self.lang, " — 仅按下不松开", " — Press without releasing"),
                                TaskActionType::MouseUp => t(self.lang, " — 仅松开按键", " — Release only"),
                                _ => "",
                            };
                            ui.label(egui::RichText::new(sub).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "鼠标键:", "Button:"));
                            for (v, label) in [(0, t(self.lang, "左键", "Left")), (1, t(self.lang, "右键", "Right")), (2, t(self.lang, "中键", "Middle"))] {
                                if selectable_btn(ui, *button == v, label).clicked() { *button = v; }
                            }
                        });
                        if *action_type == TaskActionType::MouseClick {
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(t(self.lang, "长按:", "Hold:"));
                                ui.add(egui::TextEdit::singleline(hold_str).desired_width(80.0));
                                ui.label(t(self.lang, "ms (0=单击)", "ms (0=click)"));
                            });
                            if let Ok(v) = hold_str.parse::<u64>() { *hold_ms = v.min(60000); }
                        }
                        ui.add_space(6.0);
                        let mut jitter_val = 0u32;
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "抖动范围:", "Jitter:"));
                            ui.add(egui::TextEdit::singleline(jitter_str).desired_width(60.0));
                            ui.label(t(self.lang, "px (0=不抖动)", "px (0=off)"));
                        });
                        if let Ok(v) = jitter_str.parse::<u32>() { jitter_val = v.min(50); }
                        ui.add_space(8.0);
                        let btn = *button; let hold = *hold_ms; let act = *action_type;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() { done = true; }
                        });
                        if done {
                            let param = if act == TaskActionType::MouseClick {
                                (btn as u64 & 0xFF) | (hold << 8)
                            } else {
                                btn as u64
                            };
                            self.state.tasks.push(TaskStep { action: act, param, extra: jitter_val });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigKey { vk, hold_ms, hold_str, recording, action_type } => {
                        let (title, title_color, subtitle) = match *action_type {
                            TaskActionType::KeyPress => (t(self.lang, "按键", "Key Press"), C_GREEN2, t(self.lang, " — 按下并松开键盘按键", " — Press and release a key")),
                            TaskActionType::KeyDown => (t(self.lang, "按下按键", "Key Down"), C_GREEN, t(self.lang, " — 按下键盘不松开", " — Hold key down")),
                            TaskActionType::KeyUp => (t(self.lang, "松开按键", "Key Up"), C_GREEN2, t(self.lang, " — 松开按下的键盘键", " — Release held key")),
                            _ => unreachable!(),
                        };
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(title).size(15.0).color(title_color));
                            ui.label(egui::RichText::new(subtitle).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        if *recording {
                            ui.label(egui::RichText::new(t(self.lang, "请按下一个键…", "Press a key…")).color(C_GREEN).size(14.0));
                            fn is_down(vk: i32) -> bool {
                                unsafe { (GetAsyncKeyState(vk) & 0x8000u16 as i16) != 0 }
                            }
                            'key_rec: for vk_code in 0..256i32 {
                                if is_mouse_vk(vk_code) { continue; }
                                if is_down(vk_code) {
                                    *vk = vk_code;
                                    *recording = false;
                                    break 'key_rec;
                                }
                            }
                        } else {
                            let tmpl = tf(self.lang, "当前按键: {key}", "Current: {key}");
                            ui.horizontal(|ui| {
                                ui.label(tmpl.replace("{key}", &vk_name(*vk, self.lang)));
                                if ui.small_button(t(self.lang, "重新录制", "Re-record")).clicked() { *recording = true; }
                            });
                        }
                        if *action_type == TaskActionType::KeyPress {
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(t(self.lang, "长按:", "Hold:"));
                                ui.add(egui::TextEdit::singleline(hold_str).desired_width(80.0));
                                ui.label(t(self.lang, "ms (0=按一下)", "ms (0=tap)"));
                            });
                            if let Ok(v) = hold_str.parse::<u64>() { *hold_ms = v.min(60000); }
                        }
                        ui.add_space(8.0);
                        let key_vk = *vk; let hold = *hold_ms; let act = *action_type;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() { done = true; }
                        });
                        if done {
                            let param = if act == TaskActionType::KeyPress {
                                (key_vk as u64 & 0xFFFF) | (hold << 16)
                            } else {
                                key_vk as u64 & 0xFFFF
                            };
                            self.state.tasks.push(TaskStep { action: act, param, extra: 0 });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigWheel { up } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "滚轮", "Wheel")).size(15.0).color(C_MAUVE));
                            ui.label(egui::RichText::new(t(self.lang, "  — 鼠标滚轮滚动", "  — Mouse wheel scroll")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "方向:", "Direction:"));
                            for (v, label) in [(true, t(self.lang, "上滚", "Up")), (false, t(self.lang, "下滚", "Down"))] {
                                if selectable_btn(ui, *up == v, label).clicked() { *up = v; }
                            }
                        });
                        ui.add_space(8.0);
                        let wheel_up = *up;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() { done = true; }
                        });
                        if done {
                            let param = if wheel_up { 1 } else { 0 };
                            self.state.tasks.push(TaskStep { action: TaskActionType::MouseWheel, param, extra: 0 });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigMove { preset, secs_str, custom_x, custom_y, window_title, is_relative } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "移动", "Move")).size(15.0).color(C_TEAL));
                            ui.label(egui::RichText::new(t(self.lang, "  — 移动鼠标到目标位置", "  — Move mouse to target")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.label(t(self.lang, "目标位置:", "Target:"));
                        ui.add_space(4.0);
                        let custom_idx = MOUSE_MOVE_PRESETS.len();
                        let dist_idx = MOUSE_MOVE_PRESETS.len() + 1;
                        let window_idx = MOUSE_MOVE_PRESETS.len() + 2;
                        egui::Grid::new("move_presets").spacing([10.0, 4.0]).show(ui, |ui| {
                            let mut count = 0;
                            for (i, (_, _, label)) in MOUSE_MOVE_PRESETS.iter().enumerate() {
                                if selectable_btn(ui, *preset == i, *label).clicked() { *preset = i; *is_relative = false; }
                                count += 1;
                                if count % 2 == 0 { ui.end_row(); }
                            }
                            if selectable_btn(ui, *preset == custom_idx, t(self.lang, "指定坐标", "Coordinates")).clicked() { *preset = custom_idx; *is_relative = false; }
                            count += 1;
                            if count % 2 == 0 { ui.end_row(); }
                            if selectable_btn(ui, *preset == dist_idx, t(self.lang, "指定距离", "Relative")).clicked() { *preset = dist_idx; *is_relative = true; }
                            count += 1;
                            if count % 2 == 0 { ui.end_row(); }
                            if selectable_btn(ui, *preset == window_idx, t(self.lang, "窗口居中", "Center")).clicked() { *preset = window_idx; *is_relative = false; }
                            count += 1;
                            if count % 2 == 0 { ui.end_row(); }
                            if count % 2 != 0 { ui.end_row(); }
                        });
                        if *preset == custom_idx || *preset == dist_idx {
                            ui.add_space(4.0);
                            let (hint_x, hint_y) = if *is_relative { ("ΔX", "ΔY") } else { ("X", "Y") };
                            ui.horizontal(|ui| {
                                ui.label(format!("{}:", hint_x));
                                ui.add(egui::TextEdit::singleline(custom_x).desired_width(70.0).hint_text("0"));
                                ui.label(format!("{}:", hint_y));
                                ui.add(egui::TextEdit::singleline(custom_y).desired_width(70.0).hint_text("0"));
                            });
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(
                                if *is_relative { t(self.lang, "相对当前光标位置偏移", "Offset from cursor") } else { t(self.lang, "屏幕绝对坐标", "Screen absolute") }
                            ).size(11.0).color(C_SUBTEXT));
                        } else if *preset == window_idx {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(t(self.lang, "窗口标题:", "Window title:"));
                                ui.add(egui::TextEdit::singleline(window_title).desired_width(200.0).hint_text(t(self.lang, "例如：记事本", "e.g. Notepad")));
                            });
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "时间:", "Time:"));
                            let mut secs = secs_str.parse::<f64>().unwrap_or(1.0);
                            if ui.add(egui::DragValue::new(&mut secs).range(0.0..=60.0).speed(0.1)).changed() {
                                *secs_str = format!("{:.1}", secs);
                            }
                            ui.label(t(self.lang, "秒", "s"));
                        });
                        ui.add_space(8.0);
                        let p = *preset;
                        let duration = (secs_str.parse::<f64>().unwrap_or(1.0) * 1000.0) as u32;
                        let x_str = custom_x.clone();
                        let y_str = custom_y.clone();
                        let win_title = window_title.clone();
                        let rel = *is_relative;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() { done = true; }
                        });
                        if done {
                            let (x, y) = if p < MOUSE_MOVE_PRESETS.len() {
                                let (px, py, _) = MOUSE_MOVE_PRESETS[p];
                                (px, py)
                            } else if p == window_idx {
                                if win_title.is_empty() { close = true; }
                                let msg_id = if win_title.is_empty() { 0 } else { crate::config::save_msg(&win_title) };
                                (-2, msg_id as i32)
                            } else {
                                let cx = x_str.parse::<i32>().unwrap_or(0);
                                let cy = y_str.parse::<i32>().unwrap_or(0);
                                (cx, cy)
                            };
                            let extra = if rel { duration | RELATIVE_FLAG } else { duration };
                            self.state.tasks.push(TaskStep { action: TaskActionType::MouseMove, param: pack_move(x, y), extra });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigImageMatch { path, confidence_str, fail_msg, move_mouse, ignore_failure, window_title, thumbnail, full_image, show_preview, preview_w, preview_h } => {
                        if let Some(p) = self.picked_path.take() {
                            *path = p.clone();
                            *full_image = std::fs::read(&p).ok();
                            *preview_w = 0;
                            *preview_h = 0;
                            *thumbnail = full_image.as_ref().and_then(|data| {
                                image::load_from_memory(data).ok().and_then(|img| {
                                    let thumb = img.thumbnail(48, 48).to_rgba8();
                                    let mut buf = Vec::new();
                                    image::codecs::png::PngEncoder::new(&mut buf)
                                        .write_image(thumb.as_raw(), thumb.width(), thumb.height(), image::ExtendedColorType::Rgba8)
                                        .ok().map(|_| buf)
                                })
                            });
                        }
                        if *show_preview {
                            ui.horizontal(|ui| {
                                if ui.button(t(self.lang, "← 返回", "← Back")).clicked() {
                                    *show_preview = false;
                                }
                                if !path.is_empty() {
                                    ui.label(egui::RichText::new(&**path).size(11.0).color(C_SUBTEXT));
                                }
                            });
                            if let Some(ref data) = full_image {
                                if *preview_w == 0 {
                                    if let Ok(img) = image::load_from_memory(data) {
                                        *preview_w = img.width();
                                        *preview_h = img.height();
                                    }
                                }
                                if *preview_w > 0 {
                                    let uri = format!("bytes://full_add_{}", path);
                                    let display = egui::Image::from_bytes(uri, data.clone());
                                    ui.add(display.max_size(egui::Vec2::new(350.0, 300.0)).corner_radius(4));
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(format!("{}x{}", preview_w, preview_h)).size(11.0).color(C_SUBTEXT));
                                        if ui.button(t(self.lang, "💾 另存为", "💾 Save as")).clicked() {
                                            self.pending_save_data = Some(data.clone());
                                            self.pending_save_open = true;
                                        }
                                    });
                                }
                            }
                        } else {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "识图", "Image Match")).size(15.0).color(C_PINK));
                            ui.label(egui::RichText::new(t(self.lang, " — 屏幕找图并移动鼠标", " — Find image on screen")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if let Some(ref png_data) = thumbnail {
                                let uri = format!("bytes://thumbnail_add_{}", path);
                                let img = egui::Image::from_bytes(uri, png_data.clone());
                                if ui.add(egui::Button::image(img.max_size(egui::Vec2::splat(48.0)).corner_radius(3)).frame(false)).clicked() {
                                    *show_preview = true;
                                }
                                ui.add_space(8.0);
                            }
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(t(self.lang, "图片:", "Image:"));
                                    ui.add(egui::TextEdit::singleline(path).desired_width(160.0).hint_text(t(self.lang, "选择图片...", "Choose image...")));
                                    if ui.button(t(self.lang, "📂 浏览", "📂 Browse")).clicked() {
                                        self.pending_file_pick = Some(super::FilePickKind::Image);
                                    }
                                });
                                if !path.is_empty() {
                                    let tmpl = tf(self.lang, "已选择: {path}", "Selected: {path}");
                                    ui.label(egui::RichText::new(tmpl.replace("{path}", path)).size(11.0).color(C_SUBTEXT));
                                }
                            });
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "可信度(1-100):", "Confidence(1-100):"));
                            ui.add(egui::TextEdit::singleline(confidence_str).desired_width(40.0));
                            ui.label("%");
                        });
                        ui.add_space(6.0);
                        ui.checkbox(move_mouse, t(self.lang, "移动鼠标到匹配位置", "Move mouse to match"));
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "失败提示:", "Failure tip:"));
                            ui.add(egui::TextEdit::singleline(fail_msg).desired_width(200.0).hint_text(t(self.lang, "留空则不提示", "Leave empty for no tip")));
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "窗口标题:", "Window title:"));
                            ui.add(egui::TextEdit::singleline(window_title).desired_width(140.0).hint_text(t(self.lang, "留空=全屏搜索", "Empty=full screen")));
                        });
                        ui.add_space(4.0);
                        ui.checkbox(ignore_failure, t(self.lang, "无视失败继续执行", "Ignore failure"));
                        ui.add_space(8.0);
                        let img_path = path.clone();
                        let conf_threshold = confidence_str.parse::<u8>().unwrap_or(85).clamp(1, 100);
                        let fail_text = fail_msg.clone();
                        let do_move = *move_mouse;
                        let do_ignore = *ignore_failure;
                        let win_title = window_title.clone();
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() && !img_path.is_empty() { done = true; }
                        });
                        if done {
                            if let Ok(data) = std::fs::read(&img_path) {
                                    let image_id = crate::config::save_image(&data) as u32;
                                    let fail_id = if fail_text.is_empty() {
                                        0u32
                                    } else {
                                        crate::config::save_msg(&fail_text) as u32
                                    };
                                    let window_id = if win_title.is_empty() {
                                        0u32
                                    } else {
                                        crate::config::save_msg(&win_title) as u32
                                    };
                                    let extra = conf_threshold as u32
                                        | (if do_move { 0x100 } else { 0 })
                                        | (if do_ignore { 0x200 } else { 0 })
                                        | (window_id << 10);
                                    self.state.tasks.push(TaskStep {
                                        action: TaskActionType::ImageMatch,
                                        param: crate::constants::pack_image_match(image_id, fail_id),
                                        extra,
                                    });
                                    crate::config::save_tasks(&self.state.tasks);
                            }
                        }
                        }
                    }
                    AddState::ConfigNotify { msg } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "通知", "Notify")).size(15.0).color(C_WHITE_SOFT));
                            ui.label(egui::RichText::new(t(self.lang, "  — 弹出消息弹窗", "  — Show message popup")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "消息:", "Message:"));
                            ui.add(egui::TextEdit::singleline(msg).desired_width(250.0).hint_text(t(self.lang, "输入通知内容...", "Enter notification text...")));
                        });
                        ui.add_space(8.0);
                        let msg_text = msg.clone();
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() && !msg_text.is_empty() { done = true; }
                        });
                        if done {
                            let msg_id = crate::config::save_msg(&msg_text);
                            self.state.tasks.push(TaskStep {
                                action: TaskActionType::Notify,
                                param: msg_id,
                                extra: 0,
                            });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigCopyText { msg } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "复制文本", "Copy Text")).size(15.0).color(C_CYAN));
                            ui.label(egui::RichText::new(t(self.lang, "  — 复制自定义文本到剪贴板", "  — Copy to clipboard")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "文本:", "Text:"));
                            ui.add(egui::TextEdit::singleline(msg).desired_width(250.0).hint_text(t(self.lang, "输入要复制的文本...", "Enter text to copy...")));
                        });
                        ui.add_space(8.0);
                        let msg_text = msg.clone();
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() && !msg_text.is_empty() { done = true; }
                        });
                        if done {
                            let msg_id = crate::config::save_msg(&msg_text);
                            self.state.tasks.push(TaskStep {
                                action: TaskActionType::CopyText,
                                param: msg_id,
                                extra: 0,
                            });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigComment { msg, count_str } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "备注", "Comment")).size(15.0).color(C_SUBTEXT));
                            ui.label(egui::RichText::new(t(self.lang, "  — 仅用于备注说明，无任何操作", "  — For annotations only")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "内容:", "Content:"));
                            ui.add(egui::TextEdit::singleline(msg).desired_width(200.0).hint_text(t(self.lang, "输入备注内容（留空也行）...", "Enter comment (can be empty)...")));
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "范围:", "Range:"));
                            ui.add(egui::TextEdit::singleline(count_str).desired_width(80.0).hint_text("0"));
                            ui.label(egui::RichText::new(t(self.lang, "标注此备注覆盖后续几个任务，0 表示不标注", "How many following tasks this comment covers, 0=none")).size(11.0).color(C_SUBTEXT));
                        });
                        ui.add_space(8.0);
                        let msg_text = msg.clone();
                        let count_val = count_str.parse::<u32>().unwrap_or(0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() { done = true; }
                        });
                        if done {
                            let msg_id = if msg_text.is_empty() { 0 } else { crate::config::save_msg(&msg_text) };
                            self.state.tasks.push(TaskStep {
                                action: TaskActionType::Comment,
                                param: msg_id,
                                extra: count_val,
                            });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigWaitInput { msg, copy_input, ignore_fail, use_regex } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "等待输入", "Wait Input")).size(15.0).color(C_LAVENDER));
                            ui.label(egui::RichText::new(t(self.lang, " — 弹窗输入文本，匹配则继续", " — Input dialog, match to continue")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "匹配文本:", "Match text:"));
                            ui.add(egui::TextEdit::singleline(msg).desired_width(250.0).hint_text(t(self.lang, "输入期望的文本...", "Enter expected text...")));
                        });
                        ui.add_space(8.0);
                        ui.checkbox(copy_input, t(self.lang, "复制输入内容到剪贴板", "Copy input to clipboard"));
                        ui.add_space(4.0);
                        ui.checkbox(use_regex, t(self.lang, "使用正则匹配", "Use regex"));
                        ui.add_space(4.0);
                        ui.checkbox(ignore_fail, t(self.lang, "无视失败继续执行", "Ignore failure"));
                        ui.add_space(8.0);
                        let msg_text = msg.clone();
                        let copy_val = *copy_input;
                        let ignore_val = *ignore_fail;
                        let regex_val = *use_regex;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() && !msg_text.is_empty() { done = true; }
                        });
                        if done {
                            let msg_id = crate::config::save_msg(&msg_text);
                            let mut extra: u32 = 0;
                            if copy_val { extra |= WAITINPUT_COPY; }
                            if ignore_val { extra |= WAITINPUT_IGNORE_FAIL; }
                            if regex_val { extra |= WAITINPUT_REGEX; }
                            self.state.tasks.push(TaskStep {
                                action: TaskActionType::WaitInput,
                                param: msg_id,
                                extra,
                            });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigWaitKey { vk, recording, terminate, popup } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "等待按键", "Wait Key")).size(15.0).color(C_LAVENDER));
                            ui.label(egui::RichText::new(t(self.lang, " — 暂停等待指定按键后继续", " — Pause until key is pressed")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        // 按键录制
                        if *recording {
                            ui.label(egui::RichText::new(t(self.lang, "按下目标按键...", "Press target key...")).size(13.0).color(C_ORANGE));
                            for test_vk in 0x01..=0xFF {
                                let state = unsafe { GetAsyncKeyState(test_vk) } as u32;
                                if (state & 0x8001) != 0 && test_vk != 0x01 && test_vk != 0x02 && test_vk != 0x04 && test_vk != 0x05 && test_vk != 0x06 && test_vk != 0x07 {
                                    *vk = test_vk;
                                    *recording = false;
                                    break;
                                }
                            }
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(t(self.lang, "目标按键:", "Target key:"));
                                let key_label = vk_name(*vk, self.lang);
                                if ui.button(&key_label).clicked() {
                                    *recording = true;
                                }
                            });
                        }
                        ui.add_space(8.0);
                        // 复选框
                        ui.checkbox(terminate, t(self.lang, "按键错误终止任务", "Terminate on wrong key"));
                        ui.add_space(4.0);
                        ui.checkbox(popup, t(self.lang, "弹窗提示", "Show popup"));
                        ui.add_space(8.0);
                        // 按钮
                        let vk_val = *vk;
                        let terminate_val = *terminate;
                        let popup_val = *popup;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() { done = true; }
                        });
                        if done {
                            let mut extra: u32 = 0;
                            if terminate_val { extra |= WAITKEY_TERMINATE; }
                            if popup_val { extra |= WAITKEY_POPUP; }
                            self.state.tasks.push(TaskStep {
                                action: TaskActionType::WaitKey,
                                param: vk_val as u64,
                                extra,
                            });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    AddState::ConfigOpenProgram { msg } => {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(t(self.lang, "打开程序", "Open Program")).size(15.0).color(C_TEAL));
                            ui.label(egui::RichText::new(t(self.lang, " — 运行指定的外部程序", " — Run an external program")).size(13.0).color(C_SUBTEXT));
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "程序路径:", "Program path:"));
                            ui.add(egui::TextEdit::singleline(msg).desired_width(280.0).hint_text(t(self.lang, "如 C:\\Windows\\notepad.exe", "e.g. C:\\Windows\\notepad.exe")));
                        });
                        ui.add_space(4.0);
                        if ui.button(t(self.lang, "浏览...", "Browse...")).clicked() {
                            self.pending_file_pick = Some(super::FilePickKind::Exe);
                        }
                        if let Some(p) = self.picked_path.take() {
                            *msg = p;
                        }
                        ui.add_space(8.0);
                        let path = msg.clone();
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t(self.lang, "取消", "Cancel")).clicked() { close = true; }
                            if ui.button(t(self.lang, "添加", "Add")).clicked() && !path.is_empty() { done = true; }
                        });
                        if done {
                            let msg_id = crate::config::save_msg(&path);
                            self.state.tasks.push(TaskStep {
                                action: TaskActionType::OpenProgram,
                                param: msg_id,
                                extra: 0,
                            });
                            crate::config::save_tasks(&self.state.tasks);
                        }
                    }
                    _ => {}
                }
            });

        if done || close {
            self.add_state = AddState::None;
        } else if back {
            self.add_state = AddState::PickType;
        }
    }
}
