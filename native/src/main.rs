#![no_std]
#![no_main]
#![feature(unsize, ptr_metadata, try_blocks, sync_unsafe_cell)]

#[cfg(target_arch = "x86_64")]
macro_rules! dyn_abi {
    ($arg_types:tt, $ret:ty, $addr:expr, $arg_terms:tt) => {{
        if unsafe { (*crate::ENV.get()).is_win } {
            let func: extern "win64" fn $arg_types -> $ret = unsafe { core::mem::transmute($addr as *const ()) };
            func $arg_terms
        } else {
            let func: extern "sysv64" fn $arg_types -> $ret = unsafe { core::mem::transmute($addr as *const ()) };
            func $arg_terms
        }
    }};
}

#[cfg(target_arch = "aarch64")]
macro_rules! dyn_abi {
    ($arg_types:tt, $ret:ty, $addr:expr, $arg_terms:tt) => {{
        let func: extern "C" fn $arg_types -> $ret = unsafe { core::mem::transmute($addr as *const ()) };
        func $arg_terms
    }};
}

pub mod asm;
mod beams;
mod emitter_blocks;
mod emitter_gui;
mod emitter_items;
mod global;
pub mod jvm;
pub mod mapping_base;
mod packets;
mod registry;
mod util;

extern crate alloc;
use alloc::{ffi::CString, format};
use core::{
    alloc::{GlobalAlloc, Layout},
    arch::asm,
    cell::SyncUnsafeCell,
    panic::PanicInfo,
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};
use global::GlobalObjs;
use jvm::*;

/// JVMTI pointer for the global allocator. Must be initialized before any allocations.
static JVMTI_PTR: AtomicPtr<JVMTI> = AtomicPtr::new(ptr::null_mut());

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    'fail: {
        let Some(jvm) = (unsafe { (*ENV.get()).jvm.as_ref() }) else { break 'fail };
        let Ok(jni) = jvm.jvm.get_jni() else { break 'fail };
        if JVMTI_PTR.load(Ordering::Acquire).is_null() {
            jni.fatal_error(c"panic (before allocator init)")
        } else {
            let Ok(msg) = CString::new(format!("{info}")) else { break 'fail };
            jni.fatal_error(&msg)
        }
    }
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!("ud2", options(noreturn));
        #[cfg(target_arch = "aarch64")]
        asm!("udf #0xDEAD", options(noreturn));
    }
}

unsafe impl Sync for GlobalJVM {}
struct GlobalJVM {
    jvm: &'static JVM,
    ti: &'static JVMTI,
}

struct GlobalEnv {
    #[cfg(target_arch = "x86_64")]
    is_win: bool,
    jvm: Option<GlobalJVM>,
    objs: Option<GlobalObjs>,
}

static ENV: SyncUnsafeCell<GlobalEnv> = SyncUnsafeCell::new(GlobalEnv {
    #[cfg(target_arch = "x86_64")]
    is_win: false,
    jvm: None,
    objs: None,
});

fn ti() -> &'static JVMTI { unsafe { (*ENV.get()).jvm.as_ref().unwrap_unchecked().ti } }
fn objs() -> &'static GlobalObjs { unsafe { (*ENV.get()).objs.as_ref().unwrap_unchecked() } }

struct JvmtiAlloc;

#[global_allocator]
static GLOBAL_ALLOC: JvmtiAlloc = JvmtiAlloc;

unsafe impl GlobalAlloc for JvmtiAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ti = JVMTI_PTR.load(Ordering::Acquire);
        assert!(!ti.is_null(), "JVMTI allocator not initialized");
        assert!(layout.align() <= 16, "alignment > 16 not supported");
        match (*ti).allocate(layout.size()) {
            Ok(ptr) => ptr,
            Err(_) => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let ti = JVMTI_PTR.load(Ordering::Acquire);
        let _ = (*ti).deallocate(ptr);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // JVMTI has no realloc, implement manually
        let new_ptr = self.alloc(Layout::from_size_align_unchecked(new_size, layout.align()));
        if !new_ptr.is_null() {
            ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
            self.dealloc(ptr, layout);
        }
        new_ptr
    }
}

fn entry_common(jni: &'static JNI, inst: usize) {
    // IMPORTANT: Get JVMTI and initialize allocator FIRST, before any allocations.
    let jvm = jni.get_jvm().unwrap();
    let owned_ti = jvm.get_jvmti().unwrap();

    // Initialize allocator - after this point we can allocate
    JVMTI_PTR.store(owned_ti.raw as *const _ as *mut _, Ordering::Release);

    // Now safe to use allocating code
    unsafe { (*ENV.get()).jvm = Some(GlobalJVM { jvm, ti: owned_ti.raw }) }
    core::mem::forget(owned_ti);
    let inst = BorrowedRef::new(jni, &inst);
    unsafe { (*ENV.get()).objs = Some(GlobalObjs::new(inst.get_object_class())) }
    registry::init()
}

#[cfg(target_arch = "x86_64")]
#[no_mangle]
pub extern "sysv64" fn entry_sysv64(jni: &'static JNI, inst: usize) { entry_common(jni, inst) }

#[cfg(target_arch = "x86_64")]
#[no_mangle]
pub extern "win64" fn entry_win64(jni: &'static JNI, inst: usize) {
    unsafe { (*ENV.get()).is_win = true }
    entry_common(jni, inst)
}

#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub extern "C" fn entry_aarch64(jni: &'static JNI, inst: usize) { entry_common(jni, inst) }
