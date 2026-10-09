use crate::desktop::AppState;
use std::sync::Arc;
use tauri::{Manager, WebviewWindow};

// Tauri/Tao caches the topmost flag. Reapplying the same value can be a no-op
// even when another Windows operation has removed WS_EX_TOPMOST.
pub fn apply_pin(card: &WebviewWindow, pinned: bool) -> Result<(), String> {
    card.set_always_on_top(pinned).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    native::set_topmost(card.hwnd().map_err(|e| e.to_string())?.0, pinned)?;
    Ok(())
}

pub fn show(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let pinned = state.card_pinned.lock().map_err(|_| "小卡片状态不可用")?;
    let card = app.get_webview_window("card").ok_or("小卡片窗口不可用")?;
    card.show().map_err(|e| e.to_string())?;
    card.unminimize().map_err(|e| e.to_string())?;
    card.set_focus().map_err(|e| e.to_string())?;
    apply_pin(&card, *pinned)
}

#[cfg(windows)]
pub fn repair_pin(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let pinned = state.card_pinned.lock().map_err(|_| "小卡片状态不可用")?;
    let card = app.get_webview_window("card").ok_or("小卡片窗口不可用")?;
    native::repair(card.hwnd().map_err(|e| e.to_string())?.0, *pinned)
}

#[cfg(windows)]
mod native {
    use std::ffi::c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        #[cfg_attr(target_pointer_width = "64", link_name = "GetWindowLongPtrW")]
        #[cfg_attr(target_pointer_width = "32", link_name = "GetWindowLongW")]
        fn window_style(hwnd: *mut c_void, index: i32) -> isize;
        fn IsWindowVisible(hwnd: *mut c_void) -> i32;
        fn IsIconic(hwnd: *mut c_void) -> i32;
        fn SetWindowPos(
            hwnd: *mut c_void,
            after: *mut c_void,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
    }
    pub fn is_topmost(hwnd: *mut c_void) -> bool {
        // GWL_EXSTYLE, WS_EX_TOPMOST.
        unsafe { window_style(hwnd, -20) & 0x00000008 != 0 }
    }
    pub fn repair(hwnd: *mut c_void, pinned: bool) -> Result<(), String> {
        if pinned
            && unsafe { IsWindowVisible(hwnd) != 0 && IsIconic(hwnd) == 0 }
            && !is_topmost(hwnd)
        {
            set_topmost(hwnd, true)?;
        }
        Ok(())
    }
    pub fn set_topmost(hwnd: *mut c_void, pinned: bool) -> Result<(), String> {
        let after = if pinned { -1isize } else { -2isize } as *mut c_void;
        // SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE: preserve geometry and focus.
        if unsafe { SetWindowPos(hwnd, after, 0, 0, 0, 0, 0x0013) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[repr(C)]
        struct WindowClass {
            style: u32,
            procedure: unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize,
            class_extra: i32,
            window_extra: i32,
            instance: *mut c_void,
            icon: *mut c_void,
            cursor: *mut c_void,
            background: *mut c_void,
            menu: *const u16,
            name: *const u16,
        }
        #[link(name = "user32")]
        unsafe extern "system" {
            fn RegisterClassW(class: *const WindowClass) -> u16;
            fn DefWindowProcW(
                hwnd: *mut c_void,
                message: u32,
                wparam: usize,
                lparam: isize,
            ) -> isize;
            fn CreateWindowExW(
                ex: u32,
                class: *const u16,
                title: *const u16,
                style: u32,
                x: i32,
                y: i32,
                width: i32,
                height: i32,
                parent: *mut c_void,
                menu: *mut c_void,
                instance: *mut c_void,
                param: *mut c_void,
            ) -> *mut c_void;
            fn DestroyWindow(hwnd: *mut c_void) -> i32;
            fn ShowWindow(hwnd: *mut c_void, command: i32) -> i32;
            fn GetForegroundWindow() -> *mut c_void;
            fn PeekMessageW(
                message: *mut Message,
                hwnd: *mut c_void,
                min: u32,
                max: u32,
                remove: u32,
            ) -> i32;
            fn DispatchMessageW(message: *const Message) -> isize;
        }
        #[repr(C)]
        struct Message {
            hwnd: *mut c_void,
            message: u32,
            wparam: usize,
            lparam: isize,
            time: u32,
            x: i32,
            y: i32,
            private: u32,
        }
        fn pump() {
            let mut message: Message = unsafe { std::mem::zeroed() };
            while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, 1) } != 0 {
                unsafe {
                    DispatchMessageW(&message);
                }
            }
        }
        fn wait_for_topmost(hwnd: *mut c_void, expected: bool) {
            for _ in 0..100 {
                pump();
                if is_topmost(hwnd) == expected {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert_eq!(is_topmost(hwnd), expected);
        }
        struct TestWindow(*mut c_void);
        impl Drop for TestWindow {
            fn drop(&mut self) {
                unsafe {
                    DestroyWindow(self.0);
                }
            }
        }
        #[test]
        fn recovers_native_topmost_loss_without_activating_or_reopening_window() {
            let class: Vec<u16> = "LocalTodoPinRegression\0".encode_utf16().collect();
            let null = std::ptr::null_mut();
            let definition = WindowClass {
                style: 0,
                procedure: DefWindowProcW,
                class_extra: 0,
                window_extra: 0,
                instance: null,
                icon: null,
                cursor: null,
                background: null,
                menu: std::ptr::null(),
                name: class.as_ptr(),
            };
            assert_ne!(unsafe { RegisterClassW(&definition) }, 0);
            let window = TestWindow(unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    class.as_ptr(),
                    0x00cf0000,
                    0,
                    0,
                    30,
                    30,
                    null,
                    null,
                    null,
                    null,
                )
            });
            assert!(!window.0.is_null());
            repair(window.0, true).unwrap();
            pump();
            wait_for_topmost(window.0, false); // A hidden card stays hidden.
            assert_eq!(unsafe { IsWindowVisible(window.0) }, 0);
            unsafe {
                ShowWindow(window.0, 5);
            } // SW_SHOW: initialize like an opened application window.
            set_topmost(window.0, true).unwrap();
            pump();
            wait_for_topmost(window.0, true);
            set_topmost(window.0, false).unwrap();
            pump(); // Simulate external flag loss.
            let other = TestWindow(unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    class.as_ptr(),
                    0x00cf0000,
                    50,
                    50,
                    30,
                    30,
                    null,
                    null,
                    null,
                    null,
                )
            });
            assert!(!other.0.is_null());
            unsafe {
                ShowWindow(other.0, 5);
            }
            pump();
            let foreground = unsafe { GetForegroundWindow() };
            assert_ne!(foreground, window.0);
            repair(window.0, true).unwrap();
            pump();
            wait_for_topmost(window.0, true);
            assert_eq!(unsafe { GetForegroundWindow() }, foreground);
            set_topmost(window.0, false).unwrap();
            pump();
            repair(window.0, false).unwrap(); // Respect an explicit unpin.
            wait_for_topmost(window.0, false);
            unsafe {
                ShowWindow(window.0, 6);
            } // SW_MINIMIZE
            repair(window.0, true).unwrap();
            pump();
            wait_for_topmost(window.0, false);
            assert_ne!(unsafe { IsIconic(window.0) }, 0);
            unsafe {
                ShowWindow(window.0, 0);
            } // SW_HIDE
            repair(window.0, true).unwrap();
            pump();
            assert_eq!(unsafe { IsWindowVisible(window.0) }, 0);
        }
    }
}
