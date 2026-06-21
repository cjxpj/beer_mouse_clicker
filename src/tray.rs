//! 系统托盘图标 —— 独立线程跑消息循环，避免与 winit 冲突

use crate::lang::Lang;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HWND, POINT, HICON};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::{
    ExtractIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use winapi::um::winuser::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW,
    LoadIconW, PostQuitMessage, RegisterClassExW, SetForegroundWindow,
    ShowWindow, TrackPopupMenu, TranslateMessage,
    HWND_MESSAGE, IDI_APPLICATION, MF_STRING, MSG, SW_SHOW, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP,
    WM_RBUTTONUP, WM_USER, WNDCLASSEXW,
};

/// 托盘请求主窗口显示（eframe update 中轮询）
pub static SHOW_REQUEST: AtomicBool = AtomicBool::new(false);
/// 托盘请求退出程序（eframe update 中轮询）
pub static EXIT_REQUEST: AtomicBool = AtomicBool::new(false);

static TRAY_RUNNING: AtomicBool = AtomicBool::new(false);

const WM_TRAYICON: u32 = WM_USER + 0x100;
const TRAY_ID: u32 = 1;
const MENU_SHOW: u32 = 1;
const MENU_EXIT: u32 = 2;

/// 创建系统托盘图标（仅在尚未运行时创建）
pub fn ensure_tray() {
    if TRAY_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(|| {
        unsafe { tray_thread(); }
        TRAY_RUNNING.store(false, Ordering::SeqCst);
    });
}

/// 直接 Win32 显示窗口 + 设标志让 eframe 同步状态
unsafe fn show_main_window() {
    let title = encode_wide("酒要点点\0");
    let hwnd = FindWindowW(null(), title.as_ptr());
    if !hwnd.is_null() {
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
    }
    SHOW_REQUEST.store(true, Ordering::SeqCst);
}

/// 右击弹出菜单
unsafe fn show_context_menu(msg_hwnd: HWND) {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }
    let show_text = encode_wide(&format!("{}\0", crate::lang::t(Lang::from_global(), "显示窗口", "Show Window")));
    let exit_text = encode_wide(&format!("{}\0", crate::lang::t(Lang::from_global(), "退出程序", "Exit")));
    AppendMenuW(menu, MF_STRING, MENU_SHOW as usize, show_text.as_ptr());
    AppendMenuW(menu, MF_STRING, MENU_EXIT as usize, exit_text.as_ptr());

    let mut pt: POINT = std::mem::zeroed();
    GetCursorPos(&mut pt);
    SetForegroundWindow(msg_hwnd);

    let cmd = TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_BOTTOMALIGN | winapi::um::winuser::TPM_RETURNCMD,
        pt.x,
        pt.y,
        0,
        msg_hwnd,
        null_mut(),
    );
    DestroyMenu(menu);

    match cmd as u32 {
        MENU_SHOW => show_main_window(),
        MENU_EXIT => {
            // 不直接发 WM_CLOSE（会被后台拦截），改为设标志
            let title = encode_wide("酒要点点\0");
            let hwnd = FindWindowW(null(), title.as_ptr());
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_SHOW);
                SetForegroundWindow(hwnd);
            }
            EXIT_REQUEST.store(true, Ordering::SeqCst);
            SHOW_REQUEST.store(true, Ordering::SeqCst);
            TRAY_RUNNING.store(false, Ordering::SeqCst);
        }
        _ => {}
    }
}

unsafe fn tray_thread() {
    let hinst = GetModuleHandleW(null());

    let class_name = encode_wide("BeerClickerTray\0");
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(tray_wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinst,
        hIcon: null_mut(),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: null_mut(),
    };
    RegisterClassExW(&wc);

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        class_name.as_ptr(),
        0,
        0, 0, 0, 0,
        HWND_MESSAGE,
        null_mut(),
        hinst,
        null_mut(),
    );
    if hwnd.is_null() {
        return;
    }

    let tip_str = format!("{}\0", crate::lang::t(Lang::from_global(), "酒要点点", "Beer Clicker"));
    let tip_wide = encode_wide(&tip_str);
    let mut tip: [u16; 128] = [0; 128];
    let len = tip_wide.len().min(127);
    tip[..len].copy_from_slice(&tip_wide[..len]);

    let hicon = load_app_icon();

    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAYICON,
        hIcon: hicon,
        szTip: tip,
        dwState: 0,
        dwStateMask: 0,
        szInfo: [0; 256],
        u: std::mem::zeroed(),
        szInfoTitle: [0; 64],
        dwInfoFlags: 0,
        guidItem: std::mem::zeroed(),
        hBalloonIcon: null_mut(),
    };

    if winapi::um::shellapi::Shell_NotifyIconW(NIM_ADD, &mut nid) == 0 {
        DestroyWindow(hwnd);
        return;
    }

    let mut msg: MSG = std::mem::zeroed();
    while TRAY_RUNNING.load(Ordering::SeqCst) {
        if GetMessageW(&mut msg, hwnd, 0, 0) <= 0 {
            break;
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    winapi::um::shellapi::Shell_NotifyIconW(NIM_DELETE, &mut nid);
    winapi::um::winuser::DestroyIcon(hicon);
    DestroyWindow(hwnd);
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_TRAYICON {
        let ev = (lparam & 0xFFFF) as u32;
        if ev == WM_LBUTTONUP || ev == WM_LBUTTONDBLCLK {
            show_main_window();
        } else if ev == WM_RBUTTONUP {
            show_context_menu(hwnd);
        }
        return 0;
    }
    if msg == WM_DESTROY {
        PostQuitMessage(0);
        return 0;
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// 从 exe 中提取图标作为 HICON（资源 ID=1，由 app.rc 嵌入）
unsafe fn load_app_icon() -> HICON {
    // 优先用 ExtractIconW 从运行中 exe 提取
    if let Ok(exe) = std::env::current_exe() {
        let wide: Vec<u16> = exe.to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let icon = ExtractIconW(GetModuleHandleW(null()), wide.as_ptr(), 0);
        if !icon.is_null() && icon != 1 as HICON {
            return icon;
        }
    }
    // 回退：系统默认应用图标
    LoadIconW(null_mut(), IDI_APPLICATION)
}
