//! Records mouse and keyboard activity (positions, clicks, scrolls, key presses) with timestamps.
//! The recording never stores *which* keys were pressed, only that one was, which is all auto-zoom needs.

use super::Shared;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use velo_director::{Button, InputEvent, InputKind, Point};
use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetCursorPos, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN,
};

/// The hook callbacks are plain functions with no user data, so the active recording is found here.
struct Sink {
    shared: Arc<Shared>,
    origin: Point,
}

fn sink() -> &'static Mutex<Option<Sink>> {
    static SINK: OnceLock<Mutex<Option<Sink>>> = OnceLock::new();
    SINK.get_or_init(|| Mutex::new(None))
}

fn record(kind: InputKind, at: Option<POINT>) {
    let guard = sink().lock();
    let Some(s) = guard.as_ref() else { return };
    let (t_ms, paused) = {
        let clock = s.shared.clock.lock();
        (clock.active_ms(Instant::now()), clock.is_paused())
    };
    if paused {
        return;
    }
    let p = at.or_else(cursor).unwrap_or_default();
    s.shared.events.lock().push(InputEvent { t_ms, x: p.x as f64 - s.origin.x, y: p.y as f64 - s.origin.y, kind });
}

fn cursor() -> Option<POINT> {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p) }.ok().map(|_| p)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let kind = match wparam.0 as u32 {
            WM_LBUTTONDOWN => Some(InputKind::Down { button: Button::Left }),
            WM_LBUTTONUP => Some(InputKind::Up { button: Button::Left }),
            WM_RBUTTONDOWN => Some(InputKind::Down { button: Button::Right }),
            WM_RBUTTONUP => Some(InputKind::Up { button: Button::Right }),
            WM_MBUTTONDOWN => Some(InputKind::Down { button: Button::Middle }),
            WM_MBUTTONUP => Some(InputKind::Up { button: Button::Middle }),
            WM_MOUSEWHEEL => Some(InputKind::Scroll { dx: 0.0, dy: ((info.mouseData >> 16) as i16) as f64 }),
            _ => None,
        };
        if let Some(kind) = kind {
            record(kind, Some(info.pt));
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN) {
        record(InputKind::Key, None);
    }
    CallNextHookEx(None, code, wparam, lparam)
}

pub struct Input {
    hook_thread: Option<(JoinHandle<()>, u32)>,
    poll_thread: Option<JoinHandle<()>>,
    shared: Arc<Shared>,
}

pub fn start(shared: Arc<Shared>, origin: Point) -> Input {
    *sink().lock() = Some(Sink { shared: shared.clone(), origin });

    // Hooks need a thread that pumps messages.
    let thread_id = Arc::new(AtomicU32::new(0));
    let id_for_thread = thread_id.clone();
    let hook = thread::spawn(move || unsafe {
        id_for_thread.store(GetCurrentThreadId(), Ordering::Release);
        let module = GetModuleHandleW(None).ok().map(|m| windows::Win32::Foundation::HINSTANCE(m.0));
        let mouse: Option<HHOOK> = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0).ok();
        let keyboard: Option<HHOOK> = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0).ok();
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
        if let Some(h) = mouse {
            let _ = UnhookWindowsHookEx(h);
        }
        if let Some(h) = keyboard {
            let _ = UnhookWindowsHookEx(h);
        }
    });
    while thread_id.load(Ordering::Acquire) == 0 {
        thread::sleep(Duration::from_millis(1));
    }

    // Mouse movement is sampled rather than hooked: cheaper, and 120 Hz is smooth enough to replay.
    let poll_shared = shared.clone();
    let poll = thread::spawn(move || {
        let mut last = POINT { x: i32::MIN, y: i32::MIN };
        while !poll_shared.stop.load(Ordering::Relaxed) {
            if let Some(p) = cursor() {
                if (p.x, p.y) != (last.x, last.y) {
                    last = p;
                    record(InputKind::Move, Some(p));
                }
            }
            thread::sleep(Duration::from_millis(8));
        }
    });

    Input { hook_thread: Some((hook, thread_id.load(Ordering::Acquire))), poll_thread: Some(poll), shared }
}

impl Input {
    pub fn stop(mut self) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(t) = self.poll_thread.take() {
            let _ = t.join();
        }
        if let Some((t, id)) = self.hook_thread.take() {
            unsafe {
                let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
            let _ = t.join();
        }
        *sink().lock() = None;
    }
}
