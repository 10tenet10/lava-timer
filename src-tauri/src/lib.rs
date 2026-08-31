// 熔岩计时器 — Tauri v2 菜单栏壳
// 覆盖 src-tauri/src/lib.rs

#[cfg(target_os = "macos")]
use objc2_app_kit::{NSScrollElasticity, NSScrollView, NSView};
#[cfg(target_os = "macos")]
use objc2_foundation::NSPoint;
#[cfg(target_os = "macos")]
use std::ptr::NonNull;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, LogicalSize, Manager, PhysicalPosition, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

const EDGE_SNAP_THRESHOLD: f64 = 28.0;

#[cfg(target_os = "macos")]
fn lock_webview_scroll(view: &NSView) {
    if let Some(scroll_view) = view.downcast_ref::<NSScrollView>() {
        scroll_view.setHasHorizontalScroller(false);
        scroll_view.setHasVerticalScroller(false);
        scroll_view.setAutohidesScrollers(true);
        scroll_view.setHorizontalScrollElasticity(NSScrollElasticity::None);
        scroll_view.setVerticalScrollElasticity(NSScrollElasticity::None);
        let clip_view = scroll_view.contentView();
        clip_view.scrollToPoint(NSPoint::new(0.0, 0.0));
        scroll_view.reflectScrolledClipView(&clip_view);
    }

    for subview in view.subviews() {
        lock_webview_scroll(&subview);
    }
}

#[cfg(target_os = "macos")]
fn install_webview_scroll_lock(window: &tauri::Webview) -> tauri::Result<()> {
    window.with_webview(|webview| unsafe {
        let view: &NSView = &*webview.inner().cast();
        lock_webview_scroll(view);
    })
}

#[derive(Default)]
struct ScreenInactiveState {
    last_at: AtomicU64,
}

#[derive(Default)]
struct TrayTimerState {
    inner: Mutex<TrayTimer>,
}

#[derive(Default)]
struct TrayTimer {
    base_seconds: u64,
    run_start_ms: Option<u64>,
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn tray_elapsed_seconds(timer: &TrayTimer, now_ms: u64) -> u64 {
    timer.base_seconds
        + timer
            .run_start_ms
            .map(|started_at| now_ms.saturating_sub(started_at) / 1000)
            .unwrap_or(0)
}

fn tray_timer_title(timer: &TrayTimer, now_ms: u64) -> String {
    if timer.run_start_ms.is_none() {
        return "Lava".to_string();
    }
    let seconds = tray_elapsed_seconds(timer, now_ms);
    format!("{}:{:02}", seconds / 3600, (seconds % 3600) / 60)
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_always_on_top(true);
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[tauri::command]
fn hide_main_window(app: tauri::AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    win.hide().map_err(|e| e.to_string())
}

fn apply_tray_timer_title(app: &tauri::AppHandle, now_ms: u64) {
    let title = {
        let state = app.state::<TrayTimerState>();
        let Ok(timer) = state.inner.lock() else {
            return;
        };
        tray_timer_title(&timer, now_ms)
    };

    if let Some(tray) = app.tray_by_id("main-tray") {
        // 暂停时保留 Lava 标识，确保模板图标不可见时仍有菜单栏入口。
        let _ = tray.set_title(Some(title.as_str()));
    }
}

fn pause_tray_timer_at(app: &tauri::AppHandle, stopped_at: u64) {
    {
        let state = app.state::<TrayTimerState>();
        let Ok(mut timer) = state.inner.lock() else {
            return;
        };
        if let Some(started_at) = timer.run_start_ms.take() {
            timer.base_seconds = timer
                .base_seconds
                .saturating_add(stopped_at.saturating_sub(started_at) / 1000);
        }
    }
    apply_tray_timer_title(app, stopped_at);
}

fn install_tray_timer_updater(app: tauri::AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        apply_tray_timer_title(&app, unix_time_ms());
    });
}

#[cfg(target_os = "macos")]
fn record_screen_inactive(app: &tauri::AppHandle) {
    let stopped_at = unix_time_ms();
    app.state::<ScreenInactiveState>()
        .last_at
        .store(stopped_at, Ordering::SeqCst);
    // WebView 隐藏或熄屏时可能被系统节流，托盘必须在原生侧立即暂停。
    pause_tray_timer_at(app, stopped_at);
    let _ = app.emit("lava://screen-off", stopped_at);
}

#[tauri::command]
fn latest_screen_inactive_at(state: tauri::State<'_, ScreenInactiveState>) -> Option<u64> {
    match state.last_at.load(Ordering::SeqCst) {
        0 => None,
        timestamp => Some(timestamp),
    }
}

#[cfg(target_os = "macos")]
fn install_screen_off_observers(app: tauri::AppHandle) {
    use block2::RcBlock;
    use objc2_app_kit::{
        NSWorkspace, NSWorkspaceScreensDidSleepNotification,
        NSWorkspaceSessionDidResignActiveNotification, NSWorkspaceWillSleepNotification,
    };
    use objc2_foundation::{ns_string, NSDistributedNotificationCenter, NSNotification};

    let center = NSWorkspace::sharedWorkspace().notificationCenter();

    // 这些是 AppKit 提供、进程生命周期内稳定的通知名常量。
    let notification_names = unsafe {
        [
            NSWorkspaceScreensDidSleepNotification,
            NSWorkspaceSessionDidResignActiveNotification,
            NSWorkspaceWillSleepNotification,
        ]
    };

    for notification_name in notification_names {
        let app = app.clone();
        let block: RcBlock<dyn Fn(NonNull<NSNotification>)> = RcBlock::new(move |_| {
            record_screen_inactive(&app);
        });

        // NSWorkspace 的通知中心会持有返回的观察者，生命周期与应用一致。
        unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(notification_name),
                None,
                None,
                &block,
            );
        }
    }

    // 屏保启动早于显示器休眠；macOS 通过分布式通知发布这个状态变化。
    let screensaver_center = NSDistributedNotificationCenter::defaultCenter();
    for notification_name in [
        ns_string!("com.apple.screensaver.didstart"),
        ns_string!("com.apple.screenIsLocked"),
    ] {
        let app = app.clone();
        let block: RcBlock<dyn Fn(NonNull<NSNotification>)> = RcBlock::new(move |_| {
            record_screen_inactive(&app);
        });

        unsafe {
            screensaver_center.addObserverForName_object_queue_usingBlock(
                Some(notification_name),
                None,
                None,
                &block,
            );
        }
    }
}

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

/// 前端只同步计时基数和起点；之后由原生线程持续刷新菜单栏。
#[tauri::command]
fn sync_tray_timer(
    app: tauri::AppHandle,
    state: tauri::State<'_, TrayTimerState>,
    base_seconds: u64,
    run_start_ms: Option<u64>,
) {
    let last_inactive_at = app
        .state::<ScreenInactiveState>()
        .last_at
        .load(Ordering::SeqCst);
    {
        let Ok(mut timer) = state.inner.lock() else {
            return;
        };
        timer.base_seconds = base_seconds;
        // 熄屏前排队的旧同步命令不能重新启动已经暂停的原生计时。
        timer.run_start_ms = run_start_ms
            .map(|value| value.min(unix_time_ms()))
            .filter(|value| *value > last_inactive_at);
    }
    apply_tray_timer_title(&app, unix_time_ms());
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
    let app = tauri::Builder::default()
        .manage(WindowLayoutState::default())
        .manage(ScreenInactiveState::default())
        .manage(TrayTimerState::default())
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![
            sync_tray_timer,
            hide_main_window,
            set_main_window_size,
            main_window_expands_upward,
            snap_main_window_to_edge,
            latest_screen_inactive_at
        ])
        .on_page_load(|webview, payload| {
            #[cfg(target_os = "macos")]
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                let _ = install_webview_scroll_lock(webview);
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                install_screen_off_observers(app.handle().clone());
            }

            // 右键菜单(左键弹面板,右键退出)
            let quit = MenuItem::with_id(app, "quit", "退出 LavaTimer", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            // 托盘图标:icons/tray.png,白色+透明 → 模板图
            let icon = Image::from_path(app.path().resource_dir()?.join("icons/tray.png"))
                .or_else(|_| Image::from_path("icons/tray.png"))
                // 打包资源异常时仍可使用编译进二进制的图标，避免应用启动即退出。
                .or_else(|_| Image::from_bytes(include_bytes!("../icons/tray.png")))?;

            TrayIconBuilder::with_id("main-tray")
                .icon(icon)
                .icon_as_template(true) // 关键:自动适配深浅色菜单栏
                .title("Lava") // 即使模板图标被系统隐藏，仍保留可识别入口。
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
                                let _ = win.move_window(Position::TrayBottomCenter);
                                show_main_window(app);
                            }
                        }
                    }
                })
                .build(app)?;

            install_tray_timer_updater(app.handle().clone());

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
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen {
            has_visible_windows,
            ..
        } = event
        {
            if !has_visible_windows {
                show_main_window(app_handle);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{should_expand_upward, tray_elapsed_seconds, tray_timer_title, TrayTimer};

    #[test]
    fn native_tray_timer_advances_without_webview_ticks() {
        let timer = TrayTimer {
            base_seconds: 77 * 60,
            run_start_ms: Some(1_000),
        };

        assert_eq!(
            tray_elapsed_seconds(&timer, 66 * 60 * 1000 + 1_000),
            143 * 60
        );
        assert_eq!(tray_timer_title(&timer, 66 * 60 * 1000 + 1_000), "2:23");
    }

    #[test]
    fn paused_tray_timer_keeps_a_visible_app_title() {
        let timer = TrayTimer {
            base_seconds: 2 * 3600 + 22 * 60,
            run_start_ms: None,
        };

        assert_eq!(tray_timer_title(&timer, 123_000), "Lava");
    }

    #[test]
    fn window_expands_down_when_there_is_enough_space() {
        assert!(!should_expand_upward(100, 60, 560, 24, 900));
    }

    #[test]
    fn window_expands_up_near_the_screen_bottom() {
        assert!(should_expand_upward(800, 60, 560, 24, 900));
    }
}
