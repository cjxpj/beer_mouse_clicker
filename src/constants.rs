// ── 窗口尺寸 ────────────────────────────────────────────────
pub const WND_W: i32 = 760;
pub const WND_H: i32 = 480;

// ── 任务模式操作类型 ─────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
pub enum TaskActionType {
    MouseClick = 0,
    Delay = 1,
    KeyPress = 2,
    MouseWheel = 3,
    MouseMove = 4,
    MouseDown = 5,
    MouseUp = 6,
    KeyDown = 7,
    KeyUp = 8,
    ImageMatch = 9,
    Notify = 10,
    LostFocus = 11,
    RandomDelay = 12,
    ComboKey = 13,
    WaitUntil = 14,
    CopyText = 15,
    Comment = 16,
    WaitKey = 17,
    WaitInput = 18,
    ShowWindow = 19,
    HideWindow = 20,
    OpenProgram = 21,
}

#[derive(Clone, Copy)]
pub struct TaskStep {
    pub action: TaskActionType,
    /// 鼠标按钮变体 / 虚拟键码 / 延迟毫秒 / (x|y<<32) 用于鼠标移动
    /// 识图: 低32位 = 图片ID, 高32位 = 失败消息ID (0 = 无)
    /// 通知: 消息ID
    pub param: u64,
    /// 鼠标移动: 丝滑移动时长（毫秒），0 = 瞬移
    /// 识图: 可信度 (0-100)
    pub extra: u32,
}

/// 鼠标移动参数: x 低32位, y 高32位（均为带符号 i32）
/// 特殊值：(-1, -1) 表示屏幕居中
pub fn pack_move(x: i32, y: i32) -> u64 {
    ((x as u32) as u64) | (((y as u32) as u64) << 32)
}

pub fn unpack_move(param: u64) -> (i32, i32) {
    (param as i32, (param >> 32) as i32)
}

/// 识图参数: 低32位 image_id, 高32位 fail_msg_id
pub fn pack_image_match(image_id: u32, fail_msg_id: u32) -> u64 {
    (image_id as u64) | ((fail_msg_id as u64) << 32)
}

pub fn unpack_image_match(param: u64) -> (u32, u32) {
    (param as u32, (param >> 32) as u32)
}

/// 组合按键 param: 低32位最多4个VK码（每个1字节），extra = 按键数量
pub fn pack_combo(keys: &[u8]) -> u64 {
    let mut p: u64 = 0;
    for (i, &k) in keys.iter().enumerate() {
        p |= (k as u64) << (i * 8);
    }
    p
}

pub fn unpack_combo(param: u64, count: u32) -> Vec<u8> {
    let n = count.min(4) as usize;
    let mut keys = Vec::with_capacity(n);
    for i in 0..n {
        keys.push(((param >> (i * 8)) & 0xFF) as u8);
    }
    keys
}

/// 鼠标移动 extra: 高字节 == 1 表示相对距离模式，duration = extra & 0xFFFFFF
pub const RELATIVE_FLAG: u32 = 0x0100_0000;

/// 鼠标移动预设
pub const MOVE_CENTER: (i32, i32, &str) = (-1, -1, "居中");
pub const MOUSE_MOVE_PRESETS: &[(i32, i32, &str)] = &[
    MOVE_CENTER,
    (100, 0,   "→"),
    (-100, 0,  "←"),
    (0, 100,   "↓"),
    (0, -100,  "↑"),
];

/// 组合按键预设 (名称, 按键, 说明, VK码列表)
pub static COMBO_PRESETS: &[(&str, &str, &str, &[u8])] = &[
    ("显示桌面", "Win + D", "最小化所有窗口", &[0x5B, b'D']),
    ("文件资源管理器", "Win + E", "打开我的电脑", &[0x5B, b'E']),
    ("运行", "Win + R", "打开运行对话框", &[0x5B, b'R']),
    ("锁定", "Win + L", "锁定电脑屏幕", &[0x5B, b'L']),
    ("任务视图", "Win + Tab", "查看所有窗口", &[0x5B, 0x09]),
    ("任务管理器", "Ctrl + Shift + Esc", "打开任务管理器", &[0xA2, 0xA0, 0x1B]),
    ("截图工具", "Win + Shift + S", "框选区域截图", &[0x5B, 0xA0, b'S']),
    ("复制", "Ctrl + C", "复制选中内容", &[0xA2, b'C']),
    ("粘贴", "Ctrl + V", "粘贴剪贴板内容", &[0xA2, b'V']),
    ("全选", "Ctrl + A", "全选所有内容", &[0xA2, b'A']),
    ("撤销", "Ctrl + Z", "撤销上一步操作", &[0xA2, b'Z']),
    ("剪切", "Ctrl + X", "剪切选中内容", &[0xA2, b'X']),
    ("切换窗口", "Alt + Tab", "切换活动窗口", &[0xA4, 0x09]),
    ("关闭窗口", "Alt + F4", "关闭当前程序", &[0xA4, 0x73]),
];

/// 根据存储的按键匹配预设，返回索引，未匹配返回 0
pub fn find_combo_preset(param: u64, extra: u32) -> Option<usize> {
    let keys = unpack_combo(param, extra);
    for (i, (_, _, _, vks)) in COMBO_PRESETS.iter().enumerate() {
        if *vks == keys.as_slice() {
            return Some(i);
        }
    }
    None
}

// ── 等待按键参数 ──────────────────────────────────────────
/// extra 位: bit 0 = 错误终止, bit 1 = 弹窗提示
pub const WAITKEY_TERMINATE: u32 = 1;
pub const WAITKEY_POPUP: u32 = 2;

#[inline]
pub fn waitkey_terminate_on_wrong(extra: u32) -> bool { extra & WAITKEY_TERMINATE != 0 }
#[inline]
pub fn waitkey_show_popup(extra: u32) -> bool { extra & WAITKEY_POPUP != 0 }

// ── 等待输入参数 ──────────────────────────────────────────
/// extra 位: bit 0 = 复制输入, bit 1 = 忽略失败, bit 2 = 正则匹配
pub const WAITINPUT_COPY: u32 = 1;
pub const WAITINPUT_IGNORE_FAIL: u32 = 2;
pub const WAITINPUT_REGEX: u32 = 4;

#[inline]
pub fn waitinput_copy(extra: u32) -> bool { extra & WAITINPUT_COPY != 0 }
#[inline]
pub fn waitinput_ignore_fail(extra: u32) -> bool { extra & WAITINPUT_IGNORE_FAIL != 0 }
#[inline]
pub fn waitinput_regex(extra: u32) -> bool { extra & WAITINPUT_REGEX != 0 }
