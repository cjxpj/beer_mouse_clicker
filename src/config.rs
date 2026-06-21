use crate::constants::{TaskActionType, TaskStep};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

fn db_path() -> PathBuf {
    match std::env::current_exe() {
        Ok(mut p) => { p.pop(); p.push("beer_clicker.bmc"); p }
        Err(_) => {
            // 回退到 LOCALAPPDATA 或当前目录
            std::env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("beer_clicker.bmc")
        }
    }
}

fn db() -> &'static Mutex<Connection> {
    static DB: OnceLock<Mutex<Connection>> = OnceLock::new();
    DB.get_or_init(|| {
        let conn = Connection::open(db_path()).expect("无法打开数据库");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS config (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS images (
                id   INTEGER PRIMARY KEY,
                data BLOB NOT NULL
            );"
        ).expect("无法创建表");
        Mutex::new(conn)
    })
}

/// 从数据库加载所有配置，未找到则返回默认值
#[allow(clippy::type_complexity)]
pub fn load_all() -> (u64, i32, bool, bool, Vec<TaskStep>, bool, bool, bool, bool, bool, bool) {
    let conn = db().lock().unwrap();
    let interval = get_u64(&conn, "interval_ms", 1000);
    let hotkey = get_i32(&conn, "hotkey", 0x75);
    let lock_kb = get_str(&conn, "lock_kb", "0") == "1";
    let lock_mouse = get_str(&conn, "lock_mouse", "0") == "1";
    let tasks = load_tasks(&conn);
    let task_loop = get_str(&conn, "task_loop", "1") == "1";
    let background = get_str(&conn, "background", "0") == "1";
    let rec_compress = get_str(&conn, "rec_compress", "1") == "1";
    let autostart = get_str(&conn, "autostart", "0") == "1";
    let auto_exec = get_str(&conn, "auto_exec", "0") == "1";
    let auto_exec_boot = get_str(&conn, "auto_exec_boot", "0") == "1";
    (interval, hotkey, lock_kb, lock_mouse, tasks, task_loop, background, rec_compress, autostart, auto_exec, auto_exec_boot)
}

pub fn save_interval(ms: u64) { set("interval_ms", &ms.to_string()); }
pub fn save_hotkey(vk: i32) { set("hotkey", &vk.to_string()); }
pub fn save_lock_kb(on: bool) { set("lock_kb", if on { "1" } else { "0" }); }
pub fn save_lock_mouse(on: bool) { set("lock_mouse", if on { "1" } else { "0" }); }
pub fn save_task_loop(on: bool) { set("task_loop", if on { "1" } else { "0" }); }
pub fn save_background(on: bool) { set("background", if on { "1" } else { "0" }); }
pub fn save_rec_compress(on: bool) { set("rec_compress", if on { "1" } else { "0" }); }
pub fn load_background() -> bool { let conn = db().lock().unwrap(); get_str(&conn, "background", "0") == "1" }
pub fn load_rec_compress() -> bool { let conn = db().lock().unwrap(); get_str(&conn, "rec_compress", "1") == "1" }
pub fn save_autostart(on: bool) { set("autostart", if on { "1" } else { "0" }); }
pub fn save_auto_exec(on: bool) { set("auto_exec", if on { "1" } else { "0" }); }
pub fn save_auto_exec_boot(on: bool) { set("auto_exec_boot", if on { "1" } else { "0" }); }
pub fn save_lang(lang: &str) { set("language", lang); }
pub fn load_lang() -> String { let conn = db().lock().unwrap(); get_str(&conn, "language", "") }

/// 保存任务列表（格式: type:param:extra;type:param:extra;...）
pub fn save_tasks(tasks: &[TaskStep]) {
    let s: String = tasks.iter()
        .map(|t| format!("{}:{}:{}", t.action as u8, t.param, t.extra))
        .collect::<Vec<_>>()
        .join(";");
    set("tasks", &s);
}

/// 从连接加载任务列表（兼容旧格式 type:param）
fn load_tasks(conn: &Connection) -> Vec<TaskStep> {
    let raw = get_str(conn, "tasks", "");
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(';').filter_map(|seg| {
        let mut parts = seg.splitn(3, ':');
        let ty: u8 = parts.next()?.parse().ok()?;
        let param: u64 = parts.next()?.parse().ok()?;
        let extra: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        let action = match ty {
            0 => TaskActionType::MouseClick,
            1 => TaskActionType::Delay,
            2 => TaskActionType::KeyPress,
            3 => TaskActionType::MouseWheel,
            4 => TaskActionType::MouseMove,
            5 => TaskActionType::MouseDown,
            6 => TaskActionType::MouseUp,
            7 => TaskActionType::KeyDown,
            8 => TaskActionType::KeyUp,
            9 => TaskActionType::ImageMatch,
            10 => TaskActionType::Notify,
            11 => TaskActionType::LostFocus,
            12 => TaskActionType::RandomDelay,
            13 => TaskActionType::ComboKey,
            14 => TaskActionType::WaitUntil,
            15 => TaskActionType::CopyText,
            16 => TaskActionType::Comment,
            17 => TaskActionType::WaitKey,
            18 => TaskActionType::WaitInput,
            19 => TaskActionType::ShowWindow,
            20 => TaskActionType::HideWindow,
            21 => TaskActionType::OpenProgram,
            _ => return None,
        };
        Some(TaskStep { action, param, extra })
    }).collect()
}

// ── 辅助函数 ──

fn set(key: &str, value: &str) {
    let conn = db().lock().unwrap();
    conn.execute(
        "INSERT INTO config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    ).ok();
}

fn get_str(conn: &Connection, key: &str, default: &str) -> String {
    conn.query_row(
        "SELECT value FROM config WHERE key = ?1",
        [key],
        |row| row.get(0),
    ).unwrap_or_else(|_| default.to_string())
}

fn get_u64(conn: &Connection, key: &str, default: u64) -> u64 {
    get_str(conn, key, &default.to_string()).parse().unwrap_or(default)
}

fn get_i32(conn: &Connection, key: &str, default: i32) -> i32 {
    get_str(conn, key, &default.to_string()).parse().unwrap_or(default)
}

/// 分配新图片 ID
/// 保存图片到数据库并返回新 ID
pub fn save_image(data: &[u8]) -> u64 {
    let conn = db().lock().unwrap();
    conn.execute_batch("BEGIN IMMEDIATE").ok();
    let id = get_u64(&conn, "next_image_id", 1);
    if conn.execute("INSERT OR REPLACE INTO images (id, data) VALUES (?1, ?2)",
        rusqlite::params![id, data]).is_ok() {
        conn.execute("INSERT INTO config (key, value) VALUES ('next_image_id', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [format!("{}", id + 1).as_str()]).ok();
    }
    conn.execute_batch("COMMIT").ok();
    id
}

/// 根据 ID 从数据库加载图片数据
pub fn load_image(id: u64) -> Option<Vec<u8>> {
    let conn = db().lock().unwrap();
    conn.query_row("SELECT data FROM images WHERE id = ?1", [id], |row| row.get(0)).ok()
}

/// 删除单张图片
pub fn delete_image(id: u64) {
    if id == 0 { return; }
    let conn = db().lock().unwrap();
    conn.execute("DELETE FROM images WHERE id = ?1", [id]).ok();
}

/// 保存消息并返回 ID（事务包裹防止并发 ID 碰撞）
pub fn save_msg(text: &str) -> u64 {
    let conn = db().lock().unwrap();
    conn.execute_batch("BEGIN IMMEDIATE").ok();
    let id = get_u64(&conn, "next_msg_id", 1);
    conn.execute(
        "INSERT INTO config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [format!("msg_{}", id).as_str(), text],
    ).ok();
    conn.execute(
        "INSERT INTO config (key, value) VALUES ('next_msg_id', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [format!("{}", id + 1).as_str()],
    ).ok();
    conn.execute_batch("COMMIT").ok();
    id
}

/// 加载消息
pub fn load_msg(id: u64) -> String {
    let conn = db().lock().unwrap();
    get_str(&conn, &format!("msg_{}", id), "")
}

/// 删除消息
pub fn delete_msg(id: u64) {
    if id == 0 { return; }
    let conn = db().lock().unwrap();
    conn.execute("DELETE FROM config WHERE key = ?1", [format!("msg_{}", id).as_str()]).ok();
}

/// 设置开机自启（写入/删除 Run 注册表项）
pub fn set_autostart_registry(on: bool) {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(run_key) = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        KEY_SET_VALUE,
    ) {
        let app_name = "beer_mouse_clicker";
        if on {
            if let Ok(exe) = std::env::current_exe() {
                let cmd = format!("\"{}\" --autostart", exe.to_string_lossy());
                run_key.set_value(app_name, &cmd).ok();
            }
        } else {
            run_key.delete_value(app_name).ok();
        }
    }
}
