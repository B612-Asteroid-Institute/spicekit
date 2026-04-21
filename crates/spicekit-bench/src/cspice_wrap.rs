//! Safe, single-threaded wrappers around the CSpice functions this
//! bench needs as a parity oracle.
//!
//! CSpice is not thread-safe (kernel pool, error stack, SPK segment
//! cache are all global state) so every entry point acquires a
//! process-wide `Mutex`. First use flips CSpice into `RETURN` error
//! mode so a bad kernel surfaces as `Err` rather than aborting the
//! bench process.
//!
//! Mirrors the pattern used in adam-core's `adam_core_rs_spice` crate;
//! kept local here so spicekit-bench has no cross-repo path dep.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, Once};

use cspice_sys::{
    bodc2n_c, bodn2c_c, erract_c, failed_c, furnsh_c, getmsg_c, pxform_c, reset_c, spkez_c,
    sxform_c, unload_c, SpiceBoolean, SpiceDouble, SpiceInt,
};

static CSPICE_LOCK: Mutex<()> = Mutex::new(());
static ERRACT_INIT: Once = Once::new();

#[derive(Debug, Clone)]
pub struct SpiceError {
    pub short: String,
    pub long: String,
}

impl std::fmt::Display for SpiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.long.is_empty() {
            write!(f, "{}", self.short)
        } else {
            write!(f, "{}: {}", self.short, self.long)
        }
    }
}

impl std::error::Error for SpiceError {}

fn ensure_return_on_error() {
    ERRACT_INIT.call_once(|| {
        let op = CString::new("SET").unwrap();
        let action = CString::new("RETURN").unwrap();
        let action_len = (action.as_bytes().len() + 1) as SpiceInt;
        unsafe {
            erract_c(
                op.as_ptr() as *mut c_char,
                action_len,
                action.as_ptr() as *mut c_char,
            );
        }
    });
}

fn read_spice_message(option: &str, len: usize) -> String {
    let option_c = CString::new(option).unwrap();
    let mut buf = vec![0 as c_char; len];
    unsafe {
        getmsg_c(
            option_c.as_ptr() as *mut c_char,
            len as SpiceInt,
            buf.as_mut_ptr(),
        );
    }
    unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

fn check_and_reset() -> Result<(), SpiceError> {
    if unsafe { failed_c() } == 0 {
        return Ok(());
    }
    let short = read_spice_message("SHORT", 64);
    let long = read_spice_message("LONG", 1842);
    unsafe { reset_c() };
    Err(SpiceError { short, long })
}

fn to_cstring(s: &str) -> Result<CString, SpiceError> {
    CString::new(s).map_err(|e| SpiceError {
        short: "INVALID_STRING".to_string(),
        long: format!("string contains NUL byte: {e}"),
    })
}

pub fn furnsh(path: &str) -> Result<(), SpiceError> {
    let _guard = CSPICE_LOCK.lock().unwrap();
    ensure_return_on_error();
    let c_path = to_cstring(path)?;
    unsafe { furnsh_c(c_path.as_ptr() as *mut c_char) };
    check_and_reset()
}

pub fn unload(path: &str) -> Result<(), SpiceError> {
    let _guard = CSPICE_LOCK.lock().unwrap();
    ensure_return_on_error();
    let c_path = to_cstring(path)?;
    unsafe { unload_c(c_path.as_ptr() as *mut c_char) };
    check_and_reset()
}

pub fn spkez(
    target: i32,
    et: f64,
    frame: &str,
    abcorr: &str,
    observer: i32,
) -> Result<([f64; 6], f64), SpiceError> {
    let _guard = CSPICE_LOCK.lock().unwrap();
    ensure_return_on_error();
    let c_frame = to_cstring(frame)?;
    let c_abcorr = to_cstring(abcorr)?;
    let mut state: [SpiceDouble; 6] = [0.0; 6];
    let mut lt: SpiceDouble = 0.0;
    unsafe {
        spkez_c(
            target as SpiceInt,
            et,
            c_frame.as_ptr() as *mut c_char,
            c_abcorr.as_ptr() as *mut c_char,
            observer as SpiceInt,
            state.as_mut_ptr(),
            &mut lt,
        );
    }
    check_and_reset()?;
    Ok((state, lt))
}

pub fn sxform(from: &str, to: &str, et: f64) -> Result<[[f64; 6]; 6], SpiceError> {
    let _guard = CSPICE_LOCK.lock().unwrap();
    ensure_return_on_error();
    let c_from = to_cstring(from)?;
    let c_to = to_cstring(to)?;
    let mut xform: [[SpiceDouble; 6]; 6] = [[0.0; 6]; 6];
    unsafe {
        sxform_c(
            c_from.as_ptr() as *mut c_char,
            c_to.as_ptr() as *mut c_char,
            et,
            xform.as_mut_ptr(),
        );
    }
    check_and_reset()?;
    Ok(xform)
}

pub fn pxform(from: &str, to: &str, et: f64) -> Result<[[f64; 3]; 3], SpiceError> {
    let _guard = CSPICE_LOCK.lock().unwrap();
    ensure_return_on_error();
    let c_from = to_cstring(from)?;
    let c_to = to_cstring(to)?;
    let mut rot: [[SpiceDouble; 3]; 3] = [[0.0; 3]; 3];
    unsafe {
        pxform_c(
            c_from.as_ptr() as *mut c_char,
            c_to.as_ptr() as *mut c_char,
            et,
            rot.as_mut_ptr(),
        );
    }
    check_and_reset()?;
    Ok(rot)
}

/// Returns `Ok(Some(code))` when the name resolves, `Ok(None)` when
/// CSpice reports not-found, `Err` on a CSpice runtime error.
pub fn bodn2c(name: &str) -> Result<Option<i32>, SpiceError> {
    let _guard = CSPICE_LOCK.lock().unwrap();
    ensure_return_on_error();
    let c_name = to_cstring(name)?;
    let mut code: SpiceInt = 0;
    let mut found: SpiceBoolean = 0;
    unsafe {
        bodn2c_c(c_name.as_ptr() as *mut c_char, &mut code, &mut found);
    }
    check_and_reset()?;
    Ok(if found != 0 { Some(code as i32) } else { None })
}

/// Returns `Ok(Some(name))` when the code resolves, `Ok(None)` when
/// CSpice reports not-found, `Err` on a CSpice runtime error. Names
/// are null-terminated and trimmed of trailing whitespace.
pub fn bodc2n(code: i32) -> Result<Option<String>, SpiceError> {
    const NAME_LEN: usize = 64;
    let _guard = CSPICE_LOCK.lock().unwrap();
    ensure_return_on_error();
    let mut buf = vec![0 as c_char; NAME_LEN];
    let mut found: SpiceBoolean = 0;
    unsafe {
        bodc2n_c(
            code as SpiceInt,
            NAME_LEN as SpiceInt,
            buf.as_mut_ptr(),
            &mut found,
        );
    }
    check_and_reset()?;
    if found == 0 {
        return Ok(None);
    }
    let name = unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_string_lossy()
        .trim_end()
        .to_string();
    Ok(Some(name))
}
