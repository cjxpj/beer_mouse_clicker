//! UI 工具函数：虚拟键名、任务描述、选择按钮、面板边框等

use crate::app::theme::*;
use crate::constants::*;
use crate::lang::{t, tf, Lang};
use egui::{Color32, Frame};

const SEL_FILL: Color32 = Color32::from_rgb(50, 75, 125);

// ── 颜色工具 ──────────────────────────────────────────────

pub fn dim_color_egui(c: Color32) -> Color32 {
    Color32::from_rgb(c.r() / 2, c.g() / 2, c.b() / 2)
}

pub fn brighten_color(c: Color32, amount: i32) -> Color32 {
    Color32::from_rgb(
        (c.r() as i32 + amount).clamp(0, 255) as u8,
        (c.g() as i32 + amount).clamp(0, 255) as u8,
        (c.b() as i32 + amount).clamp(0, 255) as u8,
    )
}

// ── UI 组件 ────────────────────────────────────────────────

pub fn selectable_btn(ui: &mut egui::Ui, selected: bool, label: impl Into<egui::WidgetText>) -> egui::Response {
    let mut btn = egui::Button::new(label);
    if selected { btn = btn.fill(SEL_FILL); }
    ui.add(btn)
}

// ── 面板边框 ──────────────────────────────────────────────

pub fn make_panel_frame() -> Frame {
    Frame::NONE
        .fill(C_BG)
        .inner_margin(egui::Margin::symmetric(8, 4))
}

pub fn make_central_frame() -> Frame {
    Frame::NONE
        .fill(C_BG)
        .inner_margin(egui::Margin::symmetric(8, 4))
}

// ── 虚拟键名 ──────────────────────────────────────────────

/// 虚拟键码 → 可读名称
pub fn vk_name(vk: i32, lang: Lang) -> String {
    use winapi::um::winuser::{
        VK_LBUTTON, VK_RBUTTON, VK_MBUTTON, VK_XBUTTON1, VK_XBUTTON2,
    };
    match vk {
        // 功能键
        0x70..=0x7B => format!("F{}", vk - 0x6F),
        // 字母
        0x41..=0x5A => format!("{}", (vk as u8) as char),
        // 数字
        0x30..=0x39 => format!("{}", vk - 0x30),
        // 小键盘
        0x60..=0x69 => {
            let tmpl = tf(lang, "小键盘{}", "Num{}");
            tmpl.replace("{}", &(vk - 0x60).to_string())
        }
        // 空格回车退格等
        0x20 => t(lang, "空格", "Space").into(),
        0x0D => t(lang, "回车", "Enter").into(),
        0x09 => "Tab".into(),
        0x08 => t(lang, "退格", "Backspace").into(),
        0x1B => "Esc".into(),
        // 方向键
        0x25 => "←".into(),
        0x26 => "↑".into(),
        0x27 => "→".into(),
        0x28 => "↓".into(),
        // 编辑键
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x24 => "Home".into(),
        0x23 => "End".into(),
        0x21 => "PageUp".into(),
        0x22 => "PageDown".into(),
        // 修饰键
        0x10 | 0xA0 | 0xA1 => "Shift".into(),
        0x11 | 0xA2 | 0xA3 => "Ctrl".into(),
        0x12 | 0xA4 | 0xA5 => "Alt".into(),
        0x5B | 0x5C => "Win".into(),
        // 系统键
        0x13 => "Pause".into(),
        0x2C => "PrintScreen".into(),
        0x91 => "ScrollLock".into(),
        0x14 => "CapsLock".into(),
        0x90 => "NumLock".into(),
        // 鼠标键
        v if v == VK_LBUTTON => t(lang, "鼠标左键", "LButton").into(),
        v if v == VK_RBUTTON => t(lang, "鼠标右键", "RButton").into(),
        v if v == VK_MBUTTON => t(lang, "鼠标中键", "MButton").into(),
        v if v == VK_XBUTTON1 => t(lang, "鼠标侧键1", "X1").into(),
        v if v == VK_XBUTTON2 => t(lang, "鼠标侧键2", "X2").into(),
        // 小键盘其他键
        0x6A => t(lang, "小键盘*", "Num*").into(),
        0x6B => t(lang, "小键盘+", "Num+").into(),
        0x6D => t(lang, "小键盘-", "Num-").into(),
        0x6E => t(lang, "小键盘.", "Num.").into(),
        0x6F => t(lang, "小键盘/", "Num/").into(),
        // OEM 键
        0xBA => ";".into(),
        0xBB => "=".into(),
        0xBC => ",".into(),
        0xBD => "-".into(),
        0xBE => ".".into(),
        0xBF => "/".into(),
        0xC0 => "`".into(),
        0xDB => "[".into(),
        0xDC => "\\".into(),
        0xDD => "]".into(),
        0xDE => "'".into(),
        // 菜单键
        0x5D => t(lang, "菜单键", "Menu").into(),
        // 浏览器等多媒体键
        0xA6 | 0xA7 => t(lang, "浏览器", "Browser").into(),
        0xA8 => t(lang, "收藏夹", "Favorites").into(),
        0xA9 => t(lang, "浏览器主页", "Home").into(),
        0xAC => t(lang, "浏览器搜索", "Search").into(),
        0xB4 => t(lang, "启动邮件", "Mail").into(),
        0xAD => t(lang, "静音", "Mute").into(),
        0xAE => t(lang, "音量-", "Vol-").into(),
        0xAF => t(lang, "音量+", "Vol+").into(),
        0xB0 => t(lang, "下一曲", "Next").into(),
        0xB1 => t(lang, "上一曲", "Prev").into(),
        0xB2 => t(lang, "停止播放", "Stop").into(),
        0xB3 => t(lang, "播放/暂停", "Play").into(),
        0xE2 => "OEM_102".into(),
        _ => format!("VK:{:#X}", vk),
    }
}

// ── egui Key → VK ─────────────────────────────────────────

pub fn egui_key_to_vk(key: egui::Key) -> i32 {
    use egui::Key;
    match key {
        Key::F1 => 0x70, Key::F2 => 0x71, Key::F3 => 0x72, Key::F4 => 0x73,
        Key::F5 => 0x74, Key::F6 => 0x75, Key::F7 => 0x76, Key::F8 => 0x77,
        Key::F9 => 0x78, Key::F10 => 0x79, Key::F11 => 0x7A, Key::F12 => 0x7B,
        Key::Space => 0x20,
        Key::Enter => 0x0D,
        Key::Tab => 0x09,
        Key::A => 0x41, Key::B => 0x42, Key::C => 0x43, Key::D => 0x44,
        Key::E => 0x45, Key::F => 0x46, Key::G => 0x47, Key::H => 0x48,
        Key::I => 0x49, Key::J => 0x4A, Key::K => 0x4B, Key::L => 0x4C,
        Key::M => 0x4D, Key::N => 0x4E, Key::O => 0x4F, Key::P => 0x50,
        Key::Q => 0x51, Key::R => 0x52, Key::S => 0x53, Key::T => 0x54,
        Key::U => 0x55, Key::V => 0x56, Key::W => 0x57, Key::X => 0x58,
        Key::Y => 0x59, Key::Z => 0x5A,
        Key::Num0 => 0x30, Key::Num1 => 0x31, Key::Num2 => 0x32, Key::Num3 => 0x33,
        Key::Num4 => 0x34, Key::Num5 => 0x35, Key::Num6 => 0x36, Key::Num7 => 0x37,
        Key::Num8 => 0x38, Key::Num9 => 0x39,
        _ => 0,
    }
}

pub fn is_mouse_vk(vk: i32) -> bool {
    use winapi::um::winuser::{
        VK_LBUTTON, VK_RBUTTON, VK_MBUTTON, VK_XBUTTON1, VK_XBUTTON2,
    };
    vk == VK_LBUTTON || vk == VK_RBUTTON || vk == VK_MBUTTON
        || vk == VK_XBUTTON1 || vk == VK_XBUTTON2
}

// ── 任务步骤描述 ──────────────────────────────────────────

pub fn task_desc_egui(step: &TaskStep, lang: Lang) -> String {
    match step.action {
        TaskActionType::MouseClick => {
            let btn = match (step.param & 0xFF) as i32 {
                1 => t(lang, "右键", "Right"), 2 => t(lang, "中键", "Middle"), _ => t(lang, "左键", "Left"),
            };
            let hold_ms = step.param >> 8;
            let jitter = step.extra;
            let base = if hold_ms > 0 {
                let tmpl = tf(lang, "鼠标长按{btn} {hold_ms}ms", "Mouse long-press {btn} {hold_ms}ms");
                tmpl.replace("{btn}", btn).replace("{hold_ms}", &hold_ms.to_string())
            } else {
                let tmpl = tf(lang, "鼠标{btn}点击", "Mouse {btn} click");
                tmpl.replace("{btn}", btn)
            };
            if jitter > 0 {
                format!("{} ±{}px", base, jitter)
            } else {
                base
            }
        }
        TaskActionType::Delay => {
            let tmpl = tf(lang, "延迟 {ms} ms", "Delay {ms} ms");
            tmpl.replace("{ms}", &step.param.to_string())
        }
        TaskActionType::KeyPress => {
            let vk = (step.param & 0xFFFF) as i32;
            let hold_ms = step.param >> 16;
            let key = vk_name(vk, lang);
            if hold_ms > 0 {
                let tmpl = tf(lang, "键盘长按{key} {hold_ms}ms", "Key long-press {key} {hold_ms}ms");
                tmpl.replace("{key}", &key).replace("{hold_ms}", &hold_ms.to_string())
            } else {
                let tmpl = tf(lang, "按键 {key}", "Key {key}");
                tmpl.replace("{key}", &key)
            }
        }
        TaskActionType::MouseDown => {
            let btn = match step.param as i32 {
                1 => t(lang, "右键", "Right"), 2 => t(lang, "中键", "Middle"), _ => t(lang, "左键", "Left"),
            };
            let jitter = step.extra;
            if jitter > 0 {
                let tmpl = tf(lang, "鼠标{btn}按下 ±{jitter}px", "Mouse {btn} down ±{jitter}px");
                tmpl.replace("{btn}", btn).replace("{jitter}", &jitter.to_string())
            } else {
                let tmpl = tf(lang, "鼠标{btn}按下", "Mouse {btn} down");
                tmpl.replace("{btn}", btn)
            }
        }
        TaskActionType::MouseUp => {
            let btn = match step.param as i32 {
                1 => t(lang, "右键", "Right"), 2 => t(lang, "中键", "Middle"), _ => t(lang, "左键", "Left"),
            };
            let jitter = step.extra;
            if jitter > 0 {
                let tmpl = tf(lang, "鼠标{btn}松开 ±{jitter}px", "Mouse {btn} up ±{jitter}px");
                tmpl.replace("{btn}", btn).replace("{jitter}", &jitter.to_string())
            } else {
                let tmpl = tf(lang, "鼠标{btn}松开", "Mouse {btn} up");
                tmpl.replace("{btn}", btn)
            }
        }
        TaskActionType::KeyDown => {
            let vk = step.param as i32;
            let tmpl = tf(lang, "按下按键 {key}", "Key down {key}");
            tmpl.replace("{key}", &vk_name(vk, lang))
        }
        TaskActionType::KeyUp => {
            let vk = step.param as i32;
            let tmpl = tf(lang, "松开按键 {key}", "Key up {key}");
            tmpl.replace("{key}", &vk_name(vk, lang))
        }
        TaskActionType::MouseWheel => {
            if step.param != 0 { t(lang, "滚轮上滚", "Wheel up").into() } else { t(lang, "滚轮下滚", "Wheel down").into() }
        }
        TaskActionType::MouseMove => {
            let (x, y) = unpack_move(step.param);
            let is_relative = (step.extra >> 24) == 1;
            let duration = step.extra & 0xFFFFFF;
            let target = if x == -2 {
                let title = crate::config::load_msg(y as u64);
                if title.len() > 12 {
                    let tmpl = tf(lang, "窗口: {title}...", "Window: {title}...");
                    tmpl.replace("{title}", &title[..12])
                } else {
                    let tmpl = tf(lang, "窗口: {title}", "Window: {title}");
                    tmpl.replace("{title}", &title)
                }
            } else if x == -1 && y == -1 {
                t(lang, "居中", "Center").to_string()
            } else if x == 100 && y == 0 {
                "→".to_string()
            } else if x == -100 && y == 0 {
                "←".to_string()
            } else if x == 0 && y == 100 {
                "↓".to_string()
            } else if x == 0 && y == -100 {
                "↑".to_string()
            } else if is_relative {
                let tmpl = tf(lang, "距({x}, {y})", "Offset({x}, {y})");
                tmpl.replace("{x}", &x.to_string()).replace("{y}", &y.to_string())
            } else {
                let tmpl = tf(lang, "坐标({x}, {y})", "Pos({x}, {y})");
                tmpl.replace("{x}", &x.to_string()).replace("{y}", &y.to_string())
            };
            if duration > 0 {
                let tmpl = tf(lang, "移动 {target} {secs}s", "Move {target} {secs}s");
                tmpl.replace("{target}", &target).replace("{secs}", &format!("{:.1}", duration as f64 / 1000.0))
            } else {
                let tmpl = tf(lang, "移动 {target}", "Move {target}");
                tmpl.replace("{target}", &target)
            }
        }
        TaskActionType::ImageMatch => {
            let conf = (step.extra & 0xFF).max(1);
            let window_id = step.extra >> 10;
            let tmpl = tf(lang, "识图 (可信≥{conf}%)", "Match (conf≥{conf}%)");
            let mut desc = tmpl.replace("{conf}", &conf.to_string());
            if window_id != 0 {
                let win = crate::config::load_msg(window_id as u64);
                if !win.is_empty() {
                    let short: String = if win.chars().count() > 8 {
                        win.chars().take(8).chain("...".chars()).collect()
                    } else {
                        win
                    };
                    desc.push_str(&format!(" [{}]", short));
                }
            }
            if (step.extra & 0x200) != 0 {
                desc.push_str(t(lang, " 忽略失败", " ignore fail"));
            }
            desc
        }
        TaskActionType::Notify => {
            let msg = crate::config::load_msg(step.param);
            if msg.len() > 20 {
                let tmpl = tf(lang, "通知: {msg}...", "Notify: {msg}...");
                tmpl.replace("{msg}", &msg[..20])
            } else {
                let tmpl = tf(lang, "通知: {msg}", "Notify: {msg}");
                tmpl.replace("{msg}", &msg)
            }
        }
        TaskActionType::LostFocus => t(lang, "失去焦点", "Lost focus").to_string(),
        TaskActionType::RandomDelay => {
            let min_ms = step.param as u32 as u64;
            let max_ms = step.param >> 32;
            let tmpl = tf(lang, "随机延迟 {min}~{max}ms", "Random {min}~{max}ms");
            tmpl.replace("{min}", &min_ms.to_string()).replace("{max}", &max_ms.to_string())
        }
        TaskActionType::ComboKey => {
            if let Some(preset) = crate::constants::find_combo_preset(step.param, step.extra) {
                let (name, keys, _, _) = crate::constants::COMBO_PRESETS[preset];
                let tmpl = tf(lang, "组合按键: {name} ({keys})", "Combo: {name} ({keys})");
                tmpl.replace("{name}", name).replace("{keys}", keys)
            } else {
                let keys = crate::constants::unpack_combo(step.param, step.extra);
                let names: Vec<String> = keys.iter().map(|&vk| vk_name(vk as i32, lang)).collect();
                let tmpl = tf(lang, "组合按键: {names}", "Combo: {names}");
                tmpl.replace("{names}", &names.join("+"))
            }
        }
        TaskActionType::WaitUntil => {
            let total_min = step.param as u32;
            let h = total_min / 60;
            let m = total_min % 60;
            let tmpl = tf(lang, "整点延迟 {h}:{mm}", "Wait {h}:{mm}");
            tmpl.replace("{h}", &h.to_string()).replace("{mm}", &format!("{:02}", m))
        }
        TaskActionType::WaitKey => {
            let vk = step.param as i32;
            let key_name = vk_name(vk, lang);
            let flags = if crate::constants::waitkey_terminate_on_wrong(step.extra) {
                t(lang, "错键终止", "ErrStop").to_string()
            } else {
                String::new()
            };
            let popup_tag = if crate::constants::waitkey_show_popup(step.extra) {
                t(lang, "弹窗", "Popup").to_string()
            } else {
                String::new()
            };
            let mut parts = Vec::new();
            let tmpl_base = tf(lang, "等待按键 {key}", "WaitKey {key}");
            parts.push(tmpl_base.replace("{key}", &key_name));
            if !flags.is_empty() { parts.push(flags); }
            if !popup_tag.is_empty() { parts.push(popup_tag); }
            parts.join(" ")
        }
        TaskActionType::CopyText => {
            let msg = crate::config::load_msg(step.param);
            if msg.len() > 20 {
                let tmpl = tf(lang, "复制文本: {msg}...", "Copy text: {msg}...");
                tmpl.replace("{msg}", &msg[..20])
            } else {
                let tmpl = tf(lang, "复制文本: {msg}", "Copy text: {msg}");
                tmpl.replace("{msg}", &msg)
            }
        }
        TaskActionType::WaitInput => {
            let msg = crate::config::load_msg(step.param);
            let mut parts = Vec::new();
            let short = if msg.len() > 20 {
                let tmpl = tf(lang, "等待输入: {msg}...", "WaitInput: {msg}...");
                tmpl.replace("{msg}", &msg[..20])
            } else {
                let tmpl = tf(lang, "等待输入: {msg}", "WaitInput: {msg}");
                tmpl.replace("{msg}", &msg)
            };
            parts.push(short);
            if crate::constants::waitinput_regex(step.extra) {
                parts.push(t(lang, "正则", "Regex").to_string());
            }
            if crate::constants::waitinput_copy(step.extra) {
                parts.push(t(lang, "复制", "Copy").to_string());
            }
            if crate::constants::waitinput_ignore_fail(step.extra) {
                parts.push(t(lang, "忽略失败", "IgnoreFail").to_string());
            }
            parts.join(" ")
        }
        TaskActionType::Comment => {
            let msg = crate::config::load_msg(step.param);
            let tag = if step.extra > 0 { format!("[{}]", step.extra) } else { String::new() };
            let body = if msg.len() > 25 {
                format!("{}...", &msg[..25])
            } else if msg.is_empty() {
                String::new()
            } else {
                msg
            };
            match (tag.is_empty(), body.is_empty()) {
                (true, true) => t(lang, "备注", "Comment").into(),
                (false, true) => {
                    let tmpl = tf(lang, "备注 {tag}", "Comment {tag}");
                    tmpl.replace("{tag}", &tag)
                }
                (true, false) => {
                    let tmpl = tf(lang, "备注 {body}", "Comment {body}");
                    tmpl.replace("{body}", &body)
                }
                (false, false) => {
                    let tmpl = tf(lang, "备注 {tag} {body}", "Comment {tag} {body}");
                    tmpl.replace("{tag}", &tag).replace("{body}", &body)
                }
            }
        }
        TaskActionType::ShowWindow => t(lang, "显示程序窗口", "Show Window").into(),
        TaskActionType::HideWindow => t(lang, "隐藏程序窗口", "Hide Window").into(),
        TaskActionType::OpenProgram => {
            let path = crate::config::load_msg(step.param);
            if path.is_empty() {
                t(lang, "打开程序", "Open Program").into()
            } else {
                let short: String = if path.len() > 30 {
                    format!("{}...", &path[..30])
                } else {
                    path
                };
                let tmpl = tf(lang, "打开程序: {path}", "Open: {path}");
                tmpl.replace("{path}", &short)
            }
        }
    }
}
