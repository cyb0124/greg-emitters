#![no_std]
#![no_main]

#[cfg(target_arch = "x86_64")]
macro_rules! dyn_abi {
    ($arg_types:tt, $ret:ty, $addr:expr, $arg_terms:tt) => {{
        if unsafe { crate::ENV.is_win } {
            let func: extern "win64" fn $arg_types -> $ret = unsafe { core::mem::transmute($addr) };
            func $arg_terms
        } else {
            let func: extern "sysv64" fn $arg_types -> $ret = unsafe { core::mem::transmute($addr) };
            func $arg_terms
        }
    }};
}

#[cfg(target_arch = "aarch64")]
macro_rules! dyn_abi {
    ($arg_types:tt, $ret:ty, $addr:expr, $arg_terms:tt) => {{
        let func: extern "C" fn $arg_types -> $ret = unsafe { core::mem::transmute($addr) };
        func $arg_terms
    }};
}

pub mod asm;
mod cleaner;
mod client_utils;
mod emitter_blocks;
mod emitter_items;
mod geometry;
mod global;
pub mod jvm;
mod mapping;
pub mod mapping_base;
mod registry;
mod tile_utils;

extern crate alloc;
use alloc::{ffi::CString, format};
use core::{alloc::GlobalAlloc, alloc::Layout, arch::asm, panic::PanicInfo};
use global::GlobalObjs;
use jvm::*;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    'fail: {
        let Some(jvm) = (unsafe { ENV.jvm.as_ref() }) else { break 'fail };
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

struct GlobalJVM {
    jvm: &'static JVM,
    ti: &'static JVMTI,
}

struct GlobalEnv
where
    GlobalObjs: Sync,
{
    #[cfg(target_arch = "x86_64")]
    is_win: bool,
    jvm: Option<GlobalJVM>,
    objs: Option<GlobalObjs>,
}

#[global_allocator]
static mut ENV: GlobalEnv = GlobalEnv {
    #[cfg(target_arch = "x86_64")]
    is_win: false,
    jvm: None,
    objs: None,
};

fn ti() -> &'static JVMTI { unsafe { ENV.jvm.as_ref().unwrap_unchecked().ti } }
fn objs() -> &'static GlobalObjs { unsafe { ENV.objs.as_ref().unwrap_unchecked() } }
unsafe impl GlobalAlloc for GlobalEnv {
    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) { ti().deallocate(ptr).unwrap() }
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        assert!(layout.align() <= 16);
        ti().allocate(layout.size() as _).unwrap()
    }
}

fn entry_common(jni: &'static JNI, inst: usize) {
    let jvm = jni.get_jvm().unwrap();
    let owned_ti = jvm.get_jvmti().unwrap();
    unsafe { ENV.jvm = Some(GlobalJVM { jvm, ti: owned_ti.raw }) }
    core::mem::forget(owned_ti);
    let inst = BorrowedRef::new(jni, &inst);
    unsafe { ENV.objs = Some(GlobalObjs::new(inst.get_object_class())) }
    registry::init()
}

#[cfg(target_arch = "x86_64")]
#[no_mangle]
pub extern "sysv64" fn entry_sysv64(jni: &'static JNI, ldr_cls: usize) { entry_common(jni, ldr_cls) }

#[cfg(target_arch = "x86_64")]
#[no_mangle]
pub extern "win64" fn entry_win64(jni: &'static JNI, ldr_cls: usize) {
    unsafe { ENV.is_win = true }
    entry_common(jni, ldr_cls)
}

#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub extern "C" fn entry_aarch64(jni: &'static JNI, ldr_cls: usize) { entry_common(jni, ldr_cls) }
