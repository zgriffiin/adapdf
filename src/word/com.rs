//! Late-bound COM (`IDispatch`) and the few Win32 calls the Word layer needs.
//!
//! This file holds every `unsafe` block of the crate. Each block is inside a
//! small safe function with a `// SAFETY:` comment, so the rest of the crate
//! (and the rest of `src/word`) is safe code.

use std::ffi::c_void;
use std::fmt;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, RPC_E_CHANGED_MODE, STILL_ACTIVE};
use windows::Win32::System::Com::{
    CLSCTX_LOCAL_SERVER, CLSIDFromProgID, COINIT_APARTMENTTHREADED, CoCreateInstance,
    CoInitializeEx, CoUninitialize, DISPATCH_FLAGS, DISPATCH_METHOD, DISPATCH_PROPERTYGET,
    DISPATCH_PROPERTYPUT, DISPPARAMS, EXCEPINFO, IDispatch,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE, QueryFullProcessImageNameW, TerminateProcess,
};
use windows::Win32::System::Variant::{
    VAR_CHANGE_FLAGS, VARENUM, VARIANT, VT_BSTR, VT_I4, VariantChangeType,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::core::{BSTR, GUID, HRESULT, HSTRING, PCWSTR, PWSTR};

/// `DISP_E_EXCEPTION`: the called object raised an error, described in `EXCEPINFO`.
const DISP_E_EXCEPTION: i32 = 0x8002_0009_u32 as i32;
/// `REGDB_E_CLASSNOTREG`: the program ID is not registered (Word is not installed).
const REGDB_E_CLASSNOTREG: i32 = 0x8004_0154_u32 as i32;
/// `DISPID_PROPERTYPUT`: the dispatch id of the value in a property assignment.
const DISPID_PROPERTYPUT: i32 = -3;
/// `LOCALE_USER_DEFAULT`.
const LOCALE_USER_DEFAULT: u32 = 0x0400;

/// A failed COM call, with the message that the other side gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComError {
    /// What was being done, for example `Documents.Open`.
    pub call: String,
    pub code: i32,
    pub message: String,
}

impl ComError {
    fn new(call: &str, error: &windows::core::Error) -> ComError {
        ComError {
            call: call.to_string(),
            code: error.code().0,
            message: error.message().trim().to_string(),
        }
    }

    /// The program ID is not registered, so the program is not installed.
    pub fn is_class_not_registered(&self) -> bool {
        self.code == REGDB_E_CLASSNOTREG
    }
}

impl fmt::Display for ComError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} failed: {} (code 0x{:08X})",
            self.call, self.message, self.code as u32
        )
    }
}

impl std::error::Error for ComError {}

/// COM is started on this thread for as long as the value lives. COM objects
/// made on a thread must only be used on that thread, and this type is not
/// `Send`, so the compiler enforces it.
pub struct ComApartment {
    /// False when the thread already had COM started in another mode; then
    /// nothing was started here and nothing is stopped on drop.
    started: bool,
    not_send: PhantomData<*const ()>,
}

impl ComApartment {
    /// Starts COM on this thread as a single-threaded apartment.
    pub fn enter() -> Result<ComApartment, ComError> {
        let result = co_initialize();
        if result == RPC_E_CHANGED_MODE {
            // The thread already uses COM in another mode. Word can be driven
            // from it all the same.
            return Ok(ComApartment {
                started: false,
                not_send: PhantomData,
            });
        }
        result
            .ok()
            .map_err(|e| ComError::new("CoInitializeEx", &e))?;
        Ok(ComApartment {
            started: true,
            not_send: PhantomData,
        })
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.started {
            co_uninitialize();
        }
    }
}

fn co_initialize() -> HRESULT {
    // SAFETY: no pointer is passed. A successful call is balanced by
    // `co_uninitialize` in `ComApartment::drop`.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
}

fn co_uninitialize() {
    // SAFETY: only called to balance one successful `co_initialize` on the
    // same thread (`ComApartment` is not `Send`).
    unsafe { CoUninitialize() }
}

/// An `IDispatch` object, called by name.
pub struct Dispatch(IDispatch);

impl Dispatch {
    /// Starts a new COM server by program ID (for example `Word.Application`).
    /// It never attaches to a running object by name; whether COM gave a new
    /// process or a running one is for the caller to check.
    pub fn create(prog_id: &str) -> Result<Dispatch, ComError> {
        let name = HSTRING::from(prog_id);
        let fail = |e: windows::core::Error| ComError::new(&format!("start {prog_id}"), &e);
        let clsid = class_id_from_prog_id(&name).map_err(fail)?;
        create_local_server(&clsid).map(Dispatch).map_err(fail)
    }

    /// Reads a property.
    pub fn get(&self, name: &str) -> Result<VARIANT, ComError> {
        self.invoke(name, DISPATCH_PROPERTYGET, Vec::new())
    }

    /// Reads a property that is an object.
    pub fn get_object(&self, name: &str) -> Result<Dispatch, ComError> {
        let value = self.get(name)?;
        variant_dispatch(&value)
            .map(Dispatch)
            .ok_or_else(|| not_an_object(name))
    }

    /// Reads a number property.
    pub fn get_i32(&self, name: &str) -> Result<i32, ComError> {
        let value = self.get(name)?;
        variant_i32(&value).ok_or_else(|| not_a(name, "number"))
    }

    /// Reads a text property.
    pub fn get_string(&self, name: &str) -> Result<String, ComError> {
        let value = self.get(name)?;
        variant_string(&value).ok_or_else(|| not_a(name, "text"))
    }

    /// Sets a property.
    pub fn put(&self, name: &str, value: impl Into<VARIANT>) -> Result<(), ComError> {
        // The value of a property assignment is a named argument whose
        // dispatch id is DISPID_PROPERTYPUT, and it has no name of its own.
        self.invoke_with_ids(
            name,
            DISPATCH_PROPERTYPUT,
            vec![(DISPID_PROPERTYPUT, value.into())],
        )
        .map(drop)
    }

    /// Calls a method. Every argument is passed by name, so optional
    /// arguments in between can be left out.
    pub fn call(&self, name: &str, args: Vec<(&str, VARIANT)>) -> Result<VARIANT, ComError> {
        self.invoke(name, DISPATCH_METHOD, args)
    }

    /// Calls a method that gives a number.
    pub fn call_i32(&self, name: &str, args: Vec<(&str, VARIANT)>) -> Result<i32, ComError> {
        let value = self.call(name, args)?;
        variant_i32(&value).ok_or_else(|| not_a(name, "number"))
    }

    /// Calls a method that gives an object.
    pub fn call_object(
        &self,
        name: &str,
        args: Vec<(&str, VARIANT)>,
    ) -> Result<Dispatch, ComError> {
        let value = self.call(name, args)?;
        variant_dispatch(&value)
            .map(Dispatch)
            .ok_or_else(|| not_an_object(name))
    }

    /// Reads a property that takes arguments (an indexed property, such as the
    /// `Item` of Office's document properties) and gives an object.
    pub fn get_object_with(
        &self,
        name: &str,
        args: Vec<(&str, VARIANT)>,
    ) -> Result<Dispatch, ComError> {
        let value = self.invoke(name, DISPATCH_PROPERTYGET, args)?;
        variant_dispatch(&value)
            .map(Dispatch)
            .ok_or_else(|| not_an_object(name))
    }

    fn invoke(
        &self,
        name: &str,
        flags: DISPATCH_FLAGS,
        args: Vec<(&str, VARIANT)>,
    ) -> Result<VARIANT, ComError> {
        let fail = |e: windows::core::Error| ComError::new(name, &e);
        let mut names = vec![HSTRING::from(name)];
        names.extend(args.iter().map(|(arg, _)| HSTRING::from(*arg)));
        let ids = dispatch_ids(&self.0, &names).map_err(fail)?;
        let args = args
            .into_iter()
            .zip(&ids[1..])
            .map(|((_, value), id)| (*id, value))
            .collect();
        self.invoke_by_id(name, ids[0], flags, args)
    }

    fn invoke_with_ids(
        &self,
        name: &str,
        flags: DISPATCH_FLAGS,
        args: Vec<(i32, VARIANT)>,
    ) -> Result<VARIANT, ComError> {
        let ids =
            dispatch_ids(&self.0, &[HSTRING::from(name)]).map_err(|e| ComError::new(name, &e))?;
        self.invoke_by_id(name, ids[0], flags, args)
    }

    fn invoke_by_id(
        &self,
        name: &str,
        member: i32,
        flags: DISPATCH_FLAGS,
        args: Vec<(i32, VARIANT)>,
    ) -> Result<VARIANT, ComError> {
        let (mut ids, mut values): (Vec<i32>, Vec<VARIANT>) = args.into_iter().unzip();
        let mut result = VARIANT::default();
        let mut exception = EXCEPINFO::default();
        let outcome = invoke_raw(
            &self.0,
            member,
            flags,
            &mut ids,
            &mut values,
            &mut result,
            &mut exception,
        );
        match outcome {
            Ok(()) => Ok(result),
            Err(e) if e.code().0 == DISP_E_EXCEPTION => {
                let (scode, description) = take_exception(&mut exception);
                Err(ComError {
                    call: name.to_string(),
                    code: if scode != 0 { scode } else { e.code().0 },
                    message: description.trim().to_string(),
                })
            }
            Err(e) => Err(ComError::new(name, &e)),
        }
    }
}

/// The error code and the description in an `EXCEPINFO` that `Invoke` filled,
/// and frees the strings in it.
fn take_exception(exception: &mut EXCEPINFO) -> (i32, String) {
    let description = exception.bstrDescription.to_string();
    // SAFETY: after a call that returned DISP_E_EXCEPTION, each of the three
    // strings is empty or a BSTR that the callee allocated for the caller to
    // free. Each is dropped here once, and `exception` is not read again.
    unsafe {
        ManuallyDrop::drop(&mut exception.bstrSource);
        ManuallyDrop::drop(&mut exception.bstrDescription);
        ManuallyDrop::drop(&mut exception.bstrHelpFile);
    }
    (exception.scode, description)
}

fn not_an_object(name: &str) -> ComError {
    not_a(name, "object")
}

fn not_a(name: &str, kind: &str) -> ComError {
    ComError {
        call: name.to_string(),
        code: 0,
        message: format!("the value is not an expected {kind}"),
    }
}

fn class_id_from_prog_id(name: &HSTRING) -> windows::core::Result<GUID> {
    // SAFETY: `name` is a valid, NUL-terminated wide string that outlives the call.
    unsafe { CLSIDFromProgID(PCWSTR(name.as_ptr())) }
}

fn create_local_server(clsid: &GUID) -> windows::core::Result<IDispatch> {
    // SAFETY: `clsid` points to a valid GUID for the call. The class is made
    // in its own server process (`CLSCTX_LOCAL_SERVER`), with no outer object.
    unsafe { CoCreateInstance(clsid, None, CLSCTX_LOCAL_SERVER) }
}

/// The dispatch ids of a method or property name and then its argument names.
fn dispatch_ids(object: &IDispatch, names: &[HSTRING]) -> windows::core::Result<Vec<i32>> {
    let pointers: Vec<PCWSTR> = names.iter().map(|n| PCWSTR(n.as_ptr())).collect();
    let mut ids = vec![0_i32; names.len()];
    let count = u32::try_from(names.len()).expect("a few names");
    // SAFETY: `pointers` holds `count` valid NUL-terminated wide strings, kept
    // alive by `names`. `ids` has room for `count` ids. The IID must be a null GUID.
    unsafe {
        object.GetIDsOfNames(
            &GUID::zeroed(),
            pointers.as_ptr(),
            count,
            LOCALE_USER_DEFAULT,
            ids.as_mut_ptr(),
        )?;
    }
    Ok(ids)
}

/// Calls `IDispatch::Invoke` with named arguments: `values[i]` has the
/// dispatch id `ids[i]`.
fn invoke_raw(
    object: &IDispatch,
    member: i32,
    flags: DISPATCH_FLAGS,
    ids: &mut [i32],
    values: &mut [VARIANT],
    result: &mut VARIANT,
    exception: &mut EXCEPINFO,
) -> windows::core::Result<()> {
    let count = u32::try_from(values.len()).expect("a few arguments");
    let params = DISPPARAMS {
        rgvarg: values.as_mut_ptr(),
        rgdispidNamedArgs: ids.as_mut_ptr(),
        cArgs: count,
        cNamedArgs: count,
    };
    // SAFETY: `params` points at `values` and `ids`, which are valid, have the
    // same length (`count`) and outlive the call. `result` and `exception` are
    // valid for writes. The IID must be a null GUID.
    unsafe {
        object.Invoke(
            member,
            &GUID::zeroed(),
            LOCALE_USER_DEFAULT,
            flags,
            &params,
            Some(result),
            Some(exception),
            None,
        )
    }
}

/// The value as a number, when it is one (or can be made into one).
fn variant_i32(value: &VARIANT) -> Option<i32> {
    let converted = change_type(value, VT_I4)?;
    // SAFETY: `change_type` gave a VT_I4 variant, so `lVal` is the active field.
    Some(unsafe { converted.Anonymous.Anonymous.Anonymous.lVal })
}

/// The value as text, when it is text (or can be made into text).
fn variant_string(value: &VARIANT) -> Option<String> {
    let converted = change_type(value, VT_BSTR)?;
    // SAFETY: `change_type` gave a VT_BSTR variant, so `bstrVal` is the active field.
    let text: &BSTR = unsafe { &converted.Anonymous.Anonymous.Anonymous.bstrVal };
    Some(text.to_string())
}

/// The object in the value, when it holds one.
fn variant_dispatch(value: &VARIANT) -> Option<IDispatch> {
    IDispatch::try_from(value).ok()
}

fn change_type(value: &VARIANT, to: VARENUM) -> Option<VARIANT> {
    let mut converted = VARIANT::default();
    // SAFETY: both pointers are valid variants for the call. `converted` is an
    // empty variant that receives the result and clears itself on drop.
    unsafe { VariantChangeType(&mut converted, value, VAR_CHANGE_FLAGS(0), to) }.ok()?;
    Some(converted)
}

/// The id of the process that owns a window. `None` when there is no such window.
pub fn window_process_id(window: i32) -> Option<u32> {
    // A window handle is 32 bits wide for every process (Windows keeps it so
    // across 32 and 64 bit), so Word's 32-bit `Hwnd` value is sign-extended.
    let handle = HWND(window as isize as *mut c_void);
    let mut process = 0_u32;
    // SAFETY: `process` is valid for writes for the call. A window handle that
    // does not exist makes the call return 0 and is not unsafe.
    let thread = unsafe { GetWindowThreadProcessId(handle, Some(&mut process)) };
    (thread != 0 && process != 0).then_some(process)
}

/// The ids of all running processes whose image name is `exe_name` (compared
/// without regard to case), for example `WINWORD.EXE`.
pub fn process_ids(exe_name: &str) -> Result<Vec<u32>, ComError> {
    let fail = |e: windows::core::Error| ComError::new("list processes", &e);
    // SAFETY: no pointers are passed. The handle is closed below.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.map_err(fail)?;
    let mut ids = Vec::new();
    let mut entry = PROCESSENTRY32W {
        dwSize: u32::try_from(size_of::<PROCESSENTRY32W>()).expect("a small struct"),
        ..PROCESSENTRY32W::default()
    };
    // SAFETY: `snapshot` is a valid snapshot handle and `entry.dwSize` is set,
    // as these calls require. `entry` is valid for writes.
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
    while more {
        let len = entry
            .szExeFile
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(entry.szExeFile.len());
        if String::from_utf16_lossy(&entry.szExeFile[..len]).eq_ignore_ascii_case(exe_name) {
            ids.push(entry.th32ProcessID);
        }
        // SAFETY: as for `Process32FirstW`.
        more = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    // SAFETY: `snapshot` is a handle that this function opened and has not closed.
    // A failure to close cannot be handled, and the ids are already read.
    let _ = unsafe { CloseHandle(snapshot) };
    Ok(ids)
}

/// Ends the process `pid`, but only if its image is `exe_name` (compared
/// without regard to case). The process is opened once, and the image name is
/// read from that open handle, so the process that is checked is the process
/// that is ended even if the id is reused at the same moment. Gives `Ok(false)`
/// when there is no such process any more, and an error (and ends nothing)
/// when the process is another program.
pub fn end_process_named(pid: u32, exe_name: &str) -> Result<bool, ComError> {
    let fail = |what: &str, e: windows::core::Error| ComError::new(what, &e);
    // SAFETY: no pointer is passed. The handle is closed below.
    let handle = match unsafe {
        OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        )
    } {
        Ok(handle) => handle,
        // ERROR_INVALID_PARAMETER: there is no process with this id.
        Err(e) if e.code() == HRESULT::from_win32(87) => return Ok(false),
        Err(e) => return Err(fail("open the Word process", e)),
    };
    let result = end_if_named(handle, pid, exe_name);
    // SAFETY: `handle` is a handle that this function opened and has not closed.
    // A failure to close cannot be handled; the process is already ended or kept.
    let _ = unsafe { CloseHandle(handle) };
    result
}

fn end_if_named(handle: HANDLE, pid: u32, exe_name: &str) -> Result<bool, ComError> {
    // A process that has ended can still be opened while another program holds
    // a handle to it, and then its image name cannot be read.
    let mut code = 0_u32;
    // SAFETY: `handle` is an open process handle with the query right, and
    // `code` is valid for writes.
    if unsafe { GetExitCodeProcess(handle, &mut code) }.is_ok() && code != STILL_ACTIVE.0 as u32 {
        return Ok(false);
    }
    let mut buffer = vec![0_u16; 1024];
    let mut length = u32::try_from(buffer.len()).expect("a small buffer");
    // SAFETY: `buffer` is valid for `length` wide characters, and `length` is
    // updated to the number written. `handle` is an open process handle.
    unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    }
    .map_err(|e| ComError::new("read the name of the process", &e))?;
    let image = String::from_utf16_lossy(&buffer[..length as usize]);
    let file_name = image.rsplit(['\\', '/']).next().unwrap_or("");
    if !file_name.eq_ignore_ascii_case(exe_name) {
        return Err(ComError {
            call: "end the Word process".to_string(),
            code: 0,
            message: format!("process {pid} is {file_name}, not {exe_name}, so it was not ended"),
        });
    }
    // SAFETY: `handle` is an open process handle with the terminate right.
    unsafe { TerminateProcess(handle, 1) }
        .map(|()| true)
        .map_err(|e| ComError::new("end the Word process", &e))
}
