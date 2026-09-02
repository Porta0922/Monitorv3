#![cfg(windows)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::Arc;
use std::thread;
use winapi::shared::minwindef::{DWORD, LRESULT, UINT, WPARAM, LPARAM, LOWORD};
use winapi::shared::windef::{HMENU, HWND, POINT, HICON};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::{NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW};
use winapi::um::wingdi::{
    CreateBitmap, CreateDIBSection, DeleteObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use winapi::um::winuser::{
    self, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyIcon,
    DispatchMessageW, GetCursorPos, GetDC, GetMessageW, GetWindowLongPtrW,
    ICONINFO, IDI_APPLICATION, KillTimer, LoadIconW, MB_ICONINFORMATION,
    MB_OK, MessageBoxW,
    PostQuitMessage, RegisterClassW, ReleaseDC, SetForegroundWindow, SetTimer,
    SetWindowLongPtrW, TrackPopupMenu, TranslateMessage, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    WNDCLASSW, GWLP_USERDATA, CreateIconIndirect,
    WM_APP, WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WM_TIMER, CW_USEDEFAULT,
    MF_STRING, MF_SEPARATOR, WS_CAPTION, MSG,
};

use crate::web::WebState;

const WM_TRAYICON: UINT = WM_APP + 1;
const ID_TRAY: UINT = 1;
const CMD_OPEN_STATUS: usize = 1002;
const CMD_ABOUT: usize = 1005;
const TIMER_ICON_UPDATE: usize = 2001;

struct TrayContext {
    web_state: Arc<WebState>,
    icon_green: HICON,
    icon_red: HICON,
    popup: HMENU,
}

fn create_colored_icon(r: u8, g: u8, b: u8) -> HICON {
    unsafe {
        let hdc = GetDC(std::ptr::null_mut());

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as DWORD;
        bmi.bmiHeader.biWidth = 16;
        bmi.bmiHeader.biHeight = 16;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut pixels: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbm_color = CreateDIBSection(
            hdc, &mut bmi, DIB_RGB_COLORS, &mut pixels,
            std::ptr::null_mut(), 0,
        );

        if hbm_color.is_null() || pixels.is_null() {
            ReleaseDC(std::ptr::null_mut(), hdc);
            return LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
        }

        for y in 0..16i32 {
            for x in 0..16i32 {
                let dx = x - 7;
                let dy = y - 7;
                let inside = dx * dx + dy * dy <= 49;
                let idx = (y * 16 + x) as usize;
                let p = (pixels as *mut u32).add(idx);
                *p = if inside {
                    0xFF000000 | (r as u32) << 16 | (g as u32) << 8 | b as u32
                } else {
                    0
                };
            }
        }

        ReleaseDC(std::ptr::null_mut(), hdc);

        let hbm_mask = CreateBitmap(16, 16, 1, 1, std::ptr::null());

        let mut info = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hbm_mask as *mut _,
            hbmColor: hbm_color as *mut _,
        };
        let icon = CreateIconIndirect(&mut info);
        DeleteObject(hbm_mask as *mut _);
        DeleteObject(hbm_color as *mut _);

        if icon.is_null() {
            LoadIconW(std::ptr::null_mut(), IDI_APPLICATION)
        } else {
            icon
        }
    }
}

pub fn spawn_tray(state: Arc<WebState>) {
    thread::spawn(move || {
        unsafe {
            let hinstance = GetModuleHandleW(std::ptr::null());
            if hinstance.is_null() {
                return;
            }

            let class_name = to_wide("ActivityMonitorTrayClass");

            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(tray_wndproc),
                cbClsExtra: 0,
                cbWndExtra: std::mem::size_of::<isize>() as i32,
                hInstance: hinstance,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };

            if RegisterClassW(&wc) == 0 {
                return;
            }

            let window = CreateWindowExW(
                0,
                class_name.as_ptr(),
                std::ptr::null(),
                WS_CAPTION,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            );

            if window.is_null() {
                return;
            }

            let icon_green = create_colored_icon(35, 180, 30);
            let icon_red = create_colored_icon(220, 50, 40);

            let tip = to_wide("ActivityMonitor Agent");
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as DWORD;
            nid.hWnd = window;
            nid.uID = ID_TRAY;
            nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            nid.uCallbackMessage = WM_TRAYICON;
            nid.hIcon = icon_red;

            let copy_len = tip.len().min(128);
            let tip_dest = &mut nid.szTip[..copy_len];
            tip_dest.copy_from_slice(&tip[..copy_len]);

            Shell_NotifyIconW(NIM_ADD, &mut nid);

            let popup = CreatePopupMenu();
            winuser::AppendMenuW(popup, MF_STRING, CMD_OPEN_STATUS, to_wide("Abrir Estado").as_ptr());
            winuser::AppendMenuW(popup, MF_SEPARATOR, 0, std::ptr::null());
            winuser::AppendMenuW(popup, MF_STRING, CMD_ABOUT, to_wide("Acerca de").as_ptr());

            let ctx = Box::into_raw(Box::new(TrayContext {
                web_state: state,
                icon_green,
                icon_red,
                popup,
            }));
            SetWindowLongPtrW(window, GWLP_USERDATA, ctx as isize);

            SetTimer(window, TIMER_ICON_UPDATE, 2000, None);

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            KillTimer(window, TIMER_ICON_UPDATE);
            if !ctx.is_null() {
                let ctx_box = Box::from_raw(ctx);
                let mut del_nid: NOTIFYICONDATAW = std::mem::zeroed();
                del_nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as DWORD;
                del_nid.hWnd = window;
                del_nid.uID = ID_TRAY;
                Shell_NotifyIconW(NIM_DELETE, &mut del_nid);
                DestroyMenu(ctx_box.popup);
                DestroyIcon(ctx_box.icon_green);
                DestroyIcon(ctx_box.icon_red);
            }
        }
    });
}

unsafe extern "system" fn tray_wndproc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            let ev = lparam as UINT;
            if ev == WM_RBUTTONUP {
                let ctx_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TrayContext;
                if !ctx_ptr.is_null() {
                    let ctx = &*ctx_ptr;
                    if !ctx.popup.is_null() {
                        let mut pt: POINT = std::mem::zeroed();
                        GetCursorPos(&mut pt);
                        SetForegroundWindow(hwnd);
                        TrackPopupMenu(ctx.popup, TPM_LEFTALIGN | TPM_BOTTOMALIGN, pt.x, pt.y, 0, hwnd, std::ptr::null_mut());
                    }
                }
            } else if ev == WM_LBUTTONDBLCLK {
                let _ = open::that("http://localhost:9876");
            }
            0
        }
        WM_TIMER => {
            if wparam as usize == TIMER_ICON_UPDATE {
                let ctx_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TrayContext;
                if !ctx_ptr.is_null() {
                    let ctx = &*ctx_ptr;
                    let connected = ctx.web_state.connected.load(std::sync::atomic::Ordering::Relaxed);
                    let new_icon = if connected { ctx.icon_green } else { ctx.icon_red };

                    let mut mod_nid: NOTIFYICONDATAW = std::mem::zeroed();
                    mod_nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as DWORD;
                    mod_nid.hWnd = hwnd;
                    mod_nid.uID = ID_TRAY;
                    mod_nid.uFlags = NIF_ICON;
                    mod_nid.hIcon = new_icon;
                    Shell_NotifyIconW(NIM_MODIFY, &mut mod_nid);
                }
            }
            0
        }
        WM_COMMAND => {
            let cmd = LOWORD(wparam as u32) as usize;
            match cmd {
                CMD_OPEN_STATUS => {
                    let _ = open::that("http://localhost:9876");
                }
                CMD_ABOUT => {
                    let title = to_wide("ActivityMonitor Agent");
                    let version = env!("CARGO_PKG_VERSION");
                    let msg_text = format!(
                        "ActivityMonitor Agent v{}\n\nAgente de monitoreo y seguridad.",
                        version
                    );
                    let msg_wide = to_wide(&msg_text);
                    MessageBoxW(hwnd, msg_wide.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION);
                }
                _ => {}
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}
