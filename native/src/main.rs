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
use core::{alloc::Layout, arch::asm, cell::SyncUnsafeCell, panic::PanicInfo};
use global::GlobalObjs;
use jvm::*;

#[no_mangle]
static mut ALLOC_FNS: [usize; 3] = [0; 3];

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    'fail: {
        let Some(jvm) = (unsafe { (*ENV.get()).jvm.as_ref() }) else { break 'fail };
        let Ok(jni) = jvm.jvm.get_jni() else { break 'fail };
        let Ok(msg) = CString::new(format!("{info}")) else { break 'fail };
        jni.fatal_error(&msg)
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

struct GlobalAlloc;
#[global_allocator]
static GLOBAL_ALLOC: GlobalAlloc = GlobalAlloc;
unsafe impl core::alloc::GlobalAlloc for GlobalAlloc {
    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) { dyn_abi!((*mut u8), (), ALLOC_FNS[0], (ptr)) }

    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        assert!(layout.align() <= 16);
        dyn_abi!((usize), *mut u8, ALLOC_FNS[1], (layout.size()))
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        assert!(layout.align() <= 16);
        dyn_abi!((*mut u8, usize), *mut u8, ALLOC_FNS[2], (ptr, new_size))
    }
}

fn entry_common(jni: &'static JNI, inst: usize) {
    let jvm = jni.get_jvm().unwrap();
    let owned_ti = jvm.get_jvmti().unwrap();
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
