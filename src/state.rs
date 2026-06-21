use crate::constants::{TaskActionType, TaskStep, unpack_move, unpack_image_match};
use crate::input::{send_keyboard_input, send_mouse_input, send_wheel_input, send_smooth_mouse_move, get_screen_size, capture_screen, template_match, move_to_window_center};
use crate::lang::{t, tf, Lang};

use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use winapi::shared::windef::HHOOK;
use winapi::um::synchapi::Sleep;
use rand::Rng;

/// 可中断 sleep：每 100ms 检查一次 stop 标志，被中断返回 false
fn interruptible_sleep(ms: u32, clicking: &AtomicBool, epoch: &AtomicU64, my_epoch: u64) -> bool {
    let chunk = 100u32;
    let mut remaining = ms;
    while remaining > 0 {
        if !clicking.load(Ordering::SeqCst) || epoch.load(Ordering::SeqCst) != my_epoch {
            return false;
        }
        let now = remaining.min(chunk);
        unsafe { Sleep(now); }
        remaining = remaining.saturating_sub(now);
    }
    true
}
use winapi::um::shellapi::ShellExecuteW;
use winapi::um::winuser::{
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    SetCursorPos, GetCursorPos, GetAsyncKeyState, MessageBoxW, MB_OK, MB_ICONINFORMATION, MB_ICONWARNING,
    FindWindowW, ShowWindow, SW_MINIMIZE, SW_HIDE, SW_SHOW, SW_SHOWNORMAL, GetWindowRect,
    OpenClipboard, EmptyClipboard, SetClipboardData, CloseClipboard,
};
use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalUnlock, GlobalFree};
use winapi::shared::minwindef::DWORD;
use winapi::shared::ntdef::LONG;
use winapi::shared::ntdef::HANDLE;
use winapi::shared::windef::POINT;

// 获取本地时区偏移（秒），含夏令时
fn get_tz_offset_secs() -> i64 {
    #[repr(C)]
    struct TzInfo {
        bias: LONG,
        _standard_name: [u16; 32],
        _standard_date: [u8; 16],
        standard_bias: LONG,
        _daylight_name: [u16; 32],
        _daylight_date: [u8; 16],
        daylight_bias: LONG,
    }
    let mut tz: TzInfo = unsafe { std::mem::zeroed() };
    extern "system" { fn GetTimeZoneInformation(lp: *mut TzInfo) -> DWORD; }
    let result = unsafe { GetTimeZoneInformation(&mut tz) };
    // 返回值: 1=标准时间, 2=夏令时; bias 以分钟为单位，正值 = 西时区
    let extra = match result {
        2 => tz.daylight_bias as i64,
        _ => tz.standard_bias as i64,
    };
    -(tz.bias as i64 + extra) * 60
}

/// 应用全局状态
pub struct AppState {
    pub clicking: Arc<AtomicBool>,
    pub exec_epoch: Arc<AtomicU64>,
    pub click_thread: Option<JoinHandle<()>>,
    pub interval_ms: u64,
    pub total_clicks: Arc<AtomicU64>,
    pub tasks: Vec<TaskStep>,
    pub task_loop: bool,
    pub lock_kb: bool,
    pub lock_mouse: bool,
    pub kb_hook: HHOOK,
    pub last_f6: bool,
    pub hotkey: i32,
    pub hotkey_recording: bool,
    pub tip_ticks: u32,
    pub tip_text: String,
    pub clip_active: bool,
    pub autostart: bool,
    pub auto_exec: bool,
    pub auto_exec_boot: bool,
}

impl AppState {
    pub fn new() -> Self {
        let (interval_ms, hotkey, lock_kb, lock_mouse, tasks, task_loop, _background, _rec_compress, autostart, auto_exec, auto_exec_boot) = crate::config::load_all();
        Self {
            clicking: Arc::new(AtomicBool::new(false)),
            exec_epoch: Arc::new(AtomicU64::new(0)),
            click_thread: None,
            interval_ms,
            total_clicks: Arc::new(AtomicU64::new(0)),
            tasks,
            task_loop,
            lock_kb,
            lock_mouse,
            kb_hook: null_mut(),
            last_f6: false,
            hotkey,
            hotkey_recording: false,
            tip_ticks: 0,
            tip_text: String::new(),
            clip_active: false,
            autostart,
            auto_exec,
            auto_exec_boot,
        }
    }

    /// 切换连点状态
    pub fn toggle(&mut self) {
        if self.clicking.load(Ordering::SeqCst) {
            self.stop();
        } else {
            self.start();
        }
    }

    fn stop(&mut self) {
        self.clicking.store(false, Ordering::SeqCst);
        self.exec_epoch.fetch_add(1, Ordering::SeqCst);
        // 旧线程检测到 epoch 变化后自行退出，不 join 不阻塞 UI
        let _ = self.click_thread.take();
    }

    pub fn start(&mut self) {
        if self.click_thread.is_some() { return; } // 上一轮未完全清理则忽略
        self.clicking.store(true, Ordering::SeqCst);
        self.exec_epoch.fetch_add(1, Ordering::SeqCst);

        let clicking = self.clicking.clone();
        let epoch = self.exec_epoch.clone();
        let my_epoch = self.exec_epoch.load(Ordering::SeqCst);
        let interval = self.interval_ms;
        let tasks = self.tasks.clone();
        let total = self.total_clicks.clone();
        let task_loop = self.task_loop;

        let handle = thread::spawn(move || {
            if tasks.is_empty() {
                clicking.store(false, Ordering::SeqCst);
                return;
            }
            let mut first = true;
            while first || (task_loop && epoch.load(Ordering::SeqCst) == my_epoch) {
                first = false;
                for step in &tasks {
                    if epoch.load(Ordering::SeqCst) != my_epoch { break; }
                    if !clicking.load(Ordering::SeqCst) { break; }
                    match step.action {
                        TaskActionType::MouseClick => {
                            let btn = (step.param & 0xFF) as i32;
                            let hold_ms = (step.param >> 8) as u32;
                            let jitter = step.extra as i32;
                            let (down, up) = match btn {
                                1 => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
                                2 => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
                                _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
                            };
                            if jitter > 0 {
                                let mut pt = POINT { x: 0, y: 0 };
                                unsafe { GetCursorPos(&mut pt); }
                                let ox = rand::thread_rng().gen_range(-jitter..=jitter);
                                let oy = rand::thread_rng().gen_range(-jitter..=jitter);
                                unsafe { SetCursorPos(pt.x + ox, pt.y + oy); }
                                send_mouse_input(down);
                                if !interruptible_sleep(if hold_ms > 0 { hold_ms } else { 10 }, &clicking, &epoch, my_epoch) { break; }
                                send_mouse_input(up);
                                unsafe { SetCursorPos(pt.x, pt.y); }
                            } else {
                                send_mouse_input(down);
                                if !interruptible_sleep(if hold_ms > 0 { hold_ms } else { 10 }, &clicking, &epoch, my_epoch) { break; }
                                send_mouse_input(up);
                            }
                        }
                        TaskActionType::KeyPress => {
                            let vk = (step.param & 0xFFFF) as i32;
                            let hold_ms = (step.param >> 16) as u32;
                            send_keyboard_input(vk, true);
                            let ms = if hold_ms > 0 { hold_ms } else { 10 };
                            if !interruptible_sleep(ms, &clicking, &epoch, my_epoch) { break; }
                            send_keyboard_input(vk, false);
                        }
                        TaskActionType::KeyDown => {
                            let vk = (step.param & 0xFFFF) as i32;
                            send_keyboard_input(vk, true);
                        }
                        TaskActionType::KeyUp => {
                            let vk = (step.param & 0xFFFF) as i32;
                            send_keyboard_input(vk, false);
                        }
                        TaskActionType::Delay => {
                            let ms = (step.param as u32).clamp(1, 60000);
                            if !interruptible_sleep(ms, &clicking, &epoch, my_epoch) { break; }
                        }
                        TaskActionType::MouseWheel => {
                            send_wheel_input(step.param != 0);
                            unsafe { Sleep(10) };
                        }
                        TaskActionType::MouseMove => {
                            let (x, y) = unpack_move(step.param);
                            let is_relative = (step.extra >> 24) == 1;
                            let duration = step.extra & 0xFFFFFF;
                            if x == -2 {
                                // 移动到指定窗口中心，y 是 msg_id
                                let title = crate::config::load_msg(y as u64);
                                if !title.is_empty() {
                                    move_to_window_center(&title);
                                }
                            } else if x == -1 && y == -1 {
                                let (sw, sh) = get_screen_size();
                                send_smooth_mouse_move(sw / 2, sh / 2, duration, Some(&clicking));
                            } else if is_relative {
                                let mut pt = POINT { x: 0, y: 0 };
                                unsafe { GetCursorPos(&mut pt); }
                                send_smooth_mouse_move(pt.x.wrapping_add(x), pt.y.wrapping_add(y), duration, Some(&clicking));
                            } else {
                                send_smooth_mouse_move(x, y, duration, Some(&clicking));
                            }
                        }
                        TaskActionType::MouseDown => {
                            let btn = (step.param & 0xFF) as i32;
                            let jitter = step.extra as i32;
                            let down = match btn {
                                1 => MOUSEEVENTF_RIGHTDOWN,
                                2 => MOUSEEVENTF_MIDDLEDOWN,
                                _ => MOUSEEVENTF_LEFTDOWN,
                            };
                            if jitter > 0 {
                                let mut pt = POINT { x: 0, y: 0 };
                                unsafe { GetCursorPos(&mut pt); }
                                let ox = rand::thread_rng().gen_range(-jitter..=jitter);
                                let oy = rand::thread_rng().gen_range(-jitter..=jitter);
                                unsafe { SetCursorPos(pt.x + ox, pt.y + oy); }
                                send_mouse_input(down);
                                unsafe { SetCursorPos(pt.x, pt.y); }
                            } else {
                                send_mouse_input(down);
                            }
                        }
                        TaskActionType::MouseUp => {
                            let btn = (step.param & 0xFF) as i32;
                            let jitter = step.extra as i32;
                            let up = match btn {
                                1 => MOUSEEVENTF_RIGHTUP,
                                2 => MOUSEEVENTF_MIDDLEUP,
                                _ => MOUSEEVENTF_LEFTUP,
                            };
                            if jitter > 0 {
                                let mut pt = POINT { x: 0, y: 0 };
                                unsafe { GetCursorPos(&mut pt); }
                                let ox = rand::thread_rng().gen_range(-jitter..=jitter);
                                let oy = rand::thread_rng().gen_range(-jitter..=jitter);
                                unsafe { SetCursorPos(pt.x + ox, pt.y + oy); }
                                send_mouse_input(up);
                                unsafe { SetCursorPos(pt.x, pt.y); }
                            } else {
                                send_mouse_input(up);
                            }
                        }
                        TaskActionType::ImageMatch => {
                            let (image_id, fail_msg_id) = unpack_image_match(step.param);
                            let confidence = (step.extra & 0xFF) as u8;
                            let move_mouse = (step.extra & 0x100) != 0;
                            let ignore_failure = (step.extra & 0x200) != 0;
                            let window_title_id = step.extra >> 10;
                            let threshold = if confidence == 0 { 85 } else { confidence };
                            let mut matched = false;
                            if let Some(img_data) = crate::config::load_image(image_id as u64) {
                                if let Ok(tmpl) = image::load_from_memory(&img_data) {
                                    let tmpl = tmpl.to_rgba8();
                                    let tmpl_w = tmpl.width() as i32;
                                    let tmpl_h = tmpl.height() as i32;
                                    if let Some((sw, sh, pixels)) = capture_screen() {
                                        if window_title_id != 0 {
                                            let win_title = crate::config::load_msg(window_title_id as u64);
                                            if !win_title.is_empty() {
                                                let wide: Vec<u16> = win_title.encode_utf16().chain(std::iter::once(0)).collect();
                                                unsafe {
                                                    let hwnd = FindWindowW(std::ptr::null_mut(), wide.as_ptr());
                                                    if !hwnd.is_null() {
                                                        let mut rect = std::mem::zeroed();
                                                        if GetWindowRect(hwnd, &mut rect) != 0 {
                                                            let rx = rect.left.max(0);
                                                            let ry = rect.top.max(0);
                                                            let rw = (rect.right - rect.left).min(sw) as usize;
                                                            let rh = (rect.bottom - rect.top).min(sh) as usize;
                                                            if rw > 0 && rh > 0 {
                                                                let mut cropped = vec![0u8; rw * rh * 4];
                                                                for y in 0..rh {
                                                                    let src_y = ry as usize + y;
                                                                    if src_y >= sh as usize { break; }
                                                                    for x in 0..rw {
                                                                        let src_x = rx as usize + x;
                                                                        if src_x >= sw as usize { break; }
                                                                        let si = (src_y * sw as usize + src_x) * 4;
                                                                        let di = (y * rw + x) * 4;
                                                                        cropped[di] = pixels[si];
                                                                        cropped[di+1] = pixels[si+1];
                                                                        cropped[di+2] = pixels[si+2];
                                                                        cropped[di+3] = pixels[si+3];
                                                                    }
                                                                }
                                                                if let Some((cx, cy, _conf)) = template_match(
                                                                    rw as i32, rh as i32, &cropped,
                                                                    tmpl_w, tmpl_h, &tmpl,
                                                                    threshold,
                                                                    Some(&clicking),
                                                                ) {
                                                                    matched = true;
                                                                    if move_mouse {
                                                                        SetCursorPos(rx + cx, ry + cy);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            if let Some((cx, cy, _conf)) = template_match(
                                                sw, sh, &pixels,
                                                tmpl_w, tmpl_h, &tmpl,
                                                threshold,
                                                Some(&clicking),
                                            ) {
                                                matched = true;
                                                if move_mouse {
                                                    unsafe { SetCursorPos(cx, cy); }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if !matched {
                                if fail_msg_id != 0 {
                                    let msg = crate::config::load_msg(fail_msg_id as u64);
                                    if !msg.is_empty() {
                                        let title_str = format!("{}\0", t(Lang::from_global(), "图色识别失败", "Image match failed"));
                                        let title: Vec<u16> = title_str.encode_utf16().collect();
                                        let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
                                        unsafe {
                                            MessageBoxW(
                                                std::ptr::null_mut(),
                                                wide.as_ptr(),
                                                title.as_ptr(),
                                                MB_OK | MB_ICONINFORMATION,
                                            );
                                        }
                                    }
                                }
                                if !ignore_failure {
                                    break;
                                }
                            }
                        }
                        TaskActionType::Notify => {
                            let msg_id = step.param;
                            let msg = crate::config::load_msg(msg_id);
                            if !msg.is_empty() {
                                let title_str = format!("{}\0", t(Lang::from_global(), "通知", "Notification"));
                                let title: Vec<u16> = title_str.encode_utf16().collect();
                                let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
                                unsafe {
                                    MessageBoxW(
                                        std::ptr::null_mut(),
                                        wide.as_ptr(),
                                        title.as_ptr(),
                                        MB_OK | MB_ICONINFORMATION,
                                    );
                                }
                            }
                        }
                        TaskActionType::LostFocus => {
                            let title: Vec<u16> = "酒要点点\0".encode_utf16().collect();
                            unsafe {
                                let hwnd = FindWindowW(std::ptr::null_mut(), title.as_ptr());
                                if !hwnd.is_null() {
                                    ShowWindow(hwnd, SW_MINIMIZE);
                                }
                            }
                        }
                        TaskActionType::RandomDelay => {
                            let min_ms = (step.param & 0xFFFF_FFFF) as u32;
                            let max_ms = (step.param >> 32) as u32;
                            let ms = if max_ms > min_ms {
                                rand::thread_rng().gen_range(min_ms..=max_ms)
                            } else {
                                min_ms
                            }.max(1);
                            if !interruptible_sleep(ms, &clicking, &epoch, my_epoch) { break; }
                        }
                        TaskActionType::ComboKey => {
                            let keys = crate::constants::unpack_combo(step.param, step.extra);
                            for &vk in &keys {
                                send_keyboard_input(vk as i32, true);
                                unsafe { Sleep(20) };
                            }
                            unsafe { Sleep(50) };
                            for &vk in keys.iter().rev() {
                                send_keyboard_input(vk as i32, false);
                                unsafe { Sleep(10) };
                            }
                        }
                        TaskActionType::CopyText => {
                            let msg = crate::config::load_msg(step.param);
                            if !msg.is_empty() {
                                let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
                                unsafe {
                                    if OpenClipboard(std::ptr::null_mut()) != 0 {
                                        EmptyClipboard();
                                        let bytes = (wide.len() * 2) as usize;
                                        let hmem = GlobalAlloc(0x0002 /* GMEM_MOVEABLE */, bytes);
                                        if !hmem.is_null() {
                                            let ptr = GlobalLock(hmem) as *mut u16;
                                            if !ptr.is_null() {
                                                std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
                                                GlobalUnlock(hmem);
                                                if SetClipboardData(13 /* CF_UNICODETEXT */, hmem as HANDLE).is_null() {
                                                    GlobalFree(hmem);
                                                }
                                            } else {
                                                GlobalFree(hmem);
                                            }
                                        }
                                        CloseClipboard();
                                    }
                                }
                            }
                        }
                        TaskActionType::Comment => {
                            // 备注 — 无任何操作
                        }
                        TaskActionType::ShowWindow => {
                            let title: Vec<u16> = "酒要点点\0".encode_utf16().collect();
                            unsafe {
                                let hwnd = FindWindowW(std::ptr::null_mut(), title.as_ptr());
                                if !hwnd.is_null() {
                                    ShowWindow(hwnd, SW_SHOW);
                                }
                            }
                        }
                        TaskActionType::HideWindow => {
                            let title: Vec<u16> = "酒要点点\0".encode_utf16().collect();
                            unsafe {
                                let hwnd = FindWindowW(std::ptr::null_mut(), title.as_ptr());
                                if !hwnd.is_null() {
                                    ShowWindow(hwnd, SW_HIDE);
                                }
                            }
                        }
                        TaskActionType::OpenProgram => {
                            let path = crate::config::load_msg(step.param);
                            if !path.is_empty() {
                                let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
                                unsafe {
                                    ShellExecuteW(
                                        std::ptr::null_mut(),
                                        std::ptr::null(),
                                        wide.as_ptr(),
                                        std::ptr::null(),
                                        std::ptr::null(),
                                        SW_SHOWNORMAL,
                                    );
                                }
                            }
                        }
                        TaskActionType::WaitUntil => {
                            let total_min = step.param;
                            let target_h = total_min / 60;
                            let target_m = total_min % 60;
                            let tz_offset = get_tz_offset_secs();
                            loop {
                                if !clicking.load(Ordering::SeqCst) || epoch.load(Ordering::SeqCst) != my_epoch { break; }
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default();
                                let secs = now.as_secs();
                                let local_secs = (secs as i64 + tz_offset) as u64;
                                let current_min = (local_secs / 60) % (24 * 60);
                                let current_h = current_min / 60;
                                let current_m = current_min % 60;
                                if current_h > target_h || (current_h == target_h && current_m >= target_m) {
                                    break;
                                }
                                if !interruptible_sleep(1000, &clicking, &epoch, my_epoch) { break; }
                            }
                        }
                        TaskActionType::WaitInput => {
                            let expected = crate::config::load_msg(step.param);
                            let copy_input = crate::constants::waitinput_copy(step.extra);
                            let ignore_fail = crate::constants::waitinput_ignore_fail(step.extra);
                            let use_regex = crate::constants::waitinput_regex(step.extra);
                            // 使用 PowerShell InputBox 弹窗获取用户输入
                            let ps_script = format!(
                                "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.Interaction]::InputBox('{}', '酒要点点', '')",
                                t(Lang::from_global(), "请输入匹配文本", "Enter matching text")
                            );
                            let output = std::process::Command::new("powershell")
                                .args(["-NoProfile", "-Command", ps_script.as_str()])
                                .output();
                            if let Ok(out) = output {
                                let input = String::from_utf8_lossy(&out.stdout).trim().to_string();
                                if copy_input && !input.is_empty() {
                                    // 复制用户输入到剪贴板
                                    let wide: Vec<u16> = input.encode_utf16().chain(std::iter::once(0)).collect();
                                    unsafe {
                                        if OpenClipboard(std::ptr::null_mut()) != 0 {
                                            EmptyClipboard();
                                            let bytes = (wide.len() * 2) as usize;
                                            let hmem = GlobalAlloc(0x0002, bytes);
                                            if !hmem.is_null() {
                                                let ptr = GlobalLock(hmem) as *mut u16;
                                                if !ptr.is_null() {
                                                    std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
                                                    GlobalUnlock(hmem);
                                                    if SetClipboardData(13, hmem as HANDLE).is_null() {
                                                        GlobalFree(hmem);
                                                    }
                                                } else {
                                                    GlobalFree(hmem);
                                                }
                                            }
                                            CloseClipboard();
                                        }
                                    }
                                }
                                let match_ok = if use_regex {
                                    regex::Regex::new(&expected).map(|re| re.is_match(&input)).unwrap_or(false)
                                } else {
                                    input == expected
                                };
                                if !match_ok && !ignore_fail {
                                    clicking.store(false, Ordering::SeqCst);
                                    break;
                                }
                            } else {
                                if !ignore_fail {
                                    clicking.store(false, Ordering::SeqCst);
                                    break;
                                }
                            }
                        }
                        TaskActionType::WaitKey => {
                            let target_vk = step.param as i32;
                            let terminate = crate::constants::waitkey_terminate_on_wrong(step.extra);
                            let show_popup = crate::constants::waitkey_show_popup(step.extra);
                            if show_popup {
                                let lang = Lang::from_global();
                                let key_name = crate::app::util::vk_name(target_vk, lang);
                                let info = tf(lang,
                                    "等待按下 {key} 继续...",
                                    "Press {key} to continue...");
                                let info = info.replace("{key}", &key_name);
                                let title: Vec<u16> = "酒要点点\0".encode_utf16().collect();
                                let body: Vec<u16> = info.encode_utf16().chain(std::iter::once(0)).collect();
                                std::thread::spawn(move || unsafe {
                                    MessageBoxW(std::ptr::null_mut(), body.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION);
                                });
                            }
                            // 等待目标键松开再按下（防穿透）
                            let mut was_down = false;
                            loop {
                                if !clicking.load(Ordering::SeqCst) || epoch.load(Ordering::SeqCst) != my_epoch { break; }
                                let state = unsafe { GetAsyncKeyState(target_vk) } as u16;
                            let is_down = (state & 0x8000) != 0;
                                if was_down && !is_down {
                                    break;
                                }
                                if terminate {
                                    let mut wrong = false;
                                    for test_vk in 0x01..=0xFF {
                                        if test_vk == target_vk { continue; }
                                        let s = unsafe { GetAsyncKeyState(test_vk) } as u16;
                                        if (s & 0x8000) != 0
                                            && test_vk >= 0x08
                                            && test_vk != 0x10 && test_vk != 0x11 && test_vk != 0x12
                                            && test_vk != 0x5B && test_vk != 0x5C
                                            && test_vk != 0xA0 && test_vk != 0xA1 && test_vk != 0xA2
                                            && test_vk != 0xA3 && test_vk != 0xA4 && test_vk != 0xA5
                                        {
                                            wrong = true;
                                            break;
                                        }
                                    }
                                    if wrong {
                                        if show_popup {
                                            let lang2 = Lang::from_global();
                                            let msg = t(lang2,
                                                "按下了错误的按键，任务终止",
                                                "Wrong key pressed, task stopped");
                                            let title: Vec<u16> = "酒要点点\0".encode_utf16().collect();
                                            let body: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
                                            unsafe {
                                                MessageBoxW(std::ptr::null_mut(), body.as_ptr(), title.as_ptr(), MB_OK | MB_ICONWARNING);
                                            }
                                        }
                                        clicking.store(false, Ordering::SeqCst);
                                        break;
                                    }
                                }
                                was_down = is_down;
                                if !interruptible_sleep(50, &clicking, &epoch, my_epoch) { break; }
                            }
                            if !clicking.load(Ordering::SeqCst) { break; }
                        }
                    }
                }
                let mut cur = total.load(Ordering::SeqCst);
                while cur < u64::MAX {
                    match total.compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst) {
                        Ok(_) => break,
                        Err(v) => cur = v,
                    }
                }
                if !task_loop { break; }
                if clicking.load(Ordering::SeqCst) && interval > 0
                    && !interruptible_sleep(interval as u32, &clicking, &epoch, my_epoch) { break; }
            }
            clicking.store(false, Ordering::SeqCst);
        });
        self.click_thread = Some(handle);
    }
}
