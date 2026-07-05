//! macOS 顶栏：启用紧凑交通灯（类似备忘录），并在布局变化后重应用

#[cfg(target_os = "macos")]
pub fn configure(window: &tauri::WebviewWindow) {
    let window = window.clone();
    let _ = window.run_on_main_thread(move || {
        if let Err(err) = apply(&window) {
            eprintln!("macOS titlebar configure failed: {err}");
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn configure(_window: &tauri::WebviewWindow) {}

#[cfg(target_os = "macos")]
fn apply(window: &tauri::WebviewWindow) -> Result<(), String> {
    use objc2_app_kit::{NSWindow, NSWindowButton};
    use objc2_foundation::MainThreadMarker;

    let ptr = window.ns_window().map_err(|e| e.to_string())?;
    let mtm = MainThreadMarker::new().ok_or("must run on main thread")?;

    unsafe {
        let ns_window = &*(ptr as *const NSWindow);
        if let Some(close_btn) = ns_window.standardWindowButton(NSWindowButton::CloseButton, mtm) {
            if let Some(titlebar) = close_btn.superview() {
                titlebar.setPrefersCompactControlSizeMetrics(true);
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn attach_listeners(window: &tauri::WebviewWindow) {
    let window_for_events = window.clone();
    window.on_window_event(move |event| {
        use tauri::WindowEvent;
        if matches!(
            event,
            WindowEvent::Resized(_)
                | WindowEvent::ThemeChanged(_)
                | WindowEvent::Focused(true)
                | WindowEvent::ScaleFactorChanged { .. }
        ) {
            configure(&window_for_events);
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn attach_listeners(_window: &tauri::WebviewWindow) {}

#[cfg(target_os = "macos")]
pub fn schedule_delayed_configure(window: &tauri::WebviewWindow) {
    let window = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        configure(&window);
    });
}

#[cfg(not(target_os = "macos"))]
pub fn schedule_delayed_configure(_window: &tauri::WebviewWindow) {}
