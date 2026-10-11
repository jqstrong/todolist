//! 高知待办（Gaozhi）——Tauri 后端入口。
//!
//! 应用形态：一个常驻托盘、随时待命的键盘唤起面板（类似 Spotlight）。
//! - `store`    —— 磁盘数据（任务/设置/窗口坐标）的读写，全部原子写。
//! - `hotkey`   —— 加速键字符串解析 + 全局快捷键注册/换绑。
//! - `window`   —— 面板显示/隐藏/切换、失焦自动隐藏、位置记忆。
//! - `commands` —— 暴露给前端的 `invoke` 命令。
//!
//! 所有面向用户的文案都是简体中文。

mod commands;
mod hotkey;
mod store;
mod window;

use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager as _, RunEvent, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_global_shortcut::ShortcutState;

use store::Settings;

/// 窗口被显示时广播的事件（无载荷）。
pub const EVENT_WINDOW_SHOWN: &str = window::EVENT_WINDOW_SHOWN;
/// 需要提示用户时广播的事件（载荷为中文 `String`）。
pub const EVENT_NOTICE: &str = "notice";

/// 托盘菜单项 id。
const TRAY_MENU_TOGGLE: &str = "toggle";
const TRAY_MENU_QUIT: &str = "quit";

/// 托盘提示文字。
const TRAY_TOOLTIP: &str = "待办";

/// 全局托管状态：只放热路径要用的东西，避免在事件回调里读磁盘。
#[derive(Default)]
pub struct AppState {
    /// 当前已注册的全局快捷键。
    pub hotkey: Mutex<Option<tauri_plugin_global_shortcut::Shortcut>>,
    /// 内存中的设置副本（失焦隐藏等判断直接读这里）。
    pub settings: Mutex<Settings>,
    /// 前端挂载之前发出的提示（例如快捷键注册失败）。
    ///
    /// `setup` 里注册快捷键时网页还没加载，那时广播的 `notice` 没人听得到，
    /// 所以先存在这里，等前端起来再调 `take_notice` 取走。
    pub startup_notice: Mutex<Option<String>>,
    /// 面板最近一次被显示的时刻。
    ///
    /// 刚显示就“失去焦点”是启动时的常态：把应用拉起来的那个窗口（资源管理器、终端）
    /// 会把焦点抢回去。这时不该收起，见 `window::SHOW_GRACE`。
    pub shown_at: Mutex<Option<std::time::Instant>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance 必须第一个注册：第二个实例在其它插件初始化之前就退出。
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // 第二次启动：把已有窗口显示出来并聚焦。
            let _ = window::show(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                // 全局 handler 对按下和抬起都会触发，必须只认 Pressed，
                // 否则按住热键会反复开关面板。
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = window::toggle(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::load_data,
            commands::save_data,
            commands::set_hotkey,
            commands::hide_window,
            commands::toggle_window,
            commands::quit_app,
            commands::set_always_on_top,
            commands::set_autostart,
            commands::open_data_dir,
            commands::take_notice,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // 0) 先判断这是不是第一次启动：是的话结束时要把面板亮出来。
            let first_run = !store::data_exists(&handle);

            // 1) 读数据 → 把设置与启动同步放好。
            let data = store::load_data(&handle);
            let settings = data.settings.clone();
            app.manage(AppState {
                hotkey: Mutex::new(None),
                settings: Mutex::new(settings.clone()),
                startup_notice: Mutex::new(None),
                shown_at: Mutex::new(None),
            });

            // 2) 全局快捷键：注册失败不崩，只提示。
            match hotkey::register_initial(&handle, &settings.hotkey) {
                Ok(shortcut) => {
                    let state = handle.state::<AppState>();
                    if let Ok(mut slot) = state.hotkey.lock() {
                        *slot = Some(shortcut);
                    }
                }
                Err(message) => {
                    eprintln!("初始快捷键注册失败：{message}");
                    commands::notice(&handle, &message);
                }
            }

            // 3) 窗口：恢复位置（夹进可见显示器）→ 置顶 → 开机自启对齐。
            if !window::restore_position(&handle) {
                eprintln!("未恢复窗口位置，使用默认居中。");
            }
            if let Some(main) = handle.get_webview_window(window::MAIN_WINDOW) {
                let _ = main.set_always_on_top(settings.always_on_top);
            } else {
                eprintln!("找不到主窗口，跳过置顶设置。");
            }
            sync_autostart(&handle, settings.launch_at_startup);

            // 4) 托盘 + 窗口事件。
            if let Err(e) = setup_tray(&handle) {
                eprintln!("托盘初始化失败：{e}");
            }
            window::register_listeners(&handle);

            // 5) 第一次启动：把面板直接亮出来。否则用户双击图标之后
            //    面前什么都没有（窗口默认隐藏），只能靠托盘或快捷键再找它。
            if first_run {
                if let Err(e) = window::show(&handle) {
                    eprintln!("首次启动显示面板失败：{e}");
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // 面板不进“关闭即退出”的逻辑：点关闭只是隐藏，应用继续待在托盘里。
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("启动 Tauri 应用失败")
        .run(|_app, event| {
            if let RunEvent::ExitRequested { api, code, .. } = &event {
                // 没有窗口了也不算退出：只有显式 `app.exit()`（托盘「退出」）才真的结束。
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}

/// 建托盘：提示「待办」，菜单只有「显示 / 隐藏」和「退出」，左键单击切换面板。
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, TRAY_MENU_TOGGLE, "显示 / 隐藏", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, TRAY_MENU_QUIT, "退出", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&toggle, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip(TRAY_TOOLTIP)
        .menu(&menu)
        // 左键留给“切换面板”，右键才弹菜单。
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_MENU_TOGGLE => {
                let _ = window::toggle(app);
            }
            TRAY_MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = window::toggle(tray.app_handle());
            }
        });

    // 托盘图标直接用应用图标（icons/ 下的默认图标）。
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}

/// 让系统里的自启状态与设置一致；失败的差异只提示，不阻塞启动。
fn sync_autostart(app: &AppHandle, wanted: bool) {
    let manager = app.autolaunch();
    match manager.is_enabled() {
        Ok(current) if current == wanted => {}
        Ok(_) => {
            let outcome = if wanted {
                manager.enable()
            } else {
                manager.disable()
            };
            if let Err(e) = outcome {
                commands::notice(
                    app,
                    &format!("{}开机自启失败：{e}", if wanted { "启用" } else { "关闭" }),
                );
            }
        }
        Err(e) => eprintln!("读取开机自启状态失败：{e}"),
    }
}
