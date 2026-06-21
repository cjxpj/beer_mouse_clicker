//! 酒要点点 — 主应用框架
//!
//! 此文件保留：
//! - BeerClickerApp 结构体定义与初始化
//! - eframe::App 的 update 循环
//! - 热键轮询 / 键盘锁 / 任务清理
//! - 面板渲染：header / 控制栏 / 任务列表 / footer / 关于弹窗
//! - 拖拽排序 / 删除动画
//!
//! 子模块：
//! - theme      颜色常量 + 主题 + 字体
//! - util       工具函数（vk_name / task_desc / selectable_btn 等）
//! - recorder   录制引擎（钩子 / 键盘轮询 / 事件处理）
//! - edit_popup 编辑弹窗
//! - add_popup  添加弹窗

mod edit_popup;
mod add_popup;
mod recorder;
mod theme;
pub mod util;

use crate::constants::*;
use crate::lang::{t, Lang};
use crate::state::AppState;

use std::ptr::{null, null_mut};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Instant;

use egui::{Color32, Frame, CornerRadius, Stroke, Vec2, Align2, FontId};
use image::ImageEncoder;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::{
    ClipCursor, GetAsyncKeyState, GetCursorPos,
    SetWindowsHookExW, UnhookWindowsHookEx,
    CallNextHookEx,
    WH_KEYBOARD_LL,
    MessageBoxW, MB_OK, MB_ICONERROR,
};

use edit_popup::EditState;
use add_popup::AddState;
use recorder::RecEvent;
use theme::*;
use util::*;

// ── 应用结构体 ────────────────────────────────────────────

/// 文件选择类型，区分图片和可执行程序
#[derive(Clone, Copy, PartialEq)]
enum FilePickKind {
    Image,
    Exe,
}

pub struct BeerClickerApp {
    state: AppState,
    selected_task: Option<usize>,
    edit_index: Option<usize>,
    edit_state: EditState,
    add_state: AddState,
    drag_idx: Option<usize>,
    drag_start_y: f32,
    drag_accum: f32,
    drag_has_moved: bool,
    was_primary_down: bool,
    hovered_task: Option<usize>,
    first_frame: bool,
    background: bool,
    show_about: bool,
    swap_offsets: Vec<(usize, f32)>,
    delete_animations: Vec<(usize, f32)>, // (任务索引, 进度 0→1)
    /// 识图缩略图缓存 image_id → PNG 字节数据
    thumbnail_cache: std::collections::HashMap<u64, Option<Vec<u8>>>,
    show_coords: bool,
    /// 延迟文件选择：标记触发后在 update 开头调用文件对话框
    pending_file_pick: Option<FilePickKind>,
    picked_path: Option<String>,
    /// 放大预览时另存为：数据 + 触发保存对话框标志
    pending_save_data: Option<Vec<u8>>,
    pending_save_open: bool,
    save_target: Option<String>,
    /// 录制
    recording: bool,
    rec_armed: bool,
    rec_compress: bool,
    lang: Lang,
    rec_mouse_hook: winapi::shared::windef::HHOOK,
    rec_rx: Option<mpsc::Receiver<RecEvent>>,
    rec_last_time: Instant,
    rec_last_pos: (i32, i32),
    rec_prev_keys: [u8; 256],
    /// 每个按键的按下时刻（用于长按判定）
    rec_key_down: [Option<Instant>; 256],
}

impl BeerClickerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_chinese_font(&cc.egui_ctx);
        setup_theme(&cc.egui_ctx);
        let state = AppState::new();
        // 同步注册表（用户在外部删除了注册表项时恢复）
        crate::config::set_autostart_registry(state.autostart);
        Self {
            state,
            selected_task: None,
            edit_index: None,
            edit_state: EditState::None,
            add_state: AddState::None,
            drag_idx: None,
            drag_start_y: 0.0,
            drag_accum: 0.0,
            drag_has_moved: false,
            was_primary_down: false,
            hovered_task: None,
            first_frame: true,
            background: crate::config::load_background(),
            show_about: false,
            swap_offsets: Vec::new(),
            delete_animations: Vec::new(),
            thumbnail_cache: std::collections::HashMap::new(),
            show_coords: false,
            pending_file_pick: None,
            picked_path: None,
            pending_save_data: None,
            pending_save_open: false,
            save_target: None,
            recording: false,
            rec_armed: false,
            rec_compress: crate::config::load_rec_compress(),
            lang: {
                let saved = crate::config::load_lang();
                let l = match saved.as_str() {
                    "en" => Lang::EN,
                    "cn" => Lang::CN,
                    _ => Lang::detect_system(),
                };
                l.save_to_global();
                l
            },
            rec_mouse_hook: null_mut(),
            rec_rx: None,
            rec_last_time: Instant::now(),
            rec_last_pos: (0, 0),
            rec_prev_keys: [0u8; 256],
            rec_key_down: [None; 256],
        }
    }

    fn poll_hotkey(&mut self, ctx: &egui::Context) {
        if self.state.hotkey_recording {
            return;
        }
        if !is_mouse_vk(self.state.hotkey) {
            let key_down = unsafe { (GetAsyncKeyState(self.state.hotkey) & 0x8000u16 as i16) != 0 };
            if key_down && !self.state.last_f6 {
                // 待命中按热键：开始录制
                if self.rec_armed {
                    self.rec_armed = false;
                    self.start_recording();
                    self.state.last_f6 = key_down;
                    return;
                }
                // 录制中按热键：停止录制
                if self.recording {
                    self.stop_recording();
                    self.state.last_f6 = key_down;
                    return;
                }
                // 后台模式：热键先唤出窗口，再切换
                if self.background {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                }
                self.state.toggle();
                if self.state.clicking.load(Ordering::SeqCst) && self.recording {
                    self.stop_recording();
                }
            }
            self.state.last_f6 = key_down;
        }
    }

    fn update_locks(&mut self) {
        let clicking = self.state.clicking.load(Ordering::SeqCst);

        // 键盘钩子：锁键盘时安装
        if self.state.lock_kb && clicking {
            if self.state.kb_hook.is_null() {
                let hinst = unsafe { GetModuleHandleW(null()) };
                self.state.kb_hook = unsafe {
                    SetWindowsHookExW(
                        WH_KEYBOARD_LL,
                        Some(kb_hook_proc),
                        hinst,
                        0,
                    )
                };
            }
        } else {
            if !self.state.kb_hook.is_null() {
                unsafe { UnhookWindowsHookEx(self.state.kb_hook); }
                self.state.kb_hook = null_mut();
            }
        }

        // 鼠标锁：录制中不锁定，否则无法正常录制鼠标操作
        if self.state.lock_mouse && clicking && !self.recording {
            let mut pt: winapi::shared::windef::POINT = unsafe { std::mem::zeroed() };
            unsafe { GetCursorPos(&mut pt) };
            let rect = winapi::shared::windef::RECT {
                left: pt.x,
                top: pt.y,
                right: pt.x + 1,
                bottom: pt.y + 1,
            };
            unsafe { ClipCursor(&rect) };
            self.state.clip_active = true;
        } else if self.state.clip_active {
            unsafe { ClipCursor(null_mut()) };
            self.state.clip_active = false;
        }
    }

    /// 清理任务关联的数据库记录（图片、消息）
    fn cleanup_task_db(task: &TaskStep) {
        match task.action {
            TaskActionType::ImageMatch => {
                let (image_id, fail_msg_id) = unpack_image_match(task.param);
                crate::config::delete_image(image_id as u64);
                if fail_msg_id != 0 { crate::config::delete_msg(fail_msg_id as u64); }
                let window_title_id = task.extra >> 10;
                if window_title_id != 0 { crate::config::delete_msg(window_title_id as u64); }
            }
            TaskActionType::Notify => {
                if task.param != 0 { crate::config::delete_msg(task.param); }
            }
            TaskActionType::CopyText => {
                if task.param != 0 { crate::config::delete_msg(task.param); }
            }
            TaskActionType::Comment => {
                if task.param != 0 { crate::config::delete_msg(task.param); }
            }
            TaskActionType::OpenProgram => {
                if task.param != 0 { crate::config::delete_msg(task.param); }
            }
            TaskActionType::MouseMove => {
                let (x, _y) = unpack_move(task.param);
                // 窗口居中：y 的高位存储 msg_id
                if x == -2 {
                    let msg_id = (task.param >> 32) as u32;
                    if msg_id != 0 { crate::config::delete_msg(msg_id as u64); }
                }
            }
            _ => {}
        }
    }

    fn render_task_items(&mut self, ui: &mut egui::Ui, row_range: std::ops::Range<usize>) {
        let mut edit_idx: Option<usize> = None;
        let mut swap_action: Option<(usize, usize)> = None;
        let row_h = 40.0;

        // 手动拖拽：用原始输入检测按下，帧间位移计算交换
        let pointer = ui.ctx().input(|inp| inp.pointer.interact_pos());
        let primary_down = ui.ctx().input(|inp| inp.pointer.button_down(egui::PointerButton::Primary));
        let primary_just_pressed = primary_down && !self.was_primary_down;
        self.was_primary_down = primary_down;
        if !primary_down && self.drag_idx.is_some() {
            self.drag_idx = None;
            self.drag_accum = 0.0;
            self.drag_has_moved = false;
        }

        // 衰减换位动画偏移（柔和指数衰减）
        self.swap_offsets.iter_mut().for_each(|(_, o)| *o *= 0.92);
        self.swap_offsets.retain(|(_, o)| o.abs() > 0.2);

        // 删除动画推进（progress 0→1，约 0.33s 完成）
        let anim_speed = 0.05;
        let mut completed: Vec<usize> = Vec::new();
        for (idx, prog) in self.delete_animations.iter_mut() {
            *prog += anim_speed;
            if *prog >= 1.0 {
                completed.push(*idx);
            }
        }
        // 从大到小排序并移除，避免索引偏移
        completed.sort_unstable_by(|a, b| b.cmp(a));
        for ci in completed {
            Self::cleanup_task_db(&self.state.tasks[ci]);
            self.state.tasks.remove(ci);
            crate::config::save_tasks(&self.state.tasks);
            if self.selected_task == Some(ci) {
                self.selected_task = None;
            } else if let Some(ref mut s) = self.selected_task {
                if *s > ci { *s -= 1; }
            }
            // 移除对应动画条目 + 修正其他动画索引
            self.delete_animations.retain(|(idx, _)| *idx != ci);
            for (idx, _) in self.delete_animations.iter_mut() {
                if *idx > ci { *idx -= 1; }
            }
            if let Some(di) = self.drag_idx {
                if di == ci { self.drag_idx = None; }
                else if di > ci { self.drag_idx = Some(di - 1); }
            }
            // 修正 swap_offsets 中的索引
            self.swap_offsets.iter_mut().for_each(|(idx, _)| {
                if *idx > ci { *idx -= 1; }
            });
            self.swap_offsets.retain(|(idx, _)| *idx != ci);
            // 修正 hovered_task
            if self.hovered_task == Some(ci) { self.hovered_task = None; }
            else if let Some(ref mut h) = self.hovered_task { if *h > ci { *h -= 1; } }
        }

        // 预加载识图缩略图
        for step in &self.state.tasks {
            if let TaskActionType::ImageMatch = step.action {
                let (image_id, _) = unpack_image_match(step.param);
                if image_id != 0 {
                    self.thumbnail_cache.entry(image_id as u64).or_insert_with(|| {
                        crate::config::load_image(image_id as u64)
                            .and_then(|raw| {
                                image::load_from_memory(&raw).ok()
                                    .and_then(|img| {
                                        let thumb = img.thumbnail(40, 40).to_rgba8();
                                        let mut buf = Vec::new();
                                        image::codecs::png::PngEncoder::new(&mut buf)
                                            .write_image(thumb.as_raw(), thumb.width(), thumb.height(), image::ExtendedColorType::Rgba8)
                                            .ok().map(|_| buf)
                                    })
                            })
                    });
                }
            }
        }
        let n = self.state.tasks.len();

        // 预计算备注覆盖范围：comment_coverage[i] = Some((comment_idx, 剩余行数))
        let mut comment_coverage: Vec<Option<(usize, usize)>> = vec![None; n];
        {
            let mut cover_remaining: usize = 0;
            let mut cover_comment_idx: usize = 0;
            for (idx, cov) in comment_coverage.iter_mut().enumerate().take(n) {
                let step = &self.state.tasks[idx];
                if step.action == TaskActionType::Comment && step.extra > 0 {
                    cover_remaining = step.extra as usize;
                    cover_comment_idx = idx;
                    // 备注自身是组头，有连接器但不消耗覆盖数
                    *cov = Some((idx, cover_remaining + 1));
                } else if cover_remaining > 0 {
                    *cov = Some((cover_comment_idx, cover_remaining));
                    cover_remaining -= 1;
                }
            }
            // 修复：覆盖范围超过列表末尾时，最末被覆盖行标记为末行（圆形底角）
            if cover_remaining > 0 && n > 0 {
                let last_covered = comment_coverage.iter()
                    .rposition(|c| c.map(|(_, rem)| rem > 1).unwrap_or(false));
                if let Some(li) = last_covered {
                    comment_coverage[li] = Some((cover_comment_idx, 1));
                }
            }
        }

        let end = row_range.end.min(n);
        for i in row_range.start..end {
            // 备注覆盖信息
            let cover_info = comment_coverage.get(i).and_then(|c| *c);
            let is_comment_first = cover_info.map(|(ci, _rem)| ci == i).unwrap_or(false);
            let is_comment_covered = cover_info.map(|(ci, _)| ci != i).unwrap_or(false);
            let is_comment_last = cover_info.map(|(_, rem)| rem == 1).unwrap_or(false);
            // 当前卡片若有动画偏移，加在前头（弹性位移）
            let anim_offset = self.swap_offsets.iter()
                .find(|(idx, _)| *idx == i)
                .map(|(_, o)| *o)
                .unwrap_or(0.0);
            if anim_offset > 0.0 {
                ui.add_space(anim_offset);
            }
            let (color, desc) = {
                let step = &self.state.tasks[i];
                let color = match step.action {
                    TaskActionType::MouseClick => C_BLUE,
                    TaskActionType::Delay => C_PEACH,
                    TaskActionType::KeyPress => C_GREEN2,
                    TaskActionType::MouseWheel => C_MAUVE,
                    TaskActionType::MouseMove => C_TEAL,
                    TaskActionType::MouseDown => C_YELLOW,
                    TaskActionType::MouseUp => C_RED,
                    TaskActionType::KeyDown => C_GREEN,
                    TaskActionType::KeyUp => C_GREEN2,
                    TaskActionType::ImageMatch => C_PINK,
                    TaskActionType::Notify => C_WHITE_SOFT,
                    TaskActionType::LostFocus => C_LAVENDER,
                    TaskActionType::RandomDelay => C_ORANGE,
                    TaskActionType::ComboKey => C_CYAN,
                    TaskActionType::WaitUntil => C_ORANGE,
                    TaskActionType::CopyText => C_LAVENDER,
                    TaskActionType::Comment => C_SUBTEXT,
                    TaskActionType::WaitKey => C_LAVENDER,
                    TaskActionType::WaitInput => C_LAVENDER,
                    TaskActionType::ShowWindow => C_GREEN2,
                    TaskActionType::HideWindow => C_ORANGE,
                    TaskActionType::OpenProgram => C_TEAL,
                };
                (color, task_desc_egui(step, self.lang))
            };
            let is_selected = self.selected_task == Some(i);
            let is_dragging = self.drag_idx == Some(i);
            let is_hovered = self.hovered_task == Some(i) && !is_dragging && !is_selected;
            let is_deleting = self.delete_animations.iter().find(|(idx, _)| *idx == i);
            let delete_shrink = is_deleting.map(|(_, p)| *p).unwrap_or(0.0);

            let frame = Frame::NONE
                .fill(if is_dragging || is_selected {
                    C_SURFACE1
                } else if is_hovered {
                    C_HOVER
                } else {
                    C_SURFACE0
                })
                .corner_radius(CornerRadius::same(6))
                .shadow(egui::Shadow::NONE)
                .inner_margin(4.0);

            // 删除动画：向右收缩
            let shrink_factor = delete_shrink.powf(1.3);
            let spacer_w = ui.available_size_before_wrap().x * shrink_factor;

            let mut response: Option<egui::Response> = None;
            ui.horizontal(|ui| {
                if spacer_w > 0.0 {
                    ui.allocate_space(egui::vec2(spacer_w, 0.0));
                }
                response = Some(frame.show(ui, |ui| {
                    ui.set_min_height(32.0);
                    ui.horizontal(|ui| {
                        // 备注覆盖连接线
                        if is_comment_covered || is_comment_first {
                            let conn_w = 4.0;
                            let conn_color = dim_color_egui(C_CYAN);
                            let (conn_rect, _) = ui.allocate_exact_size(
                                Vec2::new(conn_w, 32.0),
                                egui::Sense::hover(),
                            );
                            let corner_radius = if is_comment_covered && !is_comment_first && !is_comment_last {
                                CornerRadius::ZERO // 中间行
                            } else {
                                CornerRadius::same(2) // 首行/末行/单行
                            };
                            ui.painter().rect_filled(conn_rect, corner_radius, conn_color);
                            ui.add_space(2.0);
                        }
                        // 序号
                        let num_size = Vec2::new(22.0, 32.0);
                        let (num_rect, _) = ui.allocate_exact_size(num_size, egui::Sense::hover());
                        ui.painter().text(
                            num_rect.center(),
                            Align2::CENTER_CENTER,
                            format!("{}", i + 1),
                            FontId::proportional(11.0),
                            C_SUBTEXT,
                        );

                        // 左侧色条
                        let bar_w = 5.0;
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(bar_w, 32.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, CornerRadius::same(2),
                            if is_hovered { brighten_color(color, 40) } else { color });

                        ui.add_space(8.0);

                        // 识图缩略图
                        if let TaskActionType::ImageMatch = self.state.tasks[i].action {
                            let (image_id, _) = unpack_image_match(self.state.tasks[i].param);
                            if image_id != 0 {
                                if let Some(Some(ref png_data)) = self.thumbnail_cache.get(&(image_id as u64)) {
                                    let uri = format!("bytes://thumbnail_{}", image_id);
                                    let img = egui::Image::from_bytes(uri, png_data.clone());
                                    ui.add(img.max_size(egui::Vec2::splat(26.0)).corner_radius(3));
                                    ui.add_space(6.0);
                                }
                            }
                        }

                        // 描述标签 — 短按选中，长按拖拽
                        let label_resp = ui.add(egui::Label::new(egui::RichText::new(desc.clone()).size(13.0)).sense(egui::Sense::click()));
                        if label_resp.clicked() {
                            self.selected_task = Some(i);
                        }

                        if self.delete_animations.iter().all(|(idx, _)| *idx != i) {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button(t(self.lang, "×", "×")).clicked() {
                                    self.delete_animations.push((i, 0.0));
                                }
                                if ui.small_button(t(self.lang, "编辑", "Edit")).clicked() {
                                    edit_idx = Some(i);
                                }
                            });
                        }
                    });
                }).response);
            });
            let Some(response) = response else { continue; };

            // 弹性动画：负偏移（卡片上移），加在之后让后续卡片下移
            if anim_offset < 0.0 {
                ui.add_space(-anim_offset);
            }

            // 拖拽中：帧间位移计算交换
            if let Some(di) = self.drag_idx {
                if di == i {
                    if let Some(p) = pointer {
                        let dy = p.y - self.drag_start_y;
                        let net = dy - self.drag_accum;
                        if dy.abs() >= 6.0 {
                            self.drag_has_moved = true;
                        }
                        if net < -row_h && i > 0 {
                            swap_action = Some((i, i - 1));
                            self.drag_accum -= row_h;
                        } else if net > row_h && i + 1 < n {
                            swap_action = Some((i, i + 1));
                            self.drag_accum += row_h;
                        }
                    }
                }
            }

            // 拖拽启动：仅左侧描述区域
            if primary_just_pressed
                && delete_shrink == 0.0
                && matches!(self.add_state, AddState::None)
                && self.edit_index.is_none()
                && !self.show_about
            {
                if let Some(p) = pointer {
                    let drag_rect = egui::Rect::from_min_max(
                        egui::pos2(response.rect.left(), response.rect.top()),
                        egui::pos2(response.rect.right() - 72.0, response.rect.bottom()),
                    );
                    if drag_rect.contains(p) {
                        self.selected_task = Some(i);
                        self.drag_idx = Some(i);
                        self.drag_start_y = p.y;
                        self.drag_accum = 0.0;
                    }
                }
            }

            // 更新 hover 状态
            if response.hovered() && !is_dragging {
                self.hovered_task = Some(i);
            } else if self.hovered_task == Some(i) {
                self.hovered_task = None;
            }

            // 执行swap
            if let Some((a, b)) = swap_action.take() {
                self.state.tasks.swap(a, b);
                crate::config::save_tasks(&self.state.tasks);
                self.drag_idx = Some(b);
                if self.selected_task == Some(a) { self.selected_task = Some(b); }
                else if self.selected_task == Some(b) { self.selected_task = Some(a); }
                // 被挤开的卡片弹性动画
                if b > a {
                    if !self.swap_offsets.iter().any(|(idx, _)| *idx == a) {
                        self.swap_offsets.push((a, -(row_h * 0.7)));
                    }
                } else {
                    if !self.swap_offsets.iter().any(|(idx, _)| *idx == a) {
                        self.swap_offsets.push((a, row_h * 0.7));
                    }
                }
            }
        }

        if let Some(i) = edit_idx {
            self.start_edit(i);
        }

        if self.state.tasks.is_empty() {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(t(self.lang, "任务队列为空", "Task queue is empty")).size(16.0).color(C_SUBTEXT));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(t(self.lang, "点击右上+ 添加任务", "Click + to add a task")).size(12.0).color(dim_color_egui(C_SUBTEXT)));
            });
            ui.add_space(60.0);
        }
    }

    // ── 面板 ──────────────────────────────────────────────

    /// 顶部 header：状态指示 + 累计计数
    fn draw_header(&mut self, ui: &mut egui::Ui) {
        let is_on = self.state.clicking.load(Ordering::SeqCst);
        let count = self.state.total_clicks.load(Ordering::SeqCst);

        ui.horizontal(|ui| {
            let (status, dot_color) = if is_on {
                (t(self.lang, "点击..", "Running"), C_GREEN)
            } else {
                (t(self.lang, "等待开始", "Idle"), C_SUBTEXT)
            };
            let dot_r = 4.0;
            let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(dot_r * 2.0, dot_r * 2.0), egui::Sense::hover());
            ui.painter().circle_filled(dot_rect.center(), dot_r, dot_color);
            ui.label(egui::RichText::new(status).size(13.0).color(dot_color));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let lang_btn = egui::Button::new(
                    egui::RichText::new(self.lang.short_label()).size(13.0).color(C_SUBTEXT)
                ).min_size(Vec2::new(28.0, 22.0)).fill(Color32::TRANSPARENT);
                if ui.add(lang_btn).clicked() {
                    self.lang = self.lang.toggle();
                    self.lang.save_to_global();
                    crate::config::save_lang(match self.lang { Lang::CN => "cn", Lang::EN => "en" });
                }
                ui.add_space(8.0);
                if ui.button(t(self.lang, "+ 添加任务", "+ Add Task")).clicked() {
                    self.add_state = AddState::PickType;
                }
                ui.add_space(8.0);
                ui.label(egui::RichText::new(format!("{} {}", t(self.lang, "累计", "Total"), count)).size(12.0).color(C_SUBTEXT));
            });
        });
    }

    /// 控制栏：间隔 / 热键 / 循环 / 锁键 / 锁鼠
    fn draw_control_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(t(self.lang, "间隔", "Interval")).size(12.0).color(C_SUBTEXT));
            let mut secs = self.state.interval_ms as f64 / 1000.0;
            let resp = ui.add(
                egui::DragValue::new(&mut secs)
                    .range(0.01..=f64::MAX)
                    .suffix(t(self.lang, "秒", "s"))
                    .speed(0.01)
                    .custom_formatter(|v, _| format!("{:.2}", v)),
            );
            if resp.changed() {
                let ms = (secs * 1000.0).round() as u64;
                let ms = ms.max(10);
                self.state.interval_ms = ms;
                crate::config::save_interval(ms);
            }
            ui.add_space(20.0);

            ui.label(egui::RichText::new(t(self.lang, "热键", "Hotkey")).size(12.0).color(C_SUBTEXT));
            let hotkey_text = if self.state.hotkey_recording {
                t(self.lang, "按下按键...", "Press a key...").to_owned()
            } else {
                vk_name(self.state.hotkey, self.lang)
            };
            if ui.button(&hotkey_text).clicked() {
                self.state.hotkey_recording = true;
            }

            let (rec_text, rec_color) = if self.recording {
                (t(self.lang, "⏹ 停止录制", "⏹ Stop Record"), C_RED)
            } else if self.rec_armed {
                (t(self.lang, "⏺ 等待热键...", "⏺ Waiting..."), C_ORANGE)
            } else {
                (t(self.lang, "⏺ 录制", "⏺ Record"), C_TEXT)
            };
            if ui.add(
                egui::Button::new(egui::RichText::new(rec_text).size(12.0).color(rec_color))
                    .min_size(Vec2::new(60.0, 0.0))
            ).clicked() {
                if self.recording {
                    self.stop_recording();
                } else {
                    self.rec_armed = !self.rec_armed;
                }
            }

            let mut compress = self.rec_compress;
            if ui.checkbox(&mut compress, t(self.lang, "录制压缩", "Compress")).changed() {
                self.rec_compress = compress;
                crate::config::save_rec_compress(compress);
            }

            let mut loop_on = self.state.task_loop;
            if ui.checkbox(&mut loop_on, t(self.lang, "循环", "Loop")).changed() {
                self.state.task_loop = loop_on;
                crate::config::save_task_loop(loop_on);
            }

            let mut lock_kb = self.state.lock_kb;
            if ui.checkbox(&mut lock_kb, t(self.lang, "锁键", "Lock KB")).changed() {
                self.state.lock_kb = lock_kb;
                crate::config::save_lock_kb(lock_kb);
            }
            let mut lock_mouse = self.state.lock_mouse;
            if ui.checkbox(&mut lock_mouse, t(self.lang, "锁鼠", "Lock Mouse")).changed() {
                self.state.lock_mouse = lock_mouse;
                crate::config::save_lock_mouse(lock_mouse);
            }
            let mut auto_exec = self.state.auto_exec;
            if ui.checkbox(&mut auto_exec, t(self.lang, "启动执行", "Exec on Launch"))
                .on_hover_text(t(self.lang,
                    "手动打开程序时自动开始连点",
                    "Auto-start clicking when launched manually"))
                .changed()
            {
                self.state.auto_exec = auto_exec;
                crate::config::save_auto_exec(auto_exec);
            }
            let mut auto_exec_boot = self.state.auto_exec_boot;
            if ui.checkbox(&mut auto_exec_boot, t(self.lang, "开机执行", "Exec on Boot"))
                .on_hover_text(t(self.lang,
                    "随Windows开机自启时自动开始连点",
                    "Auto-start clicking when launched at system boot"))
                .changed()
            {
                self.state.auto_exec_boot = auto_exec_boot;
                crate::config::save_auto_exec_boot(auto_exec_boot);
            }
        });
    }

    /// 中央主区域：任务列表铺满
    fn draw_main_area(&mut self, ui: &mut egui::Ui) {
        let list_w = ui.available_width().max(200.0);
        self.draw_task_list_inner(ui, list_w);
    }

    fn draw_task_list_inner(&mut self, ui: &mut egui::Ui, width: f32) {
        let dragging = self.drag_idx.is_some();
        let n = self.state.tasks.len();
        let row_h = 40.0;
        egui::ScrollArea::vertical()
            .id_salt("tasks")
            .auto_shrink([false, false])
            .scroll_bar_visibility(if dragging {
                egui::scroll_area::ScrollBarVisibility::AlwaysHidden
            } else {
                egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded
            })
            .drag_to_scroll(!dragging)
            .show_rows(ui, row_h, n, |ui, row_range| {
                ui.set_min_width(width);
                self.render_task_items(ui, row_range);
            });
    }

    /// 底部 footer：操作按钮 + 任务计数
    fn draw_footer(&mut self, ui: &mut egui::Ui) {
        // 顶部分隔线
        ui.painter().hline(
            ui.available_rect_before_wrap().x_range(),
            ui.cursor().top(),
            Stroke::new(1.0, C_SURFACE1),
        );
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button(t(self.lang, "📋 复制", "📋 Copy")).clicked() {
                if let Some(i) = self.selected_task {
                    if i < self.state.tasks.len() {
                        let clone = self.state.tasks[i];
                        self.state.tasks.insert(i + 1, clone);
                        crate::config::save_tasks(&self.state.tasks);
                    }
                }
            }
            if ui.button(t(self.lang, "X 清空", "X Clear")).clicked() {
                for t in &self.state.tasks {
                    Self::cleanup_task_db(t);
                }
                self.state.tasks.clear();
                crate::config::save_tasks(&self.state.tasks);
                self.selected_task = None;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(t(self.lang, "关于", "About")).clicked() {
                    self.show_about = true;
                }
                ui.add_space(8.0);
                if ui.checkbox(&mut self.background, t(self.lang, "后台", "Tray")).changed() {
                    crate::config::save_background(self.background);
                }
                ui.add_space(8.0);
                let mut autostart = self.state.autostart;
                if ui.checkbox(&mut autostart, t(self.lang, "开机自启", "Auto Start"))
                    .on_hover_text(t(self.lang,
                        "Windows启动时自动运行本程序",
                        "Launch this app when Windows starts"))
                    .changed()
                {
                    self.state.autostart = autostart;
                    crate::config::save_autostart(autostart);
                    crate::config::set_autostart_registry(autostart);
                }
                ui.add_space(8.0);
                ui.checkbox(&mut self.show_coords, t(self.lang, "监控坐标", "Coords"));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(format!("{} {}", self.state.tasks.len(), t(self.lang, "个任务", "tasks"))).size(13.0).color(C_SUBTEXT));
            });
        });
    }

    fn draw_about_popup(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }
        egui::Window::new(t(self.lang, "关于", "About"))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                ui.set_min_width(240.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(t(self.lang, "酒要点点", "Beer Clicker")).size(18.0).strong());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("v0.2.0").size(12.0).color(C_SUBTEXT));
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("QQ: 2960965389").size(13.0).color(C_TEXT));
                    ui.add_space(4.0);
                    ui.hyperlink_to("Github: https://github.com/cjxpj", "https://github.com/cjxpj");
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("QQ群: 310345976").size(13.0).color(C_TEXT));
                    ui.add_space(10.0);
                    if ui.button(t(self.lang, "确定", "OK")).clicked() {
                        self.show_about = false;
                    }
                });
            });
    }
}

// ── eframe::App 实现 ──────────────────────────────────────

impl eframe::App for BeerClickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 首帧：居中窗口 + 自动执行
        if self.first_frame {
            self.first_frame = false;
            if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(cmd);
            }
            // 自动执行：启动时自动开始连点（启动执行/开机执行互斥，各管各的）
            let is_autostart_launch = std::env::args().any(|a| a == "--autostart");
            let should_auto_start = if is_autostart_launch {
                self.state.auto_exec_boot
            } else {
                self.state.auto_exec
            };
            if should_auto_start && self.state.tasks.iter().any(|t| matches!(t.action, TaskActionType::MouseClick | TaskActionType::MouseMove | TaskActionType::MouseWheel | TaskActionType::KeyPress | TaskActionType::ComboKey | TaskActionType::ImageMatch | TaskActionType::WaitUntil | TaskActionType::WaitKey | TaskActionType::WaitInput)) {
                self.state.start();
            }
        }

        // 后台模式：关闭窗口时缩到系统托盘
        if ctx.input(|i| i.viewport().close_requested()) && self.background {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            crate::tray::ensure_tray();
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        // 托盘请求恢复窗口
        if crate::tray::SHOW_REQUEST.swap(false, std::sync::atomic::Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        }
        // 托盘请求退出程序
        if crate::tray::EXIT_REQUEST.swap(false, std::sync::atomic::Ordering::SeqCst) {
            self.background = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        self.process_hotkey_recording(ctx);
        if !self.state.hotkey_recording {
            self.poll_hotkey(ctx);
        }
        self.tick_tip();
        self.process_recorded_events();
        self.update_locks();

        // 延迟文件选择：在 UI 渲染前执行
        if let Some(kind) = self.pending_file_pick.take() {
            let mut dialog = rfd::FileDialog::new();
            match kind {
                FilePickKind::Image => {
                    dialog = dialog.add_filter(t(self.lang, "图片文件", "Image files"), &["png", "jpg", "jpeg"]);
                }
                FilePickKind::Exe => {
                    dialog = dialog.add_filter(t(self.lang, "可执行文件/快捷方式", "Executable/Shortcut"), &["exe", "lnk"]);
                }
            }
            self.picked_path = dialog.pick_file().map(|p| p.to_string_lossy().to_string());
        }
        // 延迟另存为
        if self.pending_save_open {
            self.pending_save_open = false;
            self.save_target = rfd::FileDialog::new()
                .add_filter("PNG 图片", &["png"])
                .save_file()
                .map(|p| p.to_string_lossy().to_string());
        }
        if let (Some(target), Some(data)) = (self.save_target.take(), self.pending_save_data.take()) {
            if let Err(e) = std::fs::write(&target, &data) {
                let msg = format!("{}: {}\n{}", t(self.lang, "保存失败", "Save failed"), target, e);
                let title = t(self.lang, "错误", "Error");
                let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
                let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
                unsafe {
                    MessageBoxW(null_mut(), wide.as_ptr(), title_wide.as_ptr(),
                        MB_OK | MB_ICONERROR);
                }
            }
        }

        // ── 面板布局 ──
        egui::TopBottomPanel::top("header")
            .show_separator_line(false)
            .frame(make_panel_frame())
            .show(ctx, |ui| {
                self.draw_header(ui);
            });
        egui::TopBottomPanel::top("controls")
            .show_separator_line(false)
            .frame(make_panel_frame())
            .show(ctx, |ui| {
                self.draw_control_row(ui);
            });
        egui::TopBottomPanel::bottom("footer")
            .show_separator_line(false)
            .frame(make_panel_frame())
            .show(ctx, |ui| {
                if !self.state.tip_text.is_empty() {
                    ui.colored_label(C_GREEN, self.state.tip_text.as_str());
                }
                self.draw_footer(ui);
            });
        egui::CentralPanel::default().frame(make_central_frame()).show(ctx, |ui| {
            self.draw_main_area(ui);
        });

        self.draw_edit_popup(ctx);
        self.draw_add_popup(ctx);
        self.draw_about_popup(ctx);

        // 实时坐标显示
        if self.show_coords {
            let mut pt: winapi::shared::windef::POINT = unsafe { std::mem::zeroed() };
            unsafe { winapi::um::winuser::GetCursorPos(&mut pt); }
            let text = format!("({:.2}, {:.2})", pt.x, pt.y);
            let galley = ctx.fonts(|f| f.layout_no_wrap(text, egui::FontId::monospace(13.0), C_TEAL));
            let bg_w = galley.size().x + 12.0;
            let bg_h = galley.size().y + 6.0;
            let screen_w = ctx.screen_rect().width();
            let x = (screen_w - bg_w) / 2.0;
            let bg_rect = egui::Rect::from_min_size(egui::pos2(x, 2.0), egui::vec2(bg_w, bg_h));
            ctx.debug_painter().rect_filled(bg_rect, egui::CornerRadius::same(4), Color32::from_rgba_premultiplied(0, 0, 0, 180));
            ctx.debug_painter().galley(egui::pos2(x + 6.0, 5.0), galley, C_TEAL);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
    }
}

// ── 逻辑辅助 ──────────────────────────────────────────────

impl BeerClickerApp {
    fn process_hotkey_recording(&mut self, ctx: &egui::Context) {
        if !self.state.hotkey_recording { return; }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.state.hotkey_recording = false;
            return;
        }
        for key in [
            egui::Key::F1, egui::Key::F2, egui::Key::F3, egui::Key::F4,
            egui::Key::F5, egui::Key::F6, egui::Key::F7, egui::Key::F8,
            egui::Key::F9, egui::Key::F10, egui::Key::F11, egui::Key::F12,
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
        ] {
            if ctx.input(|i| i.key_pressed(key)) {
                let vk = egui_key_to_vk(key);
                if vk != 0 {
                    self.state.hotkey = vk;
                    self.state.hotkey_recording = false;
                    self.state.last_f6 = true;
                    crate::config::save_hotkey(self.state.hotkey);
                    if is_mouse_vk(self.state.hotkey) {
                        self.state.tip_text = t(self.lang, "提示：鼠标热键在窗口外点击才生效", "Tip: click outside the window to trigger mouse hotkey").into();
                        self.state.tip_ticks = 90;
                    }
                    return;
                }
            }
        }
    }

    fn tick_tip(&mut self) {
        if self.state.tip_ticks > 0 {
            self.state.tip_ticks -= 1;
            if self.state.tip_ticks == 0 {
                self.state.tip_text.clear();
            }
        }
    }
}

// ── 键盘钩子 ──────────────────────────────────────────────

unsafe extern "system" fn kb_hook_proc(code: i32, _wparam: usize, lparam: isize) -> isize {
    if code >= 0 {
        return 1; // 锁键盘
    }
    CallNextHookEx(null_mut(), code, _wparam, lparam)
}
