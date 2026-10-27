//! Read-only scan of the Warframe process for the inventory JSON the game
//! receives at login (and refreshes on loading screens).
//!
//! No offsets, no hooks, no writes: readable memory regions are read with
//! `ReadProcessMemory` and searched for keys that occur once in the inventory
//! document (`"MiscItems":[`); the enclosing JSON object is then found by
//! brace-matching backwards to its `{` and forwards to its `}`, and parsed.
//! This is the same data Overwolf-based companions use. It is opt-in; the user
//! decides whether that is acceptable for their account.

use anyhow::{Context, Result};

const ANCHORS: &[&[u8]] = &[b"\"MiscItems\":[", b"\"RegularCredits\":"];
const MUST_CONTAIN: &[&[u8]] = &[b"\"MiscItems\"", b"\"Recipes\"", b"\"XPInfo\""];
/// Hard cap for one JSON candidate; a real inventory is 1-30 MB.
const MAX_DOC: usize = 96 << 20;
/// Flush the contiguous-region buffer beyond this to bound memory use.
const MAX_RUN: usize = 512 << 20;

#[cfg(windows)]
pub fn find_pid() -> Option<u32> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut found = None;
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                let len = e.szExeFile.iter().position(|c| *c == 0).unwrap_or(e.szExeFile.len());
                let name = String::from_utf16_lossy(&e.szExeFile[..len]).to_ascii_lowercase();
                if name == "warframe.x64.exe" || name == "warframe.exe" {
                    found = Some(e.th32ProcessID);
                    break;
                }
                if Process32NextW(snap, &mut e).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
        found
    }
}

/// Where the document was last found; checked first on the next scan.
static LAST_HIT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
const WINDOW: usize = 64 << 20;
const OVERLAP: usize = 32 << 20;

#[cfg(windows)]
pub fn read_inventory_json() -> Result<serde_json::Value> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows::Win32::System::Memory::{
        VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
        PAGE_GUARD, PAGE_NOACCESS, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

    let pid = find_pid().context("Warframe is not running")?;
    let h = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) }
        .context("cannot open the Warframe process (is it running as administrator?)")?;

    // 1. Contiguous readable runs.
    let mut runs: Vec<(usize, usize)> = Vec::new(); // (base, len)
    let mut addr = 0usize;
    loop {
        let mut mbi = MEMORY_BASIC_INFORMATION::default();
        let n = unsafe {
            VirtualQueryEx(h, Some(addr as *const _), &mut mbi, std::mem::size_of::<MEMORY_BASIC_INFORMATION>())
        };
        if n == 0 {
            break;
        }
        let base = mbi.BaseAddress as usize;
        let size = mbi.RegionSize;
        let prot = mbi.Protect.0;
        let readable = mbi.State == MEM_COMMIT
            && prot & PAGE_GUARD.0 == 0
            && prot & PAGE_NOACCESS.0 == 0
            && prot
                & (PAGE_READONLY.0 | PAGE_READWRITE.0 | PAGE_WRITECOPY.0 | PAGE_EXECUTE_READ.0 | PAGE_EXECUTE_READWRITE.0)
                != 0;
        if readable && size > 0 {
            match runs.last_mut() {
                Some((b, l)) if *b + *l == base => *l += size,
                _ => runs.push((base, size)),
            }
        }
        addr = match base.checked_add(size) {
            Some(a) if a > addr => a,
            _ => break,
        };
    }

    // 2. Overlapping windows, hint window first.
    let mut windows: Vec<(usize, usize)> = Vec::new();
    for (base, len) in &runs {
        let mut off = 0usize;
        loop {
            let l = (*len - off).min(WINDOW);
            windows.push((base + off, l));
            if off + l >= *len {
                break;
            }
            off += OVERLAP;
        }
    }
    let hint = LAST_HIT.load(Ordering::Relaxed);
    if hint != 0 {
        windows.sort_by_key(|(b, l)| if (*b..*b + *l).contains(&hint) { 0 } else { 1 });
    }

    // 3. Parallel search with early exit. ReadProcessMemory is safe to call concurrently.
    struct H(HANDLE);
    unsafe impl Send for H {}
    unsafe impl Sync for H {}
    let hh = std::sync::Arc::new(H(h));
    let found: std::sync::Arc<Mutex<Option<(usize, Vec<u8>)>>> = Default::default();
    let stop = std::sync::Arc::new(AtomicBool::new(false));
    let queue = std::sync::Arc::new(Mutex::new(windows.into_iter()));
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 8);
    let threads: Vec<_> = (0..workers)
        .map(|_| {
            let (hh, found, stop, queue) = (hh.clone(), found.clone(), stop.clone(), queue.clone());
            std::thread::spawn(move || {
                let mut buf: Vec<u8> = Vec::with_capacity(WINDOW);
                loop {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }
                    let Some((base, len)) = queue.lock().unwrap().next() else { return };
                    buf.clear();
                    buf.resize(len, 0);
                    let mut got = 0usize;
                    let ok = unsafe {
                        ReadProcessMemory(hh.0, base as *const _, buf.as_mut_ptr() as *mut _, len, Some(&mut got)).is_ok()
                    };
                    if !ok || got == 0 {
                        continue;
                    }
                    if let Some(doc) = best_candidate(&buf[..got]) {
                        let mut f = found.lock().unwrap();
                        if f.as_ref().map(|(_, d)| d.len() < doc.len()).unwrap_or(true) {
                            *f = Some((base, doc));
                        }
                        stop.store(true, Ordering::Relaxed);
                        return;
                    }
                }
            })
        })
        .collect();
    for t in threads {
        let _ = t.join();
    }
    unsafe {
        let _ = CloseHandle(h);
    }

    let (base, doc) = found
        .lock()
        .unwrap()
        .take()
        .context("inventory JSON not found in game memory (it is only intact for a while after login or a loading screen)")?;
    LAST_HIT.store(base, Ordering::Relaxed);
    serde_json::from_slice(&doc).context("inventory JSON did not parse")
}

/// Run `cb` over every contiguous readable run of the Warframe process. Debug aid.
#[cfg(windows)]
pub fn walk_memory(mut cb: impl FnMut(usize, &[u8])) -> Result<()> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows::Win32::System::Memory::{VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

    let pid = find_pid().context("Warframe is not running")?;
    let h = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) }?;
    let mut addr: usize = 0;
    loop {
        let mut mbi = MEMORY_BASIC_INFORMATION::default();
        let n = unsafe { VirtualQueryEx(h, Some(addr as *const _), &mut mbi, std::mem::size_of::<MEMORY_BASIC_INFORMATION>()) };
        if n == 0 {
            break;
        }
        let base = mbi.BaseAddress as usize;
        let size = mbi.RegionSize;
        let prot = mbi.Protect.0;
        if mbi.State == MEM_COMMIT && prot & PAGE_GUARD.0 == 0 && prot & PAGE_NOACCESS.0 == 0 && size < (1 << 32) {
            let mut buf = vec![0u8; size];
            let mut got = 0usize;
            if unsafe { ReadProcessMemory(h, base as *const _, buf.as_mut_ptr() as *mut _, size, Some(&mut got)) }.is_ok() && got > 0 {
                cb(base, &buf[..got]);
            }
        }
        addr = match base.checked_add(size) {
            Some(a) if a > addr => a,
            _ => break,
        };
    }
    unsafe {
        let _ = CloseHandle(h);
    }
    Ok(())
}

/// Sequential single-buffer variant, kept for reference/debugging.
#[cfg(windows)]
#[allow(dead_code)]
fn scan(h: windows::Win32::Foundation::HANDLE) -> Result<serde_json::Value> {
    use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows::Win32::System::Memory::{
        VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
        PAGE_GUARD, PAGE_NOACCESS, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY,
    };

    let mut addr: usize = 0;
    let mut best: Option<Vec<u8>> = None;
    let mut run: Vec<u8> = Vec::new();
    let mut run_end: usize = 0;

    let flush = |run: &mut Vec<u8>, best: &mut Option<Vec<u8>>| {
        if !run.is_empty() {
            if let Some(doc) = best_candidate(run) {
                if best.as_ref().map(|b| b.len() < doc.len()).unwrap_or(true) {
                    *best = Some(doc);
                }
            }
            run.clear();
        }
    };

    loop {
        let mut mbi = MEMORY_BASIC_INFORMATION::default();
        let n = unsafe {
            VirtualQueryEx(h, Some(addr as *const _), &mut mbi, std::mem::size_of::<MEMORY_BASIC_INFORMATION>())
        };
        if n == 0 {
            break;
        }
        let base = mbi.BaseAddress as usize;
        let size = mbi.RegionSize;
        let prot = mbi.Protect.0;
        let readable = mbi.State == MEM_COMMIT
            && prot & PAGE_GUARD.0 == 0
            && prot & PAGE_NOACCESS.0 == 0
            && prot
                & (PAGE_READONLY.0 | PAGE_READWRITE.0 | PAGE_WRITECOPY.0 | PAGE_EXECUTE_READ.0 | PAGE_EXECUTE_READWRITE.0)
                != 0;

        if readable && size > 0 && size < (1 << 32) {
            if base != run_end || run.len() > MAX_RUN {
                flush(&mut run, &mut best);
            }
            let mut buf = vec![0u8; size];
            let mut got = 0usize;
            let ok = unsafe {
                ReadProcessMemory(h, base as *const _, buf.as_mut_ptr() as *mut _, size, Some(&mut got)).is_ok()
            };
            if ok && got > 0 {
                run.extend_from_slice(&buf[..got]);
                run_end = base + got;
            } else {
                flush(&mut run, &mut best);
            }
        } else {
            flush(&mut run, &mut best);
        }

        addr = match base.checked_add(size) {
            Some(a) if a > addr => a,
            _ => break,
        };
    }
    flush(&mut run, &mut best);

    let doc = best.context("inventory JSON not found in game memory (log in and wait for the orbiter to load)")?;
    serde_json::from_slice(&doc).context("inventory JSON did not parse")
}

/// Largest well-formed inventory document in `buf`.
fn best_candidate(buf: &[u8]) -> Option<Vec<u8>> {
    let mut best: Option<&[u8]> = None;
    for anchor in ANCHORS {
        for pos in memchr::memmem::find_iter(buf, anchor) {
            let Some(start) = find_start(buf, pos) else { continue };
            let Some(end) = match_object(&buf[start..]) else { continue };
            let cand = &buf[start..start + end];
            if !MUST_CONTAIN.iter().all(|k| memchr::memmem::find(cand, k).is_some()) {
                continue;
            }
            if best.map(|b| b.len() < cand.len()).unwrap_or(true) {
                best = Some(cand);
            }
        }
    }
    best.map(|b| b.to_vec())
}

/// Walk backwards from a key at `pos` (which is at depth 1 inside the document)
/// to the `{` that opens the top-level object. Strings are skipped by quote
/// toggling; a backslash before a quote marks an escaped quote.
fn find_start(buf: &[u8], pos: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_str = false;
    let lo = pos.saturating_sub(MAX_DOC);
    let mut i = pos;
    while i > lo {
        i -= 1;
        let c = buf[i];
        if in_str {
            if c == b'"' && (i == 0 || buf[i - 1] != b'\\') {
                in_str = false;
            }
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'}' | b']' => depth += 1,
            b'{' | b'[' => {
                if depth == 0 {
                    return (c == b'{').then_some(i);
                }
                depth -= 1;
            }
            0 => return None,
            _ => {}
        }
    }
    None
}

/// Length of the JSON object starting at `s[0] == b'{'`, or None if unterminated.
fn match_object(s: &[u8]) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, &c) in s.iter().enumerate().take(MAX_DOC) {
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            // Binary garbage means this was not a JSON document.
            0 => return None,
            _ => {}
        }
    }
    None
}

#[cfg(not(windows))]
pub fn find_pid() -> Option<u32> {
    None
}

/// Process id of the window in front; the overlay hides when that isn't the game.
#[cfg(windows)]
pub fn foreground_pid() -> Option<u32> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    unsafe {
        let h = GetForegroundWindow();
        if h.0.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, Some(&mut pid as *mut u32));
        (pid != 0).then_some(pid)
    }
}

/// Show the overlay without taking the foreground from the game, and keep it
/// from being activated by clicks (`no_activate`; off in edit mode).
#[cfg(windows)]
pub fn overlay_window(hwnd: isize, show: bool, no_activate: bool) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE, SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE,
    };
    unsafe {
        let h = HWND(hwnd as *mut core::ffi::c_void);
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        let flag = WS_EX_NOACTIVATE.0 as isize;
        SetWindowLongPtrW(h, GWL_EXSTYLE, if no_activate { ex | flag } else { ex & !flag });
        if show {
            let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(h, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }
}

#[cfg(not(windows))]
pub fn overlay_window(_hwnd: isize, _show: bool, _no_activate: bool) {}

#[cfg(not(windows))]
pub fn foreground_pid() -> Option<u32> {
    None
}
#[cfg(not(windows))]
pub fn read_inventory_json() -> Result<serde_json::Value> {
    anyhow::bail!("memory scan only on Windows")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_document_in_noise() {
        // Keys before the anchor, nested arrays/objects, braces inside strings.
        let doc = br#"{"Skins":[{"ItemType":"/x","ItemId":{"$oid":"1"}}],"RegularCredits":12,"MiscItems":[{"ItemType":"/a","ItemCount":1}],"Recipes":[],"XPInfo":[],"s":"br{ace\"}"}"#;
        let mut buf = vec![0u8; 100];
        buf.extend_from_slice(b"junk{\"RegularCredits\":1}more");
        buf.extend_from_slice(doc);
        buf.extend_from_slice(&[0u8; 50]);
        let got = best_candidate(&buf).unwrap();
        assert_eq!(got, doc.to_vec());
        serde_json::from_slice::<serde_json::Value>(&got).unwrap();
    }
}

/// Bring the game's main window to the front (after the overlay editor, which
/// had to take focus, closes). Returns whether a window was found.
#[cfg(windows)]
pub fn focus_game() -> bool {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    let Some(pid) = find_pid() else { return false };
    struct Ctx {
        pid: u32,
        hwnd: Option<HWND>,
    }
    unsafe extern "system" fn cb(h: HWND, l: LPARAM) -> BOOL {
        let ctx = &mut *(l.0 as *mut Ctx);
        let mut p = 0u32;
        GetWindowThreadProcessId(h, Some(&mut p as *mut u32));
        if p == ctx.pid && IsWindowVisible(h).as_bool() {
            ctx.hwnd = Some(h);
            return BOOL(0);
        }
        BOOL(1)
    }
    let mut ctx = Ctx { pid, hwnd: None };
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut ctx as *mut Ctx as isize));
        if let Some(h) = ctx.hwnd {
            let _ = ShowWindow(h, SW_RESTORE);
            return SetForegroundWindow(h).as_bool();
        }
    }
    false
}

#[cfg(not(windows))]
pub fn focus_game() -> bool {
    false
}
