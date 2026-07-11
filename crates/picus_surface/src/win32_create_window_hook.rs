//! MinHook trampoline on `user32!CreateWindowExW` to inject
//! `WS_EX_NOREDIRECTIONBITMAP` at HWND creation for DirectComposition surfaces.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use windows_sys::Win32::Foundation::{HINSTANCE, HWND};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    HMENU, WINDOW_EX_STYLE, WINDOW_STYLE, WS_EX_NOREDIRECTIONBITMAP,
};

static FORCE_NO_REDIRECTION: AtomicBool = AtomicBool::new(false);
static ORIG: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(core::ptr::null_mut());
static INSTALL_ONCE: OnceLock<HookInstallResult> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookInstallResult {
    Installed,
    Failed,
}

type CreateWindowExWFn = unsafe extern "system" fn(
    WINDOW_EX_STYLE,
    *const u16,
    *const u16,
    WINDOW_STYLE,
    i32,
    i32,
    i32,
    i32,
    HWND,
    HMENU,
    HINSTANCE,
    *const core::ffi::c_void,
) -> HWND;

/// Enable/disable creation-time `WS_EX_NOREDIRECTIONBITMAP` injection.
///
/// Call before Bevy/winit creates the HWND (before `App::run()`).
pub fn set_force_no_redirection_bitmap_on_create(enable: bool) {
    if enable {
        let result = ensure_create_window_ex_w_hook();
        if result == HookInstallResult::Failed {
            tracing::error!(
                "CreateWindowExW hook install failed; transparent windows may stay white"
            );
        }
    }
    FORCE_NO_REDIRECTION.store(enable, Ordering::SeqCst);
}

/// Whether creation-time injection is currently enabled.
#[must_use]
pub fn force_no_redirection_bitmap_on_create() -> bool {
    FORCE_NO_REDIRECTION.load(Ordering::SeqCst)
}

/// Install the MinHook trampoline on `user32!CreateWindowExW` (idempotent).
pub fn ensure_create_window_ex_w_hook() -> HookInstallResult {
    *INSTALL_ONCE.get_or_init(install_minhook)
}

fn install_minhook() -> HookInstallResult {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

    let user32: Vec<u16> = "user32.dll\0".encode_utf16().collect();
    let module = unsafe { GetModuleHandleW(user32.as_ptr()) };
    if module.is_null() {
        tracing::error!("GetModuleHandleW(user32.dll) failed");
        return HookInstallResult::Failed;
    }

    let Some(target) =
        (unsafe { GetProcAddress(module, windows_sys::core::s!("CreateWindowExW")) })
    else {
        tracing::error!("GetProcAddress(CreateWindowExW) failed");
        return HookInstallResult::Failed;
    };

    let orig = match unsafe {
        minhook::MinHook::create_hook(target as *mut _, hooked_create_window_ex_w as *mut _)
    } {
        Ok(ptr) => ptr,
        Err(status) => {
            tracing::error!(?status, "MinHook::create_hook(CreateWindowExW) failed");
            return HookInstallResult::Failed;
        }
    };

    ORIG.store(orig, Ordering::SeqCst);

    if let Err(status) = unsafe { minhook::MinHook::enable_all_hooks() } {
        tracing::error!(?status, "MinHook::enable_all_hooks failed");
        return HookInstallResult::Failed;
    }

    tracing::debug!("CreateWindowExW MinHook trampoline enabled");
    HookInstallResult::Installed
}

unsafe extern "system" fn hooked_create_window_ex_w(
    mut dw_ex_style: WINDOW_EX_STYLE,
    lp_class_name: *const u16,
    lp_window_name: *const u16,
    dw_style: WINDOW_STYLE,
    x: i32,
    y: i32,
    n_width: i32,
    n_height: i32,
    hwnd_parent: HWND,
    h_menu: HMENU,
    h_instance: HINSTANCE,
    lp_param: *const core::ffi::c_void,
) -> HWND {
    if FORCE_NO_REDIRECTION.load(Ordering::SeqCst) {
        dw_ex_style |= WS_EX_NOREDIRECTIONBITMAP;
    }

    let orig_ptr = ORIG.load(Ordering::SeqCst);
    if orig_ptr.is_null() {
        return core::ptr::null_mut();
    }
    let orig: CreateWindowExWFn = unsafe { core::mem::transmute(orig_ptr) };
    unsafe {
        orig(
            dw_ex_style,
            lp_class_name,
            lp_window_name,
            dw_style,
            x,
            y,
            n_width,
            n_height,
            hwnd_parent,
            h_menu,
            h_instance,
            lp_param,
        )
    }
}
