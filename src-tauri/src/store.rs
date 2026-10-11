//! 数据层：磁盘上的 `AppData`（任务 + 设置）与窗口位置 `WindowPos` 的读写。
//!
//! 约定：
//! - `%APPDATA%\com.jqstrong.gaozhi\data.json`  —— 任务与设置，唯一写入方是本进程。
//! - `%APPDATA%\com.jqstrong.gaozhi\window.json` —— 窗口坐标，窗口移动后防抖写入。
//! - 所有写入都是“先写临时文件再整体替换”的原子写法，避免半截 JSON 破坏存档。
//! - 读取永不失败：文件缺失、损坏、字段缺失都会退化成默认值。

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

/// 当前存档结构版本号。
pub const DATA_VERSION: u32 = 1;

/// 单条任务。
///
/// 字段全部带 `#[serde(default)]`：老的或残缺的 JSON 也一定能反序列化成功。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub done_at: Option<i64>,
}

impl Task {
    /// 创建一个新任务（时间戳为当前毫秒）。
    #[allow(dead_code)] // 后端不生成任务，保留给将来的 Rust 侧逻辑与测试使用。
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            done: false,
            created_at: now_ms(),
            done_at: None,
        }
    }
}

/// 应用设置。默认值与前端约定一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default = "default_true")]
    pub hide_on_blur: bool,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default)]
    pub launch_at_startup: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_true")]
    pub show_completed: bool,
}

/// 默认全局快捷键：Ctrl+Alt+Space。
pub fn default_hotkey() -> String {
    "Ctrl+Alt+Space".to_string()
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "system".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(),
            hide_on_blur: true,
            always_on_top: true,
            launch_at_startup: false,
            theme: default_theme(),
            show_completed: true,
        }
    }
}

/// 前端 `load_data` / `save_data` 的完整载荷。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub settings: Settings,
}

fn default_version() -> u32 {
    DATA_VERSION
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            version: DATA_VERSION,
            tasks: Vec::new(),
            settings: Settings::default(),
        }
    }
}

/// `window.json` 的内容：窗口左上角的物理像素坐标。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WindowPos {
    pub x: i32,
    pub y: i32,
}

/// 当前 Unix 毫秒时间戳。
pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 应用数据目录：`%APPDATA%\com.jqstrong.gaozhi`（目录不存在时创建）。
pub fn data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法定位数据目录：{e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建数据目录：{e}"))?;
    Ok(dir)
}

fn data_file(dir: &Path) -> PathBuf {
    dir.join("data.json")
}

fn window_file(dir: &Path) -> PathBuf {
    dir.join("window.json")
}

/// 数据文件是否已经存在——用来判断“这是不是第一次启动”。
///
/// 首次启动要把面板直接显示出来，否则用户双击图标之后什么也看不到（窗口默认是隐藏的）。
/// 目录拿不到时保守地当成“不是首次启动”，宁可不弹，也不要每次开机都弹。
pub fn data_exists<R: Runtime>(app: &AppHandle<R>) -> bool {
    match data_dir(app) {
        Ok(dir) => data_file(&dir).exists(),
        Err(_) => true,
    }
}

/// 原子写：同目录写临时文件 → `rename` 整体替换。
fn write_atomic(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("无法创建目录：{e}"))?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents).map_err(|e| format!("写入失败：{e}"))?;
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("保存失败：{e}"));
    }
    Ok(())
}

/// 读取 `data.json`。**永不失败**：任何异常都返回默认数据。
pub fn load_data<R: Runtime>(app: &AppHandle<R>) -> AppData {
    let Ok(dir) = data_dir(app) else {
        return AppData::default();
    };
    let path = data_file(&dir);
    let Ok(raw) = fs::read_to_string(&path) else {
        // 文件还不存在（首次启动）——用默认值，不算错误。
        return AppData::default();
    };
    // 记事本一类的编辑器可能写进 UTF-8 BOM。对 serde 来说那是坏字节，
    // 但对用户来说文件是好好的——不能因为一个 BOM 就把人的待办当成损坏数据丢掉。
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(&raw);
    match serde_json::from_str::<AppData>(raw) {
        Ok(data) => data,
        Err(e) => {
            // 存档损坏：留一份带时间戳的备份，方便排查，然后从默认值重新开始。
            let backup = dir.join(format!("data.corrupt-{}.json", now_ms()));
            let _ = fs::rename(&path, backup);
            eprintln!("data.json 解析失败（已备份并重置）：{e}");
            AppData::default()
        }
    }
}

/// 写入 `data.json`（原子替换）。
pub fn save_data<R: Runtime>(app: &AppHandle<R>, data: &AppData) -> Result<(), String> {
    let dir = data_dir(app)?;
    let json = serde_json::to_string_pretty(data).map_err(|e| format!("序列化失败：{e}"))?;
    write_atomic(&data_file(&dir), &json)
}

/// 读取窗口位置；文件缺失或损坏时返回 `None`（交给系统居中/默认位置）。
pub fn load_window_pos<R: Runtime>(app: &AppHandle<R>) -> Option<WindowPos> {
    let dir = data_dir(app).ok()?;
    let raw = fs::read_to_string(window_file(&dir)).ok()?;
    serde_json::from_str::<WindowPos>(&raw).ok()
}

/// 写入窗口位置（原子替换）。
pub fn save_window_pos<R: Runtime>(app: &AppHandle<R>, pos: WindowPos) -> Result<(), String> {
    let dir = data_dir(app)?;
    let json = serde_json::to_string(&pos).map_err(|e| format!("序列化失败：{e}"))?;
    write_atomic(&window_file(&dir), &json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_frozen_contract() {
        let data = AppData::default();
        assert_eq!(data.version, 1);
        assert!(data.tasks.is_empty());
        assert_eq!(data.settings.hotkey, "Ctrl+Alt+Space");
        assert!(data.settings.hide_on_blur);
        assert!(data.settings.always_on_top);
        assert!(!data.settings.launch_at_startup);
        assert_eq!(data.settings.theme, "system");
        assert!(data.settings.show_completed);
    }

    #[test]
    fn empty_json_object_deserialises_to_full_defaults() {
        // 老版本或残缺的存档不能导致解析失败。
        let data: AppData = serde_json::from_str("{}").expect("空对象应能解析");
        assert_eq!(data.version, 1);
        assert!(data.settings.hide_on_blur);
        assert_eq!(data.settings.hotkey, "Ctrl+Alt+Space");
        assert_eq!(data.settings.theme, "system");
        assert!(data.tasks.is_empty());
    }

    #[test]
    fn partial_settings_keep_other_defaults() {
        let raw = r#"{"version":1,"settings":{"hotkey":"Ctrl+Shift+K","theme":"night"}}"#;
        let data: AppData = serde_json::from_str(raw).expect("部分字段应能解析");
        assert_eq!(data.settings.hotkey, "Ctrl+Shift+K");
        assert_eq!(data.settings.theme, "night");
        // 没写的字段回落到默认值。
        assert!(data.settings.hide_on_blur);
        assert!(data.settings.always_on_top);
        assert!(!data.settings.launch_at_startup);
        assert!(data.settings.show_completed);
    }

    #[test]
    fn partial_task_keeps_defaults() {
        let raw = r#"{"tasks":[{"title":"买牛奶"}]}"#;
        let data: AppData = serde_json::from_str(raw).expect("残缺任务应能解析");
        assert_eq!(data.tasks.len(), 1);
        assert_eq!(data.tasks[0].title, "买牛奶");
        assert_eq!(data.tasks[0].id, "");
        assert!(!data.tasks[0].done);
        assert_eq!(data.tasks[0].created_at, 0);
        assert_eq!(data.tasks[0].done_at, None);
    }

    #[test]
    fn serialises_with_camel_case_field_names() {
        // 前端按 camelCase 读取，字段名不能变。
        let json = serde_json::to_string(&AppData::default()).expect("应能序列化");
        for key in [
            "\"version\"",
            "\"tasks\"",
            "\"settings\"",
            "\"hotkey\"",
            "\"hideOnBlur\"",
            "\"alwaysOnTop\"",
            "\"launchAtStartup\"",
            "\"theme\"",
            "\"showCompleted\"",
        ] {
            assert!(json.contains(key), "缺少字段 {key}：{json}");
        }
        let task = serde_json::to_string(&Task::new("t1", "写周报")).expect("应能序列化");
        for key in [
            "\"id\"",
            "\"title\"",
            "\"done\"",
            "\"createdAt\"",
            "\"doneAt\"",
        ] {
            assert!(task.contains(key), "缺少字段 {key}：{task}");
        }
    }

    #[test]
    fn round_trips_through_json() {
        let mut original = AppData::default();
        original.tasks.push(Task::new("id-1", "写周报"));
        original.tasks[0].done = true;
        original.tasks[0].done_at = Some(1_700_000_000_000);
        original.settings.theme = "night".to_string();
        original.settings.show_completed = false;

        let json = serde_json::to_string(&original).expect("应能序列化");
        let restored: AppData = serde_json::from_str(&json).expect("应能反序列化");

        assert_eq!(restored.version, original.version);
        assert_eq!(restored.tasks.len(), 1);
        assert_eq!(restored.tasks[0].id, "id-1");
        assert_eq!(restored.tasks[0].title, "写周报");
        assert!(restored.tasks[0].done);
        assert_eq!(restored.tasks[0].done_at, Some(1_700_000_000_000));
        assert_eq!(restored.settings.theme, "night");
        assert!(!restored.settings.show_completed);
    }

    #[test]
    fn window_pos_round_trips() {
        let pos = WindowPos { x: -120, y: 480 };
        let json = serde_json::to_string(&pos).expect("应能序列化");
        assert_eq!(json, r#"{"x":-120,"y":480}"#);
        let restored: WindowPos = serde_json::from_str(&json).expect("应能反序列化");
        assert_eq!((restored.x, restored.y), (-120, 480));
    }
}
