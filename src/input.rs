use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use winapi::um::winuser::{
    SendInput, INPUT, INPUT_MOUSE, INPUT_KEYBOARD, KEYEVENTF_KEYUP,
    MOUSEEVENTF_WHEEL,
    GetCursorPos, SetCursorPos, GetSystemMetrics,
    SM_CXSCREEN, SM_CYSCREEN,
};
use winapi::um::wingdi::{CreateCompatibleDC, DeleteDC, GetDIBits, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB};
use winapi::um::winuser::{GetDC, ReleaseDC, GetDesktopWindow};

/// 通过 SendInput 发送鼠标事件
pub fn send_mouse_input(flags: u32) {
    let mut input: INPUT = unsafe { mem::zeroed() };
    input.type_ = INPUT_MOUSE;
    unsafe {
        input.u.mi_mut().dwFlags = flags;
        SendInput(1, &mut input, mem::size_of::<INPUT>() as i32);
    }
}

/// 通过 SendInput 发送键盘事件
pub fn send_keyboard_input(vk: i32, down: bool) {
    let mut input: INPUT = unsafe { mem::zeroed() };
    input.type_ = INPUT_KEYBOARD;
    unsafe {
        input.u.ki_mut().wVk = vk as u16;
        input.u.ki_mut().dwFlags = if down { 0 } else { KEYEVENTF_KEYUP };
        SendInput(1, &mut input, mem::size_of::<INPUT>() as i32);
    }
}

/// 通过 SendInput 发送鼠标滚轮事件
pub fn send_wheel_input(up: bool) {
    let mut input: INPUT = unsafe { mem::zeroed() };
    input.type_ = INPUT_MOUSE;
    unsafe {
        input.u.mi_mut().dwFlags = MOUSEEVENTF_WHEEL;
        input.u.mi_mut().mouseData = if up { 120 } else { -120i32 as u32 };
        SendInput(1, &mut input, mem::size_of::<INPUT>() as i32);
    }
}

/// 获取屏幕尺寸
pub fn get_screen_size() -> (i32, i32) {
    unsafe {
        (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
    }
}

/// 丝滑移动鼠标到绝对坐标，duration_ms 为总时长（毫秒）
pub fn send_smooth_mouse_move(target_x: i32, target_y: i32, duration_ms: u32, clicking: Option<&AtomicBool>) {
    let mut start = unsafe { mem::zeroed() };
    unsafe { GetCursorPos(&mut start); }
    let sx = start.x;
    let sy = start.y;

    let steps = (duration_ms as f64 / 16.0).ceil() as i32;
    if steps <= 0 {
        unsafe { SetCursorPos(target_x, target_y); }
        return;
    }

    for i in 1..=steps {
        if let Some(flag) = clicking {
            if !flag.load(Ordering::SeqCst) { break; }
        }
        let t = i as f64 / steps as f64;
        let eased = if t < 0.5 {
            2.0 * t * t
        } else {
            -1.0 + (4.0 - 2.0 * t) * t
        };
        let cx = sx + ((target_x - sx) as f64 * eased).round() as i32;
        let cy = sy + ((target_y - sy) as f64 * eased).round() as i32;
        unsafe { SetCursorPos(cx, cy); }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

/// 截图整个桌面，返回 RGBA 像素 (宽, 高, Vec<u8>)
pub fn capture_screen() -> Option<(i32, i32, Vec<u8>)> {
    unsafe {
        let hwnd = GetDesktopWindow();
        let hdc_screen = GetDC(hwnd);
        if hdc_screen.is_null() { return None; }
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_null() { ReleaseDC(hwnd, hdc_screen); return None; }

        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);

        let hbmp = winapi::um::wingdi::CreateCompatibleBitmap(hdc_screen, w, h);
        if hbmp.is_null() {
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_screen);
            return None;
        }
        let old_bmp = winapi::um::wingdi::SelectObject(hdc_mem, hbmp as _);
        winapi::um::wingdi::BitBlt(hdc_mem, 0, 0, w, h, hdc_screen, 0, 0, winapi::um::wingdi::SRCCOPY);

        let data_size = (w * h * 4) as usize;
        let mut pixels: Vec<u8> = vec![0u8; data_size];

        let bi = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // 负值表示自上而下
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader = bi;

        GetDIBits(hdc_mem, hbmp, 0, h as u32, pixels.as_mut_ptr() as _, &mut bmi, DIB_RGB_COLORS);

        winapi::um::wingdi::SelectObject(hdc_mem, old_bmp);
        winapi::um::wingdi::DeleteObject(hbmp as _);
        DeleteDC(hdc_mem);
        ReleaseDC(hwnd, hdc_screen);

        Some((w, h, pixels))
    }
}

/// 模板匹配：在截图中查找模板图片，返回最佳匹配位置（中心坐标）和置信度（0-100）
/// 如果置信度低于阈值则返回 None
/// clicking 用于执行中断检查
#[allow(clippy::too_many_arguments)]
pub fn template_match(
    screen_w: i32, screen_h: i32, screen_pixels: &[u8],
    tmpl_w: i32, tmpl_h: i32, tmpl_pixels: &[u8],
    threshold: u8,
    clicking: Option<&AtomicBool>,
) -> Option<(i32, i32, u8)> {
    if tmpl_w > screen_w || tmpl_h > screen_h || tmpl_w <= 0 || tmpl_h <= 0 {
        return None;
    }

    // 转灰度：BGRA 布局 (Windows 32bpp BI_RGB)
    // pixel[0]=B, pixel[1]=G, pixel[2]=R, pixel[3]=A
    let screen_gray: Vec<f64> = (0..(screen_h as usize))
        .flat_map(|y| (0..(screen_w as usize)).map(move |x| {
            let i = (y * screen_w as usize + x) * 4;
            (screen_pixels[i] as f64 * 0.114 + screen_pixels[i+1] as f64 * 0.587 + screen_pixels[i+2] as f64 * 0.299) / 255.0
        }))
        .collect();

    let tmpl_gray: Vec<f64> = (0..(tmpl_h as usize))
        .flat_map(|y| (0..(tmpl_w as usize)).map(move |x| {
            let i = (y * tmpl_w as usize + x) * 4;
            (tmpl_pixels[i] as f64 * 0.299 + tmpl_pixels[i+1] as f64 * 0.587 + tmpl_pixels[i+2] as f64 * 0.114) / 255.0
        }))
        .collect();

    // 模板均值
    let tmpl_mean = tmpl_gray.iter().sum::<f64>() / tmpl_gray.len() as f64;
    let tmpl_norm: f64 = tmpl_gray.iter().map(|p| (p - tmpl_mean).powi(2)).sum::<f64>().sqrt();
    let tmpl_norm = if tmpl_norm < 1e-6 { 1.0 } else { tmpl_norm };

    let mut best_x = 0i32;
    let mut best_y = 0i32;
    let mut best_score: f64 = -1.0;

    let th = threshold as f64 / 100.0;

    for y in 0..=(screen_h - tmpl_h) as usize {
        // 每行检查一次中断标志
        if let Some(flag) = clicking {
            if !flag.load(Ordering::SeqCst) { return None; }
        }
        for x in 0..=(screen_w - tmpl_w) as usize {
            // 计算归一化互相关 (NCC)
            let mut sum_s = 0.0f64;
            let mut sum_sq = 0.0f64;
            let mut sum_corr = 0.0f64;
            let n = tmpl_gray.len() as f64;

            for ty in 0..tmpl_h as usize {
                let row_off = (y + ty) * screen_w as usize + x;
                let tmpl_off = ty * tmpl_w as usize;
                for tx in 0..tmpl_w as usize {
                    let sv = screen_gray[row_off + tx];
                    let tv = tmpl_gray[tmpl_off + tx];
                    sum_s += sv;
                    sum_sq += sv * sv;
                    sum_corr += sv * tv;
                }
            }
            let screen_mean = sum_s / n;
            let screen_var = (sum_sq / n - screen_mean * screen_mean).sqrt();
            let screen_std = if screen_var < 1e-6 { 1.0 } else { screen_var };

            let ncc = (sum_corr - n * screen_mean * tmpl_mean) / (n * screen_std * tmpl_norm);
            if ncc > best_score {
                best_score = ncc;
                best_x = x as i32;
                best_y = y as i32;
            }
        }
    }

    let confidence = (best_score * 100.0).round() as i32;
    if best_score > th && confidence >= threshold as i32 {
        Some((best_x + tmpl_w / 2, best_y + tmpl_h / 2, confidence as u8))
    } else {
        None
    }
}

/// 移动鼠标到指定窗口标题的窗口中心，返回是否成功
pub fn move_to_window_center(title: &str) -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::winuser::{FindWindowW, GetWindowRect};

    let wide: Vec<u16> = OsStr::new(title).encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        let hwnd = FindWindowW(std::ptr::null_mut(), wide.as_ptr());
        if hwnd.is_null() { return false; }
        let mut rect = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 { return false; }
        let cx = (rect.left + rect.right) / 2;
        let cy = (rect.top + rect.bottom) / 2;
        SetCursorPos(cx, cy);
        true
    }
}
