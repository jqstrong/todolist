//! 窗口（面板）：显示 / 隐藏 / 切换、失去焦点自动隐藏、位置记忆。
//!
//! 这里只碰窗口本身，不关心快捷键是怎么按下的。主窗口标签固定为 `main`
//! （见 `tauri.conf.json`）。位置写盘做了 ~500ms 防抖，避免拖动窗口时疯狂写磁盘。

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Runtime, WebviewWindow};

use crate::AppState;
use crate::store::{self, WindowPos};

/// 主窗口标签。
pub const MAIN_WINDOW: &str = "main";
/// 窗口被显示时广播给前端的无载荷事件。
pub const EVENT_WINDOW_SHOWN: &str = "window-shown";

/// 位置写盘防抖时长。
const SAVE_POS_DEBOUNCE: Duration = Duration::from_millis(500);

/// 刚显示之后的“宽限期”：这段时间里的失焦不算用户走开。
///
/// 面板自己弹出来的时候（首次启动、托盘唤起、快捷键唤起）常常立刻收到一次失焦——
/// 把应用拉起来的那个窗口会把焦点抢回去。若照收不误，用户双击图标之后什么也看不到。
/// 一秒出头足够让焦点稳定下来，又不至于让面板赖着不走。
pub const SHOW_GRACE: Duration = Duration::from_millis(1200);

/// 显示面板：显示 → 聚焦 → 记下时刻 → 广播 `window-shown`。
pub fn show<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(window) = main_window(app) else {
        return Err("找不到主窗口".to_string());
    };
    window.show().map_err(|e| format!("显示窗口失败：{e}"))?;
    window
        .set_focus()
        .map_err(|e| format!("聚焦窗口失败：{e}"))?;
    if let Ok(mut slot) = app.state::<AppState>().shown_at.lock() {
        *slot = Some(Instant::now());
    }
    app.emit(EVENT_WINDOW_SHOWN, ())
        .map_err(|e| format!("广播事件失败：{e}"))?;
    Ok(())
}

/// 隐藏面板。
pub fn hide<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(window) = main_window(app) else {
        return Err("找不到主窗口".to_string());
    };
    window.hide().map_err(|e| format!("隐藏窗口失败：{e}"))
}

/// 切换面板：可见 → 隐藏；不可见 → 显示 + 聚焦 + 广播 `window-shown`。
pub fn toggle<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(window) = main_window(app) else {
        return Err("找不到主窗口".to_string());
    };
    let visible = window.is_visible().unwrap_or(false);
    if visible { hide(app) } else { show(app) }
}

fn main_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window(MAIN_WINDOW)
}

/// 启动时恢复上次的位置，并夹到最近的可用显示器内（保证不会“跑到屏幕外找不回来”）。
///
/// 返回是否真的恢复到了保存的位置。
pub fn restore_position<R: Runtime>(app: &AppHandle<R>) -> bool {
    let Some(saved) = store::load_window_pos(app) else {
        return false; // 没有存档：用配置里的居中。
    };
    let Some(window) = main_window(app) else {
        return false;
    };
    let pos = compute_position(&window, saved);
    if window.set_position(pos).is_err() {
        return false;
    }
    true
}

/// 把保存的坐标夹进最近的显示器内，并保证窗口左上角落在可见区域。
fn compute_position<R: Runtime>(
    window: &WebviewWindow<R>,
    saved: WindowPos,
) -> PhysicalPosition<i32> {
    let size = window.outer_size().ok();
    // 拿不到显示器信息时，退化成“原样恢复”。
    let Ok(monitors) = window.available_monitors() else {
        return PhysicalPosition::new(saved.x, saved.y);
    };
    if monitors.is_empty() {
        return PhysicalPosition::new(saved.x, saved.y);
    }

    // 目标显示器：离保存坐标最近的那一块（多屏拔掉后仍能回到剩下的屏幕上）。
    let monitor = monitors
        .iter()
        .min_by_key(|m| {
            let mp = m.position();
            let ms = m.size();
            let dx = axis_distance(
                i64::from(saved.x),
                i64::from(mp.x),
                i64::from(mp.x) + i64::from(ms.width),
            );
            let dy = axis_distance(
                i64::from(saved.y),
                i64::from(mp.y),
                i64::from(mp.y) + i64::from(ms.height),
            );
            dx + dy
        })
        .expect("上面已确认非空");

    let mp = monitor.position();
    let ms = monitor.size();
    let min_x = i64::from(mp.x);
    let min_y = i64::from(mp.y);
    let max_x = min_x + (i64::from(ms.width) - i64::from(size.map_or(0, |s| s.width))).max(0);
    let max_y = min_y + (i64::from(ms.height) - i64::from(size.map_or(0, |s| s.height))).max(0);

    let x = i64::from(saved.x).clamp(min_x, max_x);
    let y = i64::from(saved.y).clamp(min_y, max_y);
    PhysicalPosition::new(x as i32, y as i32)
}

/// 点到区间的距离；落在区间内为 0。
fn axis_distance(value: i64, min: i64, max: i64) -> i64 {
    if value < min {
        min - value
    } else if value > max {
        value - max
    } else {
        0
    }
}

/// 注册窗口事件监听：移动后防抖写盘 + 失去焦点时按设置隐藏。
///
/// 在 `setup` 里调用一次。所有错误都在闭包内部消化，不会让应用崩溃。
pub fn register_listeners<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = main_window(app) else {
        return;
    };

    // 每次移动自增；等待结束后只有“最后一次移动”的那一轮会写盘。
    let generation = Arc::new(AtomicU64::new(0));
    let app_for_move = app.clone();
    let gen_for_move = Arc::clone(&generation);
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Moved(_) = event {
            let ticket = gen_for_move.fetch_add(1, Ordering::SeqCst) + 1;
            let app = app_for_move.clone();
            let gen_thread = Arc::clone(&gen_for_move);
            // 用普通线程做防抖：不占用异步运行时，也不引入额外依赖。
            std::thread::spawn(move || {
                std::thread::sleep(SAVE_POS_DEBOUNCE);
                if gen_thread.load(Ordering::SeqCst) != ticket {
                    return; // 期间又移动过，交给更新的那一轮。
                }
                if let Some(window) = app.get_webview_window(MAIN_WINDOW)
                    && let Ok(pos) = window.outer_position()
                {
                    let _ = store::save_window_pos(&app, WindowPos { x: pos.x, y: pos.y });
                }
            });
        }
    });

    let app_for_focus = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            let state = app_for_focus.state::<AppState>();

            // 刚被唤起来就被抢走焦点：这是启动时的噪音，不是用户走开。
            let recent = state
                .shown_at
                .lock()
                .ok()
                .and_then(|slot| *slot)
                .is_some_and(|at| at.elapsed() < SHOW_GRACE);
            if recent {
                return;
            }

            // 热路径：隐藏与否只看内存里的设置，不读磁盘。
            let hide_on_blur = state
                .settings
                .lock()
                .map(|s| s.hide_on_blur)
                .unwrap_or(true);
            if hide_on_blur {
                let _ = hide(&app_for_focus);
            }
        }
    });
}
