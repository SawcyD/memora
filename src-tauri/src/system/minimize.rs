//! Windows minimize/restore event bridge.
//!
//! The WinEvent callback does no process work. It only copies the window and
//! pid into a channel; the delayed policy runs on Memora's worker thread.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Minimized,
    Restored,
}

#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub kind: EventKind,
    /// Raw HWND value. A number is Send; the Windows wrapper contains a raw
    /// pointer and should not cross threads directly.
    pub hwnd: isize,
    pub pid: u32,
}

#[cfg(windows)]
mod imp {
    use super::{Event, EventKind};
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::sync::mpsc::Sender;
    use std::sync::{Mutex, OnceLock};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowThreadProcessId, IsIconic, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART,
        WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
    };

    static SENDER: OnceLock<Mutex<Option<Sender<Event>>>> = OnceLock::new();
    static HOOK: AtomicIsize = AtomicIsize::new(0);

    unsafe extern "system" fn callback(
        _hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        _id_object: i32,
        _id_child: i32,
        _event_thread: u32,
        _event_time: u32,
    ) {
        if hwnd.0.is_null() {
            return;
        }
        let kind = match event {
            EVENT_SYSTEM_MINIMIZESTART => EventKind::Minimized,
            EVENT_SYSTEM_MINIMIZEEND => EventKind::Restored,
            _ => return,
        };
        let mut pid = 0;
        // SAFETY: pid is an owned out-parameter and hwnd came from Windows.
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == 0 {
            return;
        }
        if let Some(sender) = SENDER
            .get()
            .and_then(|slot| slot.lock().ok())
            .and_then(|sender| sender.clone())
        {
            let _ = sender.send(Event {
                kind,
                hwnd: hwnd.0 as isize,
                pid,
            });
        }
    }

    /// Installs one out-of-process hook on the Tauri event-loop thread.
    pub fn install(sender: Sender<Event>) -> Result<(), String> {
        let slot = SENDER.get_or_init(|| Mutex::new(None));
        *slot
            .lock()
            .map_err(|_| "Minimize event channel is unavailable")? = Some(sender);

        if HOOK.load(Ordering::Acquire) != 0 {
            return Ok(());
        }
        // SAFETY: the callback has the required ABI and remains valid for the
        // lifetime of the process. Tauri's Windows message loop dispatches the
        // out-of-context hook callbacks on this thread.
        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_MINIMIZESTART,
                EVENT_SYSTEM_MINIMIZEEND,
                None,
                Some(callback),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };
        if hook.0.is_null() {
            return Err("Windows could not install the minimize event hook".into());
        }
        HOOK.store(hook.0 as isize, Ordering::Release);
        Ok(())
    }

    pub fn is_minimized(hwnd: isize) -> bool {
        if hwnd == 0 {
            return false;
        }
        // SAFETY: the handle is only used for a read-only validity/state query.
        unsafe { IsIconic(HWND(hwnd as *mut std::ffi::c_void)).as_bool() }
    }
}

#[cfg(windows)]
pub use imp::{install, is_minimized};

#[cfg(not(windows))]
pub fn install(_sender: std::sync::mpsc::Sender<Event>) -> Result<(), String> {
    Err("Minimize rules are only available on Windows".into())
}

#[cfg(not(windows))]
pub fn is_minimized(_hwnd: isize) -> bool {
    false
}
