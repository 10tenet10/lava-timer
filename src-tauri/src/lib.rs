// 熔岩计时器 — Tauri v2 菜单栏壳
// 覆盖 src-tauri/src/lib.rs

use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    ActivationPolicy, Emitter, LogicalSize, Manager, PhysicalPosition, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

const EDGE_SNAP_THRESHOLD: f64 = 28.0;

#[derive(Default)]
struct WindowLayoutState {
    inner: Mutex<WindowLayout>,
}

struct WindowLayout {
    view: Option<String>,
    expand_upward: bool,
    expanded_height: f64,
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            view: None,
            expand_upward: false,
            expanded_height: 560.0,
        }
    }
}

fn should_expand_upward(
    position_y: i32,
    current_height: i32,
    expanded_height: i32,
    top: i32,
    bottom: i32,
) -> bool {
    let growth = (expanded_height - current_height).max(0);
    let space_above = (position_y - top).max(0);
    let space_below = (bottom - (position_y + current_height)).max(0);

    if space_below >= growth {
        false
    } else if space_above >= growth {
        true
    } else {
        space_above > space_below
    }
}

/// 前端每秒调用:把 "1:24" 之类的计时推到菜单栏标题
#[tauri::command]
fn set_tray_title(app: tauri::AppHandle, title: String) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_title(if title.is_empty() {
            None
        } else {
            Some(title.as_str())
        });
    }
}

/// 让透明原生窗口跟随前端真实内容尺寸，避免不可见区域拦截其它应用点击。
#[tauri::command]
fn set_main_window_size(
    app: tauri::AppHandle,
    layout_state: tauri::State<'_, WindowLayoutState>,
    width: f64,
    height: f64,
    view: String,
) -> Result<(), String> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    let width = width.clamp(80.0, 420.0);
    let height = height.clamp(40.0, 720.0);
    let scale = win.scale_factor().unwrap_or(1.0);
    let old_position = win.outer_position().map_err(|e| e.to_string())?;
    let old_size = win.outer_size().map_err(|e| e.to_string())?;
    let monitor = win
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "current monitor not found".to_string())?;
    let work_area = monitor.work_area();
    let screen_bottom = monitor.position().y + monitor.size().height as i32;
    let threshold = (EDGE_SNAP_THRESHOLD * scale).round() as i32;

    let mut layout = layout_state
        .inner
        .lock()
        .map_err(|_| "window layout state lock poisoned".to_string())?;
    let old_logical_width = old_size.width as f64 / scale;
    let old_logical_height = old_size.height as f64 / scale;
    let growing = height > old_logical_height + 1.0;
    let shrinking = height < old_logical_height - 1.0;
    let expanding_from_capsule = growing && old_logical_height <= 100.0;

    if expanding_from_capsule {
        let requested_height = (height * scale).round() as i32;
        layout.expand_upward = should_expand_upward(
            old_position.y,
            old_size.height as i32,
            requested_height,
            work_area.position.y,
            screen_bottom,
        );
    }

    let old_right = old_position.x + old_size.width as i32;
    let old_bottom = old_position.y + old_size.height as i32;
    let work_right = work_area.position.x + work_area.size.width as i32;
    let snapped_left = (old_position.x - work_area.position.x).abs() <= threshold;
    let snapped_right = (old_right - work_right).abs() <= threshold;

    win.set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;

    // set_size 通过 Tauri 事件队列执行，紧接着读取 outer_size 可能仍得到旧值。
    // 此窗口无装饰，目标外部尺寸可直接由请求的逻辑尺寸和缩放比例确定。
    let new_width = (width * scale).round() as i32;
    let new_height = (height * scale).round() as i32;

    let mut x = if snapped_left {
        work_area.position.x
    } else if snapped_right {
        work_right - new_width
    } else {
        old_position.x + ((old_logical_width - width) * scale / 2.0).round() as i32
    };

    let keep_vertical_anchor = growing || shrinking;
    let mut y = if keep_vertical_anchor && layout.expand_upward {
        old_bottom - new_height
    } else {
        old_position.y
    };

    let max_x = (work_right - new_width).max(work_area.position.x);
    let max_y = (screen_bottom - new_height).max(work_area.position.y);
    x = x.clamp(work_area.position.x, max_x);
    y = y.clamp(work_area.position.y, max_y);

    win.set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    if view != "capsule" {
        layout.expanded_height = height;
    }
    layout.view = Some(view);

    Ok(())
}

/// 返回胶囊从当前位置展开时的垂直方向，并用于及时同步前端箭头朝向。
#[tauri::command]
fn main_window_expands_upward(
    app: tauri::AppHandle,
    layout_state: tauri::State<'_, WindowLayoutState>,
    expanded_height: f64,
) -> Result<bool, String> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let scale = win.scale_factor().unwrap_or(1.0);
    let position = win.outer_position().map_err(|e| e.to_string())?;
    let size = win.outer_size().map_err(|e| e.to_string())?;
    let monitor = win
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "current monitor not found".to_string())?;
    let screen_bottom = monitor.position().y + monitor.size().height as i32;
    let target_height = (expanded_height.clamp(100.0, 720.0) * scale).round() as i32;
    let expands_upward = should_expand_upward(
        position.y,
        size.height as i32,
        target_height,
        monitor.work_area().position.y,
        screen_bottom,
    );

    let mut layout = layout_state
        .inner
        .lock()
        .map_err(|_| "window layout state lock poisoned".to_string())?;
    layout.expand_upward = expands_upward;
    Ok(expands_upward)
}

/// 拖动结束后，在靠近当前显示器工作区边缘时自动吸附。
#[tauri::command]
fn snap_main_window_to_edge(app: tauri::AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let monitor = win
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "current monitor not found".to_string())?;
    let position = win.outer_position().map_err(|e| e.to_string())?;
    let size = win.outer_size().map_err(|e| e.to_string())?;
    let work_area = monitor.work_area();

    let left = work_area.position.x;
    let top = work_area.position.y;
    let right = left + work_area.size.width as i32 - size.width as i32;
    // 底部允许延伸到物理屏幕边缘；与 Dock 重叠时由 macOS 窗口层级处理。
    let bottom = monitor.position().y + monitor.size().height as i32 - size.height as i32;
    let threshold = (EDGE_SNAP_THRESHOLD * monitor.scale_factor()).round() as i32;

    let distances = [
        (position.x - left).abs(),
        (position.x - right).abs(),
        (position.y - top).abs(),
        (position.y - bottom).abs(),
    ];
    let (edge, distance) = distances
        .iter()
        .enumerate()
        .min_by_key(|(_, distance)| *distance)
        .expect("edge distance list is not empty");

    if *distance > threshold {
        return Ok(());
    }

    let mut x = position.x.clamp(left, right.max(left));
    let mut y = position.y.clamp(top, bottom.max(top));
    match edge {
        0 => x = left,
        1 => x = right,
        2 => y = top,
        3 => y = bottom,
        _ => unreachable!(),
    }

    win.set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(WindowLayoutState::default())
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![
            set_tray_title,
            set_main_window_size,
            main_window_expands_upward,
            snap_main_window_to_edge
        ])
        .setup(|app| {
            // 菜单栏 App:不出现在 Dock
            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            // 右键菜单(左键弹面板,右键退出)
            let quit = MenuItem::with_id(app, "quit", "退出 LavaTimer", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            // 托盘图标:icons/tray.png,白色+透明 → 模板图
            let icon = Image::from_path(app.path().resource_dir()?.join("icons/tray.png"))
                .or_else(|_| Image::from_path("icons/tray.png"))?;

            TrayIconBuilder::with_id("main-tray")
                .icon(icon)
                .icon_as_template(true) // 关键:自动适配深浅色菜单栏
                .menu(&menu)
                .show_menu_on_left_click(false) // 左键留给弹面板
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // 必须转发给 positioner,否则 TrayBottomCenter 定位失效
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.set_always_on_top(true);
                                let _ = win.move_window(Position::TrayBottomCenter);
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                WindowEvent::Moved(_) if window.label() == "main" => {
                    let app = window.app_handle();
                    let layout_state = app.state::<WindowLayoutState>();
                    let Ok(mut layout) = layout_state.inner.lock() else {
                        return;
                    };
                    if layout.view.as_deref() != Some("capsule") {
                        return;
                    }

                    let Ok(position) = window.outer_position() else {
                        return;
                    };
                    let Ok(size) = window.outer_size() else {
                        return;
                    };
                    let Ok(Some(monitor)) = window.current_monitor() else {
                        return;
                    };
                    let scale = window.scale_factor().unwrap_or(1.0);
                    let screen_bottom = monitor.position().y + monitor.size().height as i32;
                    let target_height = (layout.expanded_height * scale).round() as i32;
                    let expands_upward = should_expand_upward(
                        position.y,
                        size.height as i32,
                        target_height,
                        monitor.work_area().position.y,
                        screen_bottom,
                    );

                    if expands_upward != layout.expand_upward {
                        layout.expand_upward = expands_upward;
                        drop(layout);
                        let _ = window.emit("lava://expansion-direction", expands_upward);
                    }
                }
                // ⌘W / 关闭 → 隐藏而不是退出
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
