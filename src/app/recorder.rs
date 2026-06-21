//! 录制引擎：鼠标钩子回调、键盘轮询、事件处理、移动压缩

use std::ptr::{null, null_mut};
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Instant;

use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::{
    CallNextHookEx, GetAsyncKeyState, GetCursorPos,
    SetWindowsHookExW, UnhookWindowsHookEx,
    WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_RBUTTONDOWN, WM_MBUTTONDOWN, WM_XBUTTONDOWN, WM_MOUSEWHEEL,
};

use crate::app::util::is_mouse_vk;
use crate::constants::*;

/// 长按判定阈值（毫秒）
const LONG_PRESS_MS: u64 = 300;

// ── 录制事件类型 ──────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum RecEvent {
    MouseDown { button: i32, x: i32, y: i32 },
    MouseMove { dx: i32, dy: i32 },
    Wheel { up: bool },
}

pub static REC_TX: Mutex<Option<mpsc::Sender<RecEvent>>> = Mutex::new(None);
pub static REC_LAST_POS: Mutex<(i32, i32)> = Mutex::new((0, 0));
pub static REC_LAST_MOVE: Mutex<(i32, i32)> = Mutex::new((-9999, -9999));

#[repr(C)]
#[allow(non_snake_case)]
struct Msllhookstruct {
    pt: winapi::shared::windef::POINT,
    mouseData: u32,
    flags: u32,
    time: u32,
    dwExtraInfo: usize,
}

pub unsafe extern "system" fn rec_mouse_hook(code: i32, wparam: usize, lparam: isize) -> isize {
    if code >= 0 {
        let ms = &*(lparam as *const Msllhookstruct);
        const LLMHF_INJECTED: u32 = 0x1;
        if (ms.flags & LLMHF_INJECTED) == 0 {
            match wparam as u32 {
                WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN => {
                    let button = match wparam as u32 {
                        WM_LBUTTONDOWN => 0,
                        WM_RBUTTONDOWN => 1,
                        WM_MBUTTONDOWN => 2,
                        WM_XBUTTONDOWN => {
                            let xbtn = (ms.mouseData >> 16) as u16;
                            if xbtn == 1 { 3 } else { 4 }
                        }
                        _ => unreachable!(),
                    };
                    let x = ms.pt.x;
                    let y = ms.pt.y;
                    if let Ok(g) = REC_TX.lock() {
                        if let Some(ref tx) = *g {
                            let _ = tx.send(RecEvent::MouseDown { button, x, y });
                        }
                    }
                    if let Ok(mut pos) = REC_LAST_POS.lock() { *pos = (x, y); }
                }
                WM_MOUSEWHEEL => {
                    let delta = (ms.mouseData >> 16) as i16;
                    if delta != 0 {
                        if let Ok(g) = REC_TX.lock() {
                            if let Some(ref tx) = *g {
                                let _ = tx.send(RecEvent::Wheel { up: delta > 0 });
                            }
                        }
                    }
                }
                _ => {
                    let (mx, my) = (ms.pt.x, ms.pt.y);
                    if let Ok(mut pos) = REC_LAST_POS.lock() { *pos = (mx, my); }
                    if let Ok(mut last) = REC_LAST_MOVE.lock() {
                        let dx = mx.wrapping_sub(last.0);
                        let dy = my.wrapping_sub(last.1);
                        if dx * dx + dy * dy > 25 {
                            *last = (mx, my);
                            drop(last);
                            if let Ok(g) = REC_TX.lock() {
                                if let Some(ref tx) = *g {
                                    let _ = tx.send(RecEvent::MouseMove { dx, dy });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

// ── 键盘状态初始化 ────────────────────────────────────────

pub fn rec_prev_keys_init(prev: &mut [u8; 256]) {
    for vk in 0..256i32 {
        if unsafe { (GetAsyncKeyState(vk) & 0x8000u16 as i16) != 0 } {
            prev[vk as usize] = 0x80;
        }
    }
}

// ── BeerClickerApp 录制方法 ───────────────────────────────

use super::BeerClickerApp;

impl BeerClickerApp {
    pub fn start_recording(&mut self) {
        self.recording = true;
        self.rec_last_time = Instant::now();
        let (tx, rx) = mpsc::channel();
        self.rec_rx = Some(rx);
        if let Ok(mut g) = REC_TX.lock() { *g = Some(tx); }

        let mut pt: winapi::shared::windef::POINT = unsafe { std::mem::zeroed() };
        unsafe { GetCursorPos(&mut pt); }
        if let Ok(mut pos) = REC_LAST_POS.lock() { *pos = (pt.x, pt.y); }
        if let Ok(mut last) = REC_LAST_MOVE.lock() { *last = (pt.x, pt.y); }
        self.rec_last_pos = (pt.x, pt.y);
        self.state.tasks.push(TaskStep {
            action: TaskActionType::MouseMove,
            param: pack_move(pt.x, pt.y),
            extra: 0,
        });
        crate::config::save_tasks(&self.state.tasks);

        let hinst = unsafe { GetModuleHandleW(null()) };
        self.rec_mouse_hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(rec_mouse_hook), hinst, 0) };
        rec_prev_keys_init(&mut self.rec_prev_keys);
    }

    pub fn stop_recording(&mut self) {
        self.recording = false;
        if !self.rec_mouse_hook.is_null() {
            unsafe { UnhookWindowsHookEx(self.rec_mouse_hook); }
            self.rec_mouse_hook = null_mut();
        }
        if let Ok(mut g) = REC_TX.lock() { *g = None; }
        self.rec_rx = None;

        if self.rec_compress {
            self.compress_moves();
        }
    }

    pub fn process_recorded_events(&mut self) {
        if let Some(ref rx) = self.rec_rx {
            let hotkey = self.state.hotkey;
            let mut dirty = false;
            while let Ok(ev) = rx.try_recv() {
                dirty = true;
                let elapsed = self.rec_last_time.elapsed().as_millis() as u64;
                self.rec_last_time = Instant::now();
                match ev {
                    RecEvent::MouseDown { button, x, y } => {
                        if (x, y) != self.rec_last_pos {
                            let dx = x - self.rec_last_pos.0;
                            let dy = y - self.rec_last_pos.1;
                            if elapsed >= 30 {
                                self.state.tasks.push(TaskStep {
                                    action: TaskActionType::MouseMove,
                                    param: pack_move(dx, dy),
                                    extra: elapsed.min(60000) as u32 | RELATIVE_FLAG,
                                });
                            }
                            self.rec_last_pos = (x, y);
                        } else if elapsed >= 30 {
                            self.state.tasks.push(TaskStep {
                                action: TaskActionType::Delay,
                                param: elapsed.min(60000),
                                extra: 0,
                            });
                        }
                        self.state.tasks.push(TaskStep {
                            action: TaskActionType::MouseClick,
                            param: button as u64 & 0xFF,
                            extra: 0,
                        });
                    }
                    RecEvent::MouseMove { dx, dy } => {
                        self.rec_last_pos = (self.rec_last_pos.0.wrapping_add(dx), self.rec_last_pos.1.wrapping_add(dy));
                        self.state.tasks.push(TaskStep {
                            action: TaskActionType::MouseMove,
                            param: pack_move(dx, dy),
                            extra: elapsed.min(60000) as u32 | RELATIVE_FLAG,
                        });
                    }
                    RecEvent::Wheel { up } => {
                        if elapsed >= 30 {
                            self.state.tasks.push(TaskStep {
                                action: TaskActionType::Delay,
                                param: elapsed.min(60000),
                                extra: 0,
                            });
                        }
                        self.state.tasks.push(TaskStep {
                            action: TaskActionType::MouseWheel,
                            param: if up { 1 } else { 0 },
                            extra: 0,
                        });
                    }
                }
            }

            fn is_down(vk: i32) -> bool {
                unsafe { (GetAsyncKeyState(vk) & 0x8000u16 as i16) != 0 }
            }

            let ctrl = is_down(0xA2) || is_down(0xA3) || is_down(0x11);
            let shift = is_down(0xA0) || is_down(0xA1) || is_down(0x10);
            let alt = is_down(0xA4) || is_down(0xA5) || is_down(0x12);
            let win = is_down(0x5B) || is_down(0x5C);
            let has_mod = ctrl || shift || alt || win;

            let prev_elapsed = self.rec_last_time.elapsed().as_millis() as u64;
            for vk in 0..256i32 {
                let was_pressed = (self.rec_prev_keys[vk as usize] & 0x80) != 0;
                let is_pressed = is_down(vk);

                if !was_pressed && is_pressed {
                    self.rec_key_down[vk as usize] = Some(Instant::now());

                    if is_mouse_vk(vk) || vk == hotkey {
                    } else {
                        if prev_elapsed >= 30 {
                            self.state.tasks.push(TaskStep {
                                action: TaskActionType::Delay,
                                param: prev_elapsed.min(60000),
                                extra: 0,
                            });
                        }
                        self.rec_last_time = Instant::now();

                        if has_mod && vk < 256 {
                            let mut combo: Vec<u8> = Vec::with_capacity(4);
                            if ctrl { combo.push(0xA2); }
                            if shift { combo.push(0xA0); }
                            if alt { combo.push(0xA4); }
                            if win { combo.push(0x5B); }
                            combo.push(vk as u8);
                            self.state.tasks.push(TaskStep {
                                action: TaskActionType::ComboKey,
                                param: pack_combo(&combo),
                                extra: combo.len() as u32,
                            });
                        } else {
                            self.state.tasks.push(TaskStep {
                                action: TaskActionType::KeyPress,
                                param: vk as u64 & 0xFFFF,
                                extra: 0,
                            });
                        }
                        dirty = true;
                    }
                }

                if was_pressed && !is_pressed {
                    if is_mouse_vk(vk) || vk == hotkey { }
                    else {
                        let hold_ms = self.rec_key_down[vk as usize].map_or(0, |t| t.elapsed().as_millis() as u64);
                        if hold_ms >= LONG_PRESS_MS {
                            if let Some(last) = self.state.tasks.last_mut() {
                                match last.action {
                                    TaskActionType::KeyPress if (last.param & 0xFFFF) == vk as u64 => {
                                        last.param |= (hold_ms & 0xFFFF) << 16;
                                    }
                                    TaskActionType::ComboKey => {
                                        // 从任务数据中解包组合键（而非当前修饰键状态）
                                        let combo_keys = self.state.tasks.pop().map(|t| crate::constants::unpack_combo(t.param, t.extra)).unwrap_or_default();
                                        // 按下修饰键和主键
                                        for &vk in &combo_keys {
                                            self.state.tasks.push(TaskStep { action: TaskActionType::KeyDown, param: vk as u64, extra: 0 });
                                        }
                                        // 延迟
                                        self.state.tasks.push(TaskStep { action: TaskActionType::Delay, param: hold_ms.min(60000), extra: 0 });
                                        // 释放主键和修饰键（逆序）
                                        let main_vk = combo_keys.last().copied().unwrap_or(vk as u8) as u64;
                                        self.state.tasks.push(TaskStep { action: TaskActionType::KeyUp, param: main_vk, extra: 0 });
                                        for &mk in combo_keys.iter().rev().skip(1) {
                                            self.state.tasks.push(TaskStep { action: TaskActionType::KeyUp, param: mk as u64, extra: 0 });
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    self.rec_key_down[vk as usize] = None;
                }

                if is_pressed {
                    self.rec_prev_keys[vk as usize] |= 0x80;
                } else {
                    self.rec_prev_keys[vk as usize] &= !0x80;
                }
            }
            if dirty {
                crate::config::save_tasks(&self.state.tasks);
            }
        }
    }

    /// 合并连续 MouseMove：dx/dy 累加，duration 取最大者
    fn compress_moves(&mut self) {
        let tasks = &mut self.state.tasks;
        let mut i = 0;
        while i + 1 < tasks.len() {
            if tasks[i].action == TaskActionType::MouseMove
                && tasks[i + 1].action == TaskActionType::MouseMove
            {
                // 不合并绝对/相对混合移动
                let rel1 = tasks[i].extra & RELATIVE_FLAG;
                let rel2 = tasks[i + 1].extra & RELATIVE_FLAG;
                if rel1 != rel2 { i += 1; continue; }
                let (dx, dy) = unpack_move(tasks[i].param);
                let (dx2, dy2) = unpack_move(tasks[i + 1].param);
                let dur1 = tasks[i].extra as u64 & !(RELATIVE_FLAG as u64);
                let dur2 = tasks[i + 1].extra as u64 & !(RELATIVE_FLAG as u64);
                tasks[i].param = pack_move(dx.wrapping_add(dx2), dy.wrapping_add(dy2));
                tasks[i].extra = (dur1.max(dur2) as u32) | rel1;
                tasks.remove(i + 1);
            } else {
                i += 1;
            }
        }
        crate::config::save_tasks(&self.state.tasks);
    }
}
