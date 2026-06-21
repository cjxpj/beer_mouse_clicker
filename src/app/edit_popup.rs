//! 编辑任务弹窗：EditState 枚举 + start_edit + draw_edit_popup

use crate::app::theme::*;
use crate::app::util::*;
use crate::constants::*;
use crate::lang::{t, tf};
use egui::Align2;
use image::ImageEncoder;

#[derive(Clone)]
pub enum EditState {
    None,
    Delay { ms_str: String },
    Click { button: i32, hold_ms: u64, hold_str: String, action_type: TaskActionType, jitter: u32, jitter_str: String },
    Key { vk: i32, hold_ms: u64, hold_str: String, recording: bool, action_type: TaskActionType },
    Wheel { up: bool },
    Move { preset: usize, secs_str: String, custom_x: String, custom_y: String, window_title: String, is_relative: bool },
    ImageMatch { image_id: u32, new_path: String, new_thumbnail: Option<Vec<u8>>, full_image: Option<Vec<u8>>, show_preview: bool, preview_w: u32, preview_h: u32, confidence_str: String, fail_msg: String, move_mouse: bool, ignore_failure: bool, window_title: String },
    Notify { msg: String },
    LostFocus,
    RandomDelay { min_str: String, max_str: String },
    ComboKey { preset: Option<usize> },
    WaitUntil { hour_str: String, minute_str: String },
    CopyText { msg: String },
    Comment { msg: String, count_str: String },
    WaitKey { vk: i32, recording: bool, terminate: bool, popup: bool },
    WaitInput { msg: String, copy_input: bool, ignore_fail: bool, use_regex: bool },
    ShowWindow,
    HideWindow,
    OpenProgram { msg: String },
}

use super::BeerClickerApp;

impl BeerClickerApp {
    pub fn start_edit(&mut self, idx: usize) {
        let step = &self.state.tasks[idx];
        self.edit_index = Some(idx);
        self.edit_state = match step.action {
            TaskActionType::Delay => EditState::Delay { ms_str: step.param.to_string() },
            TaskActionType::MouseClick => {
                let btn = (step.param & 0xFF) as i32;
                let hold = step.param >> 8;
                EditState::Click { button: btn, hold_ms: hold, hold_str: hold.to_string(), action_type: TaskActionType::MouseClick, jitter: step.extra, jitter_str: step.extra.to_string() }
            }
            TaskActionType::MouseDown => {
                let btn = step.param as i32;
                EditState::Click { button: btn, hold_ms: 0, hold_str: "0".into(), action_type: TaskActionType::MouseDown, jitter: step.extra, jitter_str: step.extra.to_string() }
            }
            TaskActionType::MouseUp => {
                let btn = step.param as i32;
                EditState::Click { button: btn, hold_ms: 0, hold_str: "0".into(), action_type: TaskActionType::MouseUp, jitter: step.extra, jitter_str: step.extra.to_string() }
            }
            TaskActionType::KeyPress => {
                let vk = (step.param & 0xFFFF) as i32;
                let hold = step.param >> 16;
                EditState::Key { vk, hold_ms: hold, hold_str: hold.to_string(), recording: false, action_type: TaskActionType::KeyPress }
            }
            TaskActionType::KeyDown => {
                let vk = step.param as i32;
                EditState::Key { vk, hold_ms: 0, hold_str: "0".into(), recording: false, action_type: TaskActionType::KeyDown }
            }
            TaskActionType::KeyUp => {
                let vk = step.param as i32;
                EditState::Key { vk, hold_ms: 0, hold_str: "0".into(), recording: false, action_type: TaskActionType::KeyUp }
            }
            TaskActionType::MouseWheel => {
                EditState::Wheel { up: step.param != 0 }
            }
            TaskActionType::ImageMatch => {
                let (image_id, fail_msg_id) = unpack_image_match(step.param);
                let fail_msg = if fail_msg_id != 0 {
                    crate::config::load_msg(fail_msg_id as u64)
                } else {
                    String::new()
                };
                let move_mouse = (step.extra & 0x100) != 0;
                let ignore_failure = (step.extra & 0x200) != 0;
                let full_image = crate::config::load_image(image_id as u64);
                let window_title_id = step.extra >> 10;
                let window_title = if window_title_id != 0 {
                    crate::config::load_msg(window_title_id as u64)
                } else {
                    String::new()
                };
                EditState::ImageMatch {
                    image_id,
                    new_path: String::new(),
                    new_thumbnail: None,
                    full_image,
                    show_preview: false,
                    preview_w: 0,
                    preview_h: 0,
                    confidence_str: (step.extra & 0xFF).to_string(),
                    fail_msg,
                    move_mouse,
                    ignore_failure,
                    window_title,
                }
            }
            TaskActionType::Notify => {
                EditState::Notify {
                    msg: crate::config::load_msg(step.param),
                }
            }
            TaskActionType::LostFocus => EditState::LostFocus,
            TaskActionType::ShowWindow => EditState::ShowWindow,
            TaskActionType::HideWindow => EditState::HideWindow,
            TaskActionType::OpenProgram => {
                EditState::OpenProgram {
                    msg: crate::config::load_msg(step.param),
                }
            }
            TaskActionType::RandomDelay => {
                let min_ms = step.param as u32 as u64;
                let max_ms = step.param >> 32;
                EditState::RandomDelay { min_str: min_ms.to_string(), max_str: max_ms.to_string() }
            }
            TaskActionType::ComboKey => {
                let preset = find_combo_preset(step.param, step.extra);
                EditState::ComboKey { preset }
            }
            TaskActionType::WaitUntil => {
                let hour = step.param as u32 / 60;
                let minute = step.param as u32 % 60;
                EditState::WaitUntil { hour_str: hour.to_string(), minute_str: format!("{:02}", minute) }
            }
            TaskActionType::WaitKey => {
                let vk = step.param as i32;
                EditState::WaitKey {
                    vk,
                    recording: false,
                    terminate: crate::constants::waitkey_terminate_on_wrong(step.extra),
                    popup: crate::constants::waitkey_show_popup(step.extra),
                }
            }
            TaskActionType::WaitInput => {
                EditState::WaitInput {
                    msg: crate::config::load_msg(step.param),
                    copy_input: crate::constants::waitinput_copy(step.extra),
                    ignore_fail: crate::constants::waitinput_ignore_fail(step.extra),
                    use_regex: crate::constants::waitinput_regex(step.extra),
                }
            }
            TaskActionType::CopyText => {
                EditState::CopyText {
                    msg: crate::config::load_msg(step.param),
                }
            }
            TaskActionType::Comment => {
                let count = step.extra;
                EditState::Comment {
                    msg: crate::config::load_msg(step.param),
                    count_str: if count == 0 { String::new() } else { count.to_string() },
                }
            }
            TaskActionType::MouseMove => {
                let (x, y) = unpack_move(step.param);
                let is_rel = (step.extra >> 24) == 1;
                let duration = step.extra & 0xFFFFFF;
                let (preset, window_title) = if x == -2 {
                    let title = crate::config::load_msg(y as u64);
                    (MOUSE_MOVE_PRESETS.len() + 2, title)
                } else if is_rel {
                    (MOUSE_MOVE_PRESETS.len() + 1, String::new())
                } else {
                    let p = if x == -1 && y == -1 { 0 }
                        else if x == 100 && y == 0 { 1 }
                        else if x == -100 && y == 0 { 2 }
                        else if x == 0 && y == 100 { 3 }
                        else if x == 0 && y == -100 { 4 }
                        else { MOUSE_MOVE_PRESETS.len() };
                    (p, String::new())
                };
                let custom_idx = MOUSE_MOVE_PRESETS.len();
                let dist_idx = MOUSE_MOVE_PRESETS.len() + 1;
                let custom_x = if preset == custom_idx || preset == dist_idx { x.to_string() } else { String::new() };
                let custom_y = if preset == custom_idx || preset == dist_idx { y.to_string() } else { String::new() };
                let secs = if duration > 0 {
                    format!("{:.1}", duration as f64 / 1000.0)
                } else {
                    "0.0".into()
                };
                EditState::Move { preset, secs_str: secs, custom_x, custom_y, window_title, is_relative: is_rel }
            }
        };
    }

    pub fn draw_edit_popup(&mut self, ctx: &egui::Context) {
        let edit_idx = match self.edit_index {
            Some(i) if i < self.state.tasks.len() => i,
            _ => { self.edit_index = None; self.edit_state = EditState::None; return; }
        };

        let title = t(self.lang, match self.edit_state {
            EditState::Delay { .. } => "编辑延迟",
            EditState::Click { action_type, .. } => match action_type {
                TaskActionType::MouseDown => "编辑鼠标按下",
                TaskActionType::MouseUp => "编辑鼠标松开",
                _ => "编辑鼠标点击",
            },
            EditState::Key { action_type, .. } => match action_type {
                TaskActionType::KeyPress => "编辑按键",
                TaskActionType::KeyDown => "编辑按下按键",
                TaskActionType::KeyUp => "编辑松开按键",
                _ => "编辑按键",
            },
            EditState::Wheel { .. } => "编辑滚轮",
            EditState::Move { .. } => "编辑移动",
            EditState::ImageMatch { .. } => "编辑识图",
            EditState::Notify { .. } => "编辑通知",
            EditState::LostFocus => "失去焦点",
            EditState::ShowWindow => "显示程序窗口",
            EditState::HideWindow => "隐藏程序窗口",
            EditState::OpenProgram { .. } => "编辑打开程序",
            EditState::RandomDelay { .. } => "编辑随机延迟",
            EditState::ComboKey { .. } => "编辑组合按键",
            EditState::WaitUntil { .. } => "编辑整点延迟",
            EditState::WaitKey { .. } => "编辑等待按键",
            EditState::WaitInput { .. } => "编辑等待输入",
            EditState::CopyText { .. } => "编辑复制文本",
            EditState::Comment { .. } => "编辑备注",
            EditState::None => return,
        }, match self.edit_state {
            EditState::Delay { .. } => "Edit Delay",
            EditState::Click { action_type, .. } => match action_type {
                TaskActionType::MouseDown => "Edit Hold",
                TaskActionType::MouseUp => "Edit Release",
                _ => "Edit Click",
            },
            EditState::Key { action_type, .. } => match action_type {
                TaskActionType::KeyPress => "Edit Key Press",
                TaskActionType::KeyDown => "Edit Key Down",
                TaskActionType::KeyUp => "Edit Key Up",
                _ => "Edit Key Press",
            },
            EditState::Wheel { .. } => "Edit Wheel",
            EditState::Move { .. } => "Edit Move",
            EditState::ImageMatch { .. } => "Edit Image Match",
            EditState::Notify { .. } => "Edit Notify",
            EditState::LostFocus => "Lost Focus",
            EditState::ShowWindow => "Show Window",
            EditState::HideWindow => "Hide Window",
            EditState::OpenProgram { .. } => "Edit Open Program",
            EditState::RandomDelay { .. } => "Edit Random Delay",
            EditState::ComboKey { .. } => "Edit Combo Key",
            EditState::WaitUntil { .. } => "Edit Wait Until",
            EditState::WaitKey { .. } => "Edit Wait Key",
            EditState::WaitInput { .. } => "Edit Wait Input",
            EditState::CopyText { .. } => "Edit Copy Text",
            EditState::Comment { .. } => "Edit Comment",
            EditState::None => return,
        });

        let mut close = false;
        let mut apply = false;

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                match &mut self.edit_state {
                    EditState::Delay { ms_str } => {
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "延迟 (ms):", "Delay (ms):"));
                            ui.add(egui::TextEdit::singleline(ms_str).desired_width(100.0));
                        });
                    }
                    EditState::RandomDelay { min_str, max_str } => {
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "最(ms):", "Min(ms):"));
                            ui.add(egui::TextEdit::singleline(min_str).desired_width(60.0));
                            ui.label(t(self.lang, "  最(ms):", "  Max(ms):"));
                            ui.add(egui::TextEdit::singleline(max_str).desired_width(60.0));
                        });
                    }
                    EditState::ComboKey { preset } => {
                        const COLS: usize = 2;
                        egui::Grid::new("edit_combo_grid").show(ui, |ui| {
                            for (i, (name, keys, _, _)) in crate::constants::COMBO_PRESETS.iter().enumerate() {
                                if selectable_btn(ui, *preset == Some(i), egui::RichText::new(format!("{}|{}", name, keys)).size(11.5)).clicked() { *preset = Some(i); }
                                if (i + 1) % COLS == 0 { ui.end_row(); }
                            }
                        });
                        if let Some(p) = *preset {
                            if p < crate::constants::COMBO_PRESETS.len() {
                                let (_, _, desc, _) = crate::constants::COMBO_PRESETS[p];
                                ui.add_space(4.0);
                                let tmpl = tf(self.lang, "用途: {}", "Usage: {}");
                                ui.label(egui::RichText::new(tmpl.replace("{}", desc)).size(12.0).color(C_SUBTEXT));
                            }
                        }
                    }
                    EditState::WaitUntil { hour_str, minute_str } => {
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "等待到:", "Wait until:"));
                            ui.add(egui::TextEdit::singleline(hour_str).desired_width(40.0).hint_text(t(self.lang, "时", "h")));
                            ui.label(t(self.lang, "时", "h"));
                            ui.add(egui::TextEdit::singleline(minute_str).desired_width(40.0).hint_text(t(self.lang, "分", "m")));
                            ui.label(t(self.lang, "分", "m"));
                        });
                    }
                    EditState::WaitInput { msg, copy_input, ignore_fail, use_regex } => {
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "匹配文本:", "Match text:"));
                            ui.add(egui::TextEdit::singleline(msg).desired_width(200.0));
                        });
                        ui.checkbox(copy_input, t(self.lang, "复制输入内容到剪贴板", "Copy input to clipboard"));
                        ui.checkbox(use_regex, t(self.lang, "使用正则匹配", "Use regex"));
                        ui.checkbox(ignore_fail, t(self.lang, "无视失败继续执行", "Ignore failure"));
                    }
                    EditState::WaitKey { vk, recording, terminate, popup } => {
                        if *recording {
                            ui.label(egui::RichText::new(t(self.lang, "按下目标按键...", "Press target key...")).color(C_ORANGE));
                            for key in [
                                egui::Key::Space, egui::Key::Enter, egui::Key::Tab,
                                egui::Key::A, egui::Key::B, egui::Key::C, egui::Key::D,
                                egui::Key::E, egui::Key::F, egui::Key::G, egui::Key::H,
                                egui::Key::I, egui::Key::J, egui::Key::K, egui::Key::L,
                                egui::Key::M, egui::Key::N, egui::Key::O, egui::Key::P,
                                egui::Key::Q, egui::Key::R, egui::Key::S, egui::Key::T,
                                egui::Key::U, egui::Key::V, egui::Key::W, egui::Key::X,
                                egui::Key::Y, egui::Key::Z,
                                egui::Key::Num0, egui::Key::Num1, egui::Key::Num2, egui::Key::Num3,
                                egui::Key::Num4, egui::Key::Num5, egui::Key::Num6, egui::Key::Num7,
                                egui::Key::Num8, egui::Key::Num9,
                                egui::Key::F1, egui::Key::F2, egui::Key::F3, egui::Key::F4,
                                egui::Key::F5, egui::Key::F6, egui::Key::F7, egui::Key::F8,
                                egui::Key::F9, egui::Key::F10, egui::Key::F11, egui::Key::F12,
                            ] {
                                if ctx.input(|i| i.key_pressed(key)) {
                                    let v = egui_key_to_vk(key);
                                    if v != 0 { *vk = v; *recording = false; }
                                    break;
                                }
                            }
                        } else {
                            ui.horizontal(|ui| {
                                let tmpl = tf(self.lang, "目标按键: {}", "Target key: {}");
                                ui.label(tmpl.replace("{}", &vk_name(*vk, self.lang)));
                                if ui.button(t(self.lang, "重新录制", "Re-record")).clicked() { *recording = true; }
                            });
                        }
                        ui.checkbox(terminate, t(self.lang, "按键错误终止任务", "Terminate on wrong key"));
                        ui.checkbox(popup, t(self.lang, "弹窗提示", "Show popup"));
                    }
                    EditState::Click { button, hold_ms, hold_str, action_type, jitter, jitter_str } => {
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "按键:", "Button:"));
                            for (v, label) in [(0, "左键"), (1, "右键"), (2, "中键")] {
                                if selectable_btn(ui, *button == v, t(self.lang, label, if v == 0 { "Left" } else if v == 1 { "Right" } else { "Middle" })).clicked() { *button = v; }
                            }
                        });
                        if *action_type == TaskActionType::MouseClick {
                            ui.horizontal(|ui| {
                                ui.label(t(self.lang, "长按 (ms):", "Hold (ms):"));
                                ui.add(egui::TextEdit::singleline(hold_str).desired_width(80.0));
                                if *hold_ms > 0 {
                                    let tmpl = tf(self.lang, "当前: {}ms", "Current: {}ms");
                                    ui.label(tmpl.replace("{}", &hold_ms.to_string()));
                                }
                            });
                            if let Ok(v) = hold_str.parse::<u64>() {
                                *hold_ms = v.min(60000);
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "抖动范围:", "Jitter:"));
                            ui.add(egui::TextEdit::singleline(jitter_str).desired_width(60.0));
                            ui.label(t(self.lang, "px (0=不抖动)", "px (0=off)"));
                        });
                        if let Ok(v) = jitter_str.parse::<u32>() {
                            *jitter = v.min(50);
                        }
                    }
                    EditState::Key { vk, hold_ms, hold_str, recording, action_type } => {
                        if *recording {
                            ui.label(egui::RichText::new(t(self.lang, "请按下一个键…", "Press a key…")).color(C_GREEN));
                            for key in [
                                egui::Key::Space, egui::Key::Enter, egui::Key::Tab,
                                egui::Key::A, egui::Key::B, egui::Key::C, egui::Key::D,
                                egui::Key::E, egui::Key::F, egui::Key::G, egui::Key::H,
                                egui::Key::I, egui::Key::J, egui::Key::K, egui::Key::L,
                                egui::Key::M, egui::Key::N, egui::Key::O, egui::Key::P,
                                egui::Key::Q, egui::Key::R, egui::Key::S, egui::Key::T,
                                egui::Key::U, egui::Key::V, egui::Key::W, egui::Key::X,
                                egui::Key::Y, egui::Key::Z,
                                egui::Key::Num0, egui::Key::Num1, egui::Key::Num2, egui::Key::Num3,
                                egui::Key::Num4, egui::Key::Num5, egui::Key::Num6, egui::Key::Num7,
                                egui::Key::Num8, egui::Key::Num9,
                                egui::Key::F1, egui::Key::F2, egui::Key::F3, egui::Key::F4,
                                egui::Key::F5, egui::Key::F6, egui::Key::F7, egui::Key::F8,
                                egui::Key::F9, egui::Key::F10, egui::Key::F11, egui::Key::F12,
                            ] {
                                if ctx.input(|i| i.key_pressed(key)) {
                                    let v = egui_key_to_vk(key);
                                    if v != 0 { *vk = v; *recording = false; }
                                    break;
                                }
                            }
                        } else {
                            ui.horizontal(|ui| {
                                let tmpl = tf(self.lang, "按键: {}", "Key: {}");
                                ui.label(tmpl.replace("{}", &vk_name(*vk, self.lang)));
                                if ui.button(t(self.lang, "重新录制", "Re-record")).clicked() { *recording = true; }
                            });
                        }
                        if *action_type == TaskActionType::KeyPress {
                            ui.horizontal(|ui| {
                                ui.label(t(self.lang, "长按 (ms):", "Hold (ms):"));
                                ui.add(egui::TextEdit::singleline(hold_str).desired_width(80.0));
                            });
                            if let Ok(v) = hold_str.parse::<u64>() {
                                *hold_ms = v.min(60000);
                            }
                        }
                    }
                    EditState::Wheel { up } => {
                        ui.horizontal(|ui| {
                            for (v, label) in [(true, "上滚"), (false, "下滚")] {
                                if selectable_btn(ui, *up == v, t(self.lang, label, if v { "Up" } else { "Down" })).clicked() { *up = v; }
                            }
                        });
                    }
                    EditState::Move { preset, secs_str, custom_x, custom_y, window_title, is_relative } => {
                        ui.label(t(self.lang, "移动鼠标:", "Move mouse:"));
                        let custom_idx = MOUSE_MOVE_PRESETS.len();
                        let dist_idx = MOUSE_MOVE_PRESETS.len() + 1;
                        let window_idx = MOUSE_MOVE_PRESETS.len() + 2;
                        egui::Grid::new("move_presets_edit").spacing([10.0, 4.0]).show(ui, |ui| {
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
                            ui.add_space(2.0);
                            let (hint_x, hint_y) = if *is_relative { ("ΔX", "ΔY") } else { ("X", "Y") };
                            ui.horizontal(|ui| {
                                ui.label(format!("{}:", hint_x));
                                ui.add(egui::TextEdit::singleline(custom_x).desired_width(60.0));
                                ui.label(format!("{}:", hint_y));
                                ui.add(egui::TextEdit::singleline(custom_y).desired_width(60.0));
                            });
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(
                                t(self.lang, if *is_relative { "相对当前光标位置偏移" } else { "屏幕绝对坐标" }, if *is_relative { "Offset from cursor" } else { "Screen absolute" })
                            ).size(11.0).color(C_SUBTEXT));
                        } else if *preset == window_idx {
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label(t(self.lang, "窗口标题:", "Window title:"));
                                ui.add(egui::TextEdit::singleline(window_title).desired_width(160.0));
                            });
                        }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "时间(秒):", "Time(s):"));
                            let mut secs = secs_str.parse::<f64>().unwrap_or(1.0);
                            if ui.add(egui::DragValue::new(&mut secs).range(0.0..=60.0).speed(0.1)).changed() {
                                *secs_str = format!("{:.1}", secs);
                            }
                        });
                    }
                    EditState::ImageMatch { image_id, new_path, new_thumbnail, full_image, show_preview, preview_w, preview_h, confidence_str, fail_msg, move_mouse, ignore_failure, window_title } => {
                        if let Some(p) = self.picked_path.take() {
                            *new_path = p.clone();
                            *full_image = std::fs::read(&p).ok();
                            *preview_w = 0;
                            *preview_h = 0;
                            *new_thumbnail = full_image.as_ref().and_then(|data| {
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
                                if !new_path.is_empty() {
                                    ui.label(egui::RichText::new(&*new_path).size(11.0).color(C_SUBTEXT));
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
                                    let uri = format!("bytes://full_edit_{}", image_id);
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
                        ui.horizontal(|ui| {
                            if let Some(ref png_data) = new_thumbnail {
                                let uri = format!("bytes://thumbnail_new_{}", new_path);
                                let img = egui::Image::from_bytes(uri, png_data.clone());
                                if ui.add(egui::Button::image(img.max_size(egui::Vec2::splat(48.0)).corner_radius(3)).frame(false)).clicked() {
                                    *show_preview = true;
                                }
                                ui.add_space(8.0);
                            } else if *image_id != 0 {
                                if let Some(Some(ref png_data)) = self.thumbnail_cache.get(&(*image_id as u64)) {
                                    let uri = format!("bytes://thumbnail_edit_{}", image_id);
                                    let img = egui::Image::from_bytes(uri, png_data.clone());
                                    if ui.add(egui::Button::image(img.max_size(egui::Vec2::splat(48.0)).corner_radius(3)).frame(false)).clicked() {
                                        *show_preview = true;
                                    }
                                    ui.add_space(8.0);
                                }
                            }
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(t(self.lang, "图片:", "Image:"));
                                    ui.add(egui::TextEdit::singleline(new_path).desired_width(160.0).hint_text(t(self.lang, "留空不更改", "Leave empty to keep")));
                                    if ui.button(t(self.lang, "浏览...", "Browse...")).clicked() {
                                        self.pending_file_pick = Some(super::FilePickKind::Image);
                                    }
                                });
                                if !new_path.is_empty() {
                                    let tmpl = tf(self.lang, "新图: {}", "New: {}");
                                    ui.label(egui::RichText::new(tmpl.replace("{}", new_path)).size(11.0).color(C_SUBTEXT));
                                }
                                ui.add_space(2.0);
                                ui.horizontal(|ui| {
                                    ui.label(t(self.lang, "可信度(1-100):", "Confidence(1-100):"));
                                    ui.add(egui::TextEdit::singleline(confidence_str).desired_width(40.0));
                                    ui.label(t(self.lang, "%", "%"));
                                });
                                ui.add_space(2.0);
                                ui.checkbox(move_mouse, t(self.lang, "移动鼠标到匹配位置", "Move mouse to match"));
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(t(self.lang, "失败提示:", "Failure tip:"));
                                    ui.add(egui::TextEdit::singleline(fail_msg).desired_width(180.0).hint_text(t(self.lang, "留空则不提示", "Leave empty for no tip")));
                                });
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(t(self.lang, "窗口标题:", "Window title:"));
                                    ui.add(egui::TextEdit::singleline(window_title).desired_width(140.0).hint_text(t(self.lang, "留空=全屏搜索", "Empty=full screen")));
                                });
                                ui.add_space(2.0);
                                ui.checkbox(ignore_failure, t(self.lang, "无视失败继续执行", "Ignore failure"));
                            });
                        });
                        }
                    }
                    EditState::Notify { msg } => {
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "消息:", "Message:"));
                            ui.add(egui::TextEdit::singleline(msg).desired_width(200.0));
                        });
                    }
                    EditState::CopyText { msg } => {
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "文本:", "Text:"));
                            ui.add(egui::TextEdit::singleline(msg).desired_width(200.0));
                        });
                    }
                    EditState::Comment { msg, count_str } => {
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "内容:", "Content:"));
                            ui.add(egui::TextEdit::singleline(msg).desired_width(180.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "范围:", "Range:"));
                            ui.add(egui::TextEdit::singleline(count_str).desired_width(80.0).hint_text("0"));
                            ui.label(egui::RichText::new(t(self.lang, "覆盖后续任务数", "Tasks to cover")).size(11.0).color(C_SUBTEXT));
                        });
                    }
                    EditState::LostFocus => {
                        ui.label(t(self.lang, "失去焦点 — 最小化窗口释放焦点", "Lost Focus — Minimize window to release focus"));
                    }
                    EditState::ShowWindow => {
                        ui.label(t(self.lang, "显示程序窗口 — 将窗口显示出来", "Show Window — Show the app window"));
                    }
                    EditState::HideWindow => {
                        ui.label(t(self.lang, "隐藏程序窗口 — 将窗口隐藏起来", "Hide Window — Hide the app window"));
                    }
                    EditState::OpenProgram { msg } => {
                        ui.horizontal(|ui| {
                            ui.label(t(self.lang, "程序路径:", "Program path:"));
                            ui.add(egui::TextEdit::singleline(msg).desired_width(250.0).hint_text(t(self.lang, "如 C:\\Windows\\notepad.exe", "e.g. C:\\Windows\\notepad.exe")));
                        });
                        ui.add_space(4.0);
                        if ui.button(t(self.lang, "浏览...", "Browse...")).clicked() {
                            self.pending_file_pick = Some(super::FilePickKind::Exe);
                        }
                        if let Some(p) = self.picked_path.take() {
                            *msg = p;
                        }
                    }
                    EditState::None => {}
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(t(self.lang, "确定", "OK")).clicked() {
                        apply = true;
                    }
                    if ui.button(t(self.lang, "取消", "Cancel")).clicked() {
                        close = true;
                    }
                });
            });

        if apply {
            let idx = edit_idx;
            let new_param = match &self.edit_state {
                EditState::Delay { ms_str } => {
                    ms_str.parse::<u64>().unwrap_or(100).clamp(1, 60000)
                }
                EditState::Click { button, hold_ms, action_type, jitter, .. } => {
                    let param = if *action_type == TaskActionType::MouseClick {
                        (*button as u64 & 0xFF) | (*hold_ms << 8)
                    } else {
                        *button as u64
                    };
                    self.state.tasks[idx].action = *action_type;
                    self.state.tasks[idx].extra = *jitter;
                    param
                }
                EditState::Key { vk, hold_ms, action_type, .. } => {
                    self.state.tasks[idx].action = *action_type;
                    if *action_type == TaskActionType::KeyPress {
                        (*vk as u64 & 0xFFFF) | (*hold_ms << 16)
                    } else {
                        *vk as u64 & 0xFFFF
                    }
                }
                EditState::Wheel { up } => {
                    if *up { 1 } else { 0 }
                }
                EditState::Move { preset, secs_str, custom_x, custom_y, window_title, is_relative } => {
                    let window_idx = MOUSE_MOVE_PRESETS.len() + 2;
                    let (x, y) = if *preset < MOUSE_MOVE_PRESETS.len() {
                        let (px, py, _) = MOUSE_MOVE_PRESETS[*preset];
                        (px, py)
                    } else if *preset == window_idx {
                        crate::config::delete_msg(unpack_move(self.state.tasks[idx].param).1 as u64);
                        let new_id = if window_title.is_empty() { 0 } else { crate::config::save_msg(window_title) };
                        (-2, new_id as i32)
                    } else {
                        let cx = custom_x.parse::<i32>().unwrap_or(0);
                        let cy = custom_y.parse::<i32>().unwrap_or(0);
                        (cx, cy)
                    };
                    let duration = (secs_str.parse::<f64>().unwrap_or(1.0) * 1000.0) as u32;
                    self.state.tasks[idx].param = pack_move(x, y);
                    self.state.tasks[idx].extra = if *is_relative { duration | RELATIVE_FLAG } else { duration };
                    pack_move(x, y)
                }
                EditState::ImageMatch { confidence_str, fail_msg, move_mouse, ignore_failure, window_title, new_path, .. } => {
                    let conf = confidence_str.parse::<u8>().unwrap_or(85);
                    let old_window_id = self.state.tasks[idx].extra >> 10;
                    crate::config::delete_msg(old_window_id as u64);
                    let window_id = if window_title.is_empty() {
                        0u32
                    } else {
                        crate::config::save_msg(window_title) as u32
                    };
                    let extra = conf as u32
                        | (if *move_mouse { 0x100 } else { 0 })
                        | (if *ignore_failure { 0x200 } else { 0 })
                        | (window_id << 10);
                    self.state.tasks[idx].extra = extra;
                    let (old_image_id, old_fail_id) = unpack_image_match(self.state.tasks[idx].param);
                    let image_id = if !new_path.is_empty() {
                        if let Ok(data) = std::fs::read(new_path) {
                            crate::config::save_image(&data) as u32
                        } else {
                            old_image_id
                        }
                    } else {
                        old_image_id
                    };
                    crate::config::delete_msg(old_fail_id as u64);
                    let new_fail_id = if fail_msg.is_empty() {
                        0u32
                    } else {
                        crate::config::save_msg(fail_msg) as u32
                    };
                    pack_image_match(image_id, new_fail_id)
                }
                EditState::Notify { msg } => {
                    crate::config::delete_msg(self.state.tasks[idx].param);
                    crate::config::save_msg(msg)
                }
                EditState::CopyText { msg } => {
                    crate::config::delete_msg(self.state.tasks[idx].param);
                    crate::config::save_msg(msg)
                }
                EditState::Comment { msg, count_str } => {
                    crate::config::delete_msg(self.state.tasks[idx].param);
                    let new_id = if msg.is_empty() { 0 } else { crate::config::save_msg(msg) };
                    let count_val = count_str.parse::<u32>().unwrap_or(0);
                    self.state.tasks[idx].extra = count_val;
                    new_id
                }
                EditState::LostFocus => 0,
                EditState::ShowWindow => 0,
                EditState::HideWindow => 0,
                EditState::OpenProgram { msg } => {
                    crate::config::delete_msg(self.state.tasks[idx].param);
                    if msg.is_empty() { 0 } else { crate::config::save_msg(msg) }
                }
                EditState::RandomDelay { min_str, max_str } => {
                    let min_val = min_str.parse::<u64>().unwrap_or(100).clamp(1, 60000);
                    let max_val = max_str.parse::<u64>().unwrap_or(1000).clamp(1, 60000);
                    let (lo, hi) = if min_val <= max_val { (min_val, max_val) } else { (max_val, min_val) };
                    lo | (hi << 32)
                }
                EditState::ComboKey { preset, .. } => {
                    if let Some(p) = *preset {
                        if p < crate::constants::COMBO_PRESETS.len() {
                            let (_, _, _, vks) = crate::constants::COMBO_PRESETS[p];
                            self.state.tasks[idx].extra = vks.len() as u32;
                            pack_combo(vks)
                        } else {
                            self.state.tasks[idx].param
                        }
                    } else {
                        self.state.tasks[idx].param
                    }
                }
                EditState::WaitUntil { hour_str, minute_str } => {
                    let h = hour_str.parse::<u32>().unwrap_or(0).min(23);
                    let m = minute_str.parse::<u32>().unwrap_or(0).min(59);
                    self.state.tasks[idx].action = TaskActionType::WaitUntil;
                    (h * 60 + m) as u64
                }
                EditState::WaitKey { vk, terminate, popup, .. } => {
                    let mut extra: u32 = 0;
                    if *terminate { extra |= WAITKEY_TERMINATE; }
                    if *popup { extra |= WAITKEY_POPUP; }
                    self.state.tasks[idx].action = TaskActionType::WaitKey;
                    self.state.tasks[idx].extra = extra;
                    *vk as u64
                }
                EditState::WaitInput { msg, copy_input, ignore_fail, use_regex } => {
                    crate::config::delete_msg(self.state.tasks[idx].param);
                    let mut extra: u32 = 0;
                    if *copy_input { extra |= WAITINPUT_COPY; }
                    if *ignore_fail { extra |= WAITINPUT_IGNORE_FAIL; }
                    if *use_regex { extra |= WAITINPUT_REGEX; }
                    self.state.tasks[idx].action = TaskActionType::WaitInput;
                    self.state.tasks[idx].extra = extra;
                    crate::config::save_msg(msg)
                }
                _ => return,
            };
            self.state.tasks[idx].param = new_param;
            crate::config::save_tasks(&self.state.tasks);
            self.edit_index = None;
            self.edit_state = EditState::None;
        }
        if close {
            self.edit_index = None;
            self.edit_state = EditState::None;
        }
    }
}
