//! Top-level windows of another process (AUD-P1-5: one preview window, brought to the front
//! instead of opening a second one).

use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SetForegroundWindow, ShowWindow, SW_RESTORE,
};

struct Search {
    pid: u32,
    found: HWND,
}

unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the `Search` passed to `EnumWindows` below, alive for the whole call.
    let s = unsafe { &mut *(lparam as *mut Search) };
    let mut pid = 0u32;
    // SAFETY: plain query on a window handle given by EnumWindows.
    unsafe { GetWindowThreadProcessId(hwnd, &raw mut pid) };
    // SAFETY: as above.
    if pid == s.pid && unsafe { IsWindowVisible(hwnd) } != 0 {
        s.found = hwnd;
        return 0; // stop
    }
    1
}

/// Brings the first visible top-level window of process `pid` to the front (restored if it
/// was minimised). Returns whether such a window exists.
#[must_use]
pub fn focus_window_of(pid: u32) -> bool {
    let mut s = Search {
        pid,
        found: std::ptr::null_mut(),
    };
    // SAFETY: `visit` only reads `s` through the pointer while EnumWindows runs.
    unsafe { EnumWindows(Some(visit), (&raw mut s) as LPARAM) };
    if s.found.is_null() {
        return false;
    }
    // SAFETY: a window handle just returned by EnumWindows; failures are harmless.
    unsafe {
        if IsIconic(s.found) != 0 {
            ShowWindow(s.found, SW_RESTORE);
        }
        SetForegroundWindow(s.found);
    }
    true
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the `Vec` passed to `EnumWindows` in `visible_titles`, alive for the
    // whole call.
    let out = unsafe { &mut *(lparam as *mut Vec<(u32, String)>) };
    // SAFETY: plain queries on a window handle given by EnumWindows.
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }
    let mut buf = [0u16; 512];
    // SAFETY: `buf` is writable for its full length, which is passed as the limit.
    let n = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), 512) };
    let Ok(n) = usize::try_from(n) else {
        return 1;
    };
    if n == 0 {
        return 1;
    }
    let mut pid = 0u32;
    // SAFETY: as above.
    unsafe { GetWindowThreadProcessId(hwnd, &raw mut pid) };
    out.push((pid, String::from_utf16_lossy(&buf[..n.min(512)])));
    1
}

/// The visible top-level windows that have a title: `(process id, title)`. CRT-D uses it to see
/// whether an editor opened the project folder (its window title names the folder).
#[must_use]
pub fn visible_titles() -> Vec<(u32, String)> {
    let mut out: Vec<(u32, String)> = Vec::new();
    // SAFETY: `collect` only touches `out` through the pointer while EnumWindows runs.
    unsafe { EnumWindows(Some(collect), (&raw mut out) as LPARAM) };
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_have_text() {
        // a headless test run may have no windows at all; every listed one has a title
        assert!(visible_titles().iter().all(|(_, t)| !t.is_empty()));
    }

    #[test]
    fn a_process_without_windows_is_not_found() {
        // the test runner has no visible top-level window; pid 0 never has one
        assert!(!focus_window_of(0));
    }
}
