//! 前端可调用的命令（`invoke_handler` 注册的那批）。
//!
//! 命令名与参数名是和前端冻结的契约，不能随便改：
//! `load_data` / `save_data(data)` / `set_hotkey(accelerator)` / `hide_window` /
//! `toggle_window` / `quit_app` / `set_always_on_top(value)` /
//! `set_autostart(value)` / `open_data_dir`。
//!
//! 失败时返回中文错误串；需要提示用户的地方同时广播 `notice` 事件。

use tauri::{AppHandle, Emitter as _, Manager as _, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt as _;

use crate::hotkey;
use crate::store::{self, AppData};
use crate::window;
use crate::{AppState, EVENT_NOTICE};

/// 广播一条中文提示给前端（`notice` 事件，载荷为字符串）。
///
/// 同时记一份到托管状态里：`setup` 阶段发的提示，那时网页还没加载，
/// 事件没有听众，只能等前端起来后调 `take_notice` 取走。
pub fn notice(app: &AppHandle, message: &str) {
    if let Ok(mut slot) = app.state::<AppState>().startup_notice.lock() {
        *slot = Some(message.to_string());
    }
    if let Err(e) = app.emit(EVENT_NOTICE, message.to_string()) {
        eprintln!("广播 notice 失败：{e}");
    }
}

/// 取走启动期间留下的提示（没有就返回 `None`）。前端挂载后调用一次。
#[tauri::command]
pub fn take_notice(state: State<'_, AppState>) -> Option<String> {
    state
        .startup_notice
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
}

/// 读取全部数据。**永不失败**：文件缺失或损坏时返回完整默认值。
#[tauri::command]
pub fn load_data(app: AppHandle) -> AppData {
    store::load_data(&app)
}

/// 保存全部数据（原子写），并同步刷新内存里的设置。
#[tauri::command]
pub fn save_data(app: AppHandle, state: State<'_, AppState>, data: AppData) -> Result<(), String> {
    store::save_data(&app, &data)?;
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?;
    *settings = data.settings;
    Ok(())
}

/// 换绑全局快捷键：成功则替换，失败则保留原来的快捷键并返回中文错误。
///
/// 不负责持久化——前端拿到 `Ok` 之后再调用 `save_data` 落盘。
#[tauri::command]
pub fn set_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    accelerator: String,
) -> Result<(), String> {
    let mut current = state
        .hotkey
        .lock()
        .map_err(|_| "快捷键状态已损坏".to_string())?;

    match hotkey::rebind(&app, *current, &accelerator) {
        Ok(shortcut) => {
            *current = Some(shortcut);
            Ok(())
        }
        Err(message) => {
            // 不静默失败：既返回错误，也广播提示，前端两处都能拿到。
            notice(&app, &message);
            Err(message)
        }
    }
}

/// 隐藏面板（前端按 Esc 时调用）。
#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<(), String> {
    window::hide(&app)
}

/// 切换面板显示状态；显示时广播 `window-shown`。
#[tauri::command]
pub fn toggle_window(app: AppHandle) -> Result<(), String> {
    window::toggle(&app)
}

/// 退出应用（托盘「退出」用的是同一个底层调用）。
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// 设置窗口是否置顶。
#[tauri::command]
pub fn set_always_on_top(
    app: AppHandle,
    state: State<'_, AppState>,
    value: bool,
) -> Result<(), String> {
    let Some(window) = app.get_webview_window(window::MAIN_WINDOW) else {
        return Err("找不到主窗口".to_string());
    };
    window
        .set_always_on_top(value)
        .map_err(|e| format!("设置置顶失败：{e}"))?;
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?;
    settings.always_on_top = value;
    Ok(())
}

/// 开关开机自启，返回**实际生效**的状态（以系统为准，不假设一定成功）。
#[tauri::command]
pub fn set_autostart(app: AppHandle, value: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    let outcome = if value {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(e) = outcome {
        let message = format!("{}开机自启失败：{e}", if value { "启用" } else { "关闭" });
        notice(&app, &message);
        return Err(message);
    }
    manager.is_enabled().map_err(|e| {
        let message = format!("读取开机自启状态失败：{e}");
        notice(&app, &message);
        message
    })
}

/// 打开数据目录（目录不存在时先创建，保证一定有东西可打开）。
#[tauri::command]
pub fn open_data_dir(app: AppHandle) -> Result<(), String> {
    let dir = store::data_dir(&app)?;
    app.opener()
        .reveal_item_in_dir(&dir)
        .map_err(|e| format!("打开数据目录失败：{e}"))
}
