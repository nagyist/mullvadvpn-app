//! Utilities for working with windows on Windows.
#![allow(clippy::undocumented_unsafe_blocks)] // Remove me if you dare.

use std::{os::windows::io::AsRawHandle, ptr, sync::Arc, thread};
use tokio::sync::broadcast;
use windows_sys::{
    Win32::{
        Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM},
        System::{LibraryLoader::GetModuleHandleW, Threading::GetThreadId},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA,
            GWLP_WNDPROC, GetMessageW, GetWindowLongPtrW, PBT_APMRESUMEAUTOMATIC,
            PBT_APMRESUMESUSPEND, PBT_APMSUSPEND, PostQuitMessage, PostThreadMessageW,
            SetWindowLongPtrW, TranslateMessage, WM_DESTROY, WM_POWERBROADCAST, WM_USER,
        },
    },
    w,
};

const CLASS_NAME: *const u16 = w!("STATIC");
const REQUEST_THREAD_SHUTDOWN: u32 = WM_USER + 1;

/// Handle for closing an associated window.
/// The window is not destroyed when this is dropped.
pub struct WindowCloseHandle {
    thread: Option<std::thread::JoinHandle<()>>,
}

impl WindowCloseHandle {
    /// Close the window and wait for the thread.
    pub fn close(&mut self) {
        if let Some(thread) = self.thread.take() {
            let thread_id = unsafe { GetThreadId(thread.as_raw_handle() as HANDLE) };
            unsafe { PostThreadMessageW(thread_id, REQUEST_THREAD_SHUTDOWN, 0, 0) };
            let _ = thread.join();
        }
    }
}

/// Creates a dummy window whose messages are handled by `wnd_proc`.
pub fn create_hidden_window<F: (Fn(HWND, u32, WPARAM, LPARAM) -> LRESULT) + Send + 'static>(
    wnd_proc: F,
) -> WindowCloseHandle {
    let join_handle = thread::spawn(move || {
        let dummy_window = unsafe {
            CreateWindowExW(
                0,
                CLASS_NAME,
                ptr::null_mut(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                GetModuleHandleW(ptr::null_mut()),
                ptr::null_mut(),
            )
        };

        // Move callback information to the heap.
        // This enables us to reach the callback through a "thin pointer".
        let raw_callback = Box::into_raw(Box::new(wnd_proc));

        unsafe {
            SetWindowLongPtrW(dummy_window, GWLP_USERDATA, raw_callback as isize);
            SetWindowLongPtrW(
                dummy_window,
                GWLP_WNDPROC,
                // Clippy does not like casting function pointers to anything but usize.
                // But this is correct, since the Windows API expects a signed int for pointer.
                window_procedure::<F> as usize as isize,
            );
        }

        let mut msg = unsafe { std::mem::zeroed() };

        loop {
            let status = unsafe { GetMessageW(&mut msg, 0, 0, 0) };

            if status < 0 {
                continue;
            }
            if status == 0 {
                break;
            }

            if msg.hwnd == 0 {
                if msg.message == REQUEST_THREAD_SHUTDOWN {
                    unsafe { DestroyWindow(dummy_window) };
                }
            } else {
                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        // Free callback.
        let _ = unsafe { Box::from_raw(raw_callback) };
    });

    WindowCloseHandle {
        thread: Some(join_handle),
    }
}

unsafe extern "system" fn window_procedure<F>(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT
where
    F: Fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
{
    if message == WM_DESTROY {
        unsafe { PostQuitMessage(0) };
        return 0;
    }
    let raw_callback = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) };
    if raw_callback != 0 {
        let typed_callback = unsafe { &mut *(raw_callback as *mut F) };
        return typed_callback(window, message, wparam, lparam);
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

/// Power management events
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum PowerManagementEvent {
    /// The system is resuming from sleep or hibernation
    /// irrespective of user activity.
    ResumeAutomatic,
    /// The system is resuming from sleep or hibernation
    /// due to user activity.
    ResumeSuspend,
    /// The computer is about to enter a suspended state.
    Suspend,
}

impl PowerManagementEvent {
    fn try_from_winevent(wparam: usize) -> Option<Self> {
        use PowerManagementEvent::*;
        match wparam as u32 {
            PBT_APMRESUMEAUTOMATIC => Some(ResumeAutomatic),
            PBT_APMRESUMESUSPEND => Some(ResumeSuspend),
            PBT_APMSUSPEND => Some(Suspend),
            _ => None,
        }
    }
}

/// Provides power management events to listeners
pub struct PowerManagementListener {
    _window: Arc<WindowScopedHandle>,
    rx: broadcast::Receiver<PowerManagementEvent>,
}

impl PowerManagementListener {
    /// Creates a new listener. This is expensive compared to cloning an existing instance.
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::broadcast::channel(16);

        let power_broadcast_callback = move |window, message, wparam, lparam| {
            if message == WM_POWERBROADCAST {
                if let Some(event) = PowerManagementEvent::try_from_winevent(wparam) {
                    if tx.send(event).is_err() {
                        log::error!("Stopping power management event monitor");
                        unsafe { PostQuitMessage(0) };
                        return 0;
                    }
                }
            }
            unsafe { DefWindowProcW(window, message, wparam, lparam) }
        };

        let window = create_hidden_window(power_broadcast_callback);

        Self {
            _window: Arc::new(WindowScopedHandle(window)),
            rx,
        }
    }

    /// Returns the next power event.
    pub async fn next(&mut self) -> Option<PowerManagementEvent> {
        loop {
            match self.rx.recv().await {
                Ok(event) => break Some(event),
                Err(broadcast::error::RecvError::Closed) => {
                    log::error!("Sender was unexpectedly dropped");
                    break None;
                }
                Err(broadcast::error::RecvError::Lagged(num_skipped)) => {
                    log::warn!("Skipped {num_skipped} power broadcast events");
                }
            }
        }
    }
}

impl Default for PowerManagementListener {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PowerManagementListener {
    fn clone(&self) -> Self {
        Self {
            _window: self._window.clone(),
            rx: self.rx.resubscribe(),
        }
    }
}

struct WindowScopedHandle(WindowCloseHandle);

impl Drop for WindowScopedHandle {
    fn drop(&mut self) {
        self.0.close();
    }
}
