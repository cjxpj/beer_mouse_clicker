//! 多语言支持模块 — 中/英切换
//!
//! 用法：`t(self.lang, "取消", "Cancel")`  或  `tf(self.lang, "延迟 {}ms", "Delay {}ms")`

use std::sync::atomic::{AtomicU8, Ordering};

/// 全局语言标志 — 供 tray 等无 self 的线程读取
static GLOBAL_LANG: AtomicU8 = AtomicU8::new(0); // 0=CN, 1=EN

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Lang {
    CN,
    EN,
}

impl Lang {
    /// 按钮标签（短）
    pub fn short_label(self) -> &'static str {
        match self {
            Lang::CN => "中",
            Lang::EN => "En",
        }
    }

    /// 切换语言
    pub fn toggle(self) -> Self {
        match self {
            Lang::CN => Lang::EN,
            Lang::EN => Lang::CN,
        }
    }

    /// 从全局静态读取当前语言（供 tray 线程）
    pub fn from_global() -> Self {
        match GLOBAL_LANG.load(Ordering::SeqCst) {
            1 => Lang::EN,
            _ => Lang::CN,
        }
    }

    /// 写入全局静态（UI 切换时调用）
    pub fn save_to_global(self) {
        GLOBAL_LANG.store(match self { Lang::CN => 0, Lang::EN => 1 }, Ordering::SeqCst);
    }

    /// 检测系统语言：中文系统 → CN，其他 → EN
    pub fn detect_system() -> Self {
        let lang_id = unsafe { winapi::um::winnls::GetUserDefaultUILanguage() };
        // 主语言 ID = 低 10 位；LANG_CHINESE = 0x04
        if (lang_id & 0x3FF) == 0x04 {
            Lang::CN
        } else {
            Lang::EN
        }
    }
}

/// 翻译纯文本
#[inline]
pub fn t<'a>(lang: Lang, cn: &'a str, en: &'a str) -> &'a str {
    match lang {
        Lang::CN => cn,
        Lang::EN => en,
    }
}

/// 翻译含 format 占位符的模板字符串
/// 用法：`let tmpl = tf(lang, "{}ms", "{}ms"); format!(tmpl, val)`
#[inline]
pub fn tf<'a>(lang: Lang, cn: &'a str, en: &'a str) -> &'a str {
    match lang {
        Lang::CN => cn,
        Lang::EN => en,
    }
}
