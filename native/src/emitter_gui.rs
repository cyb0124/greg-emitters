use crate::{
    global::GlobalMtx,
    util::{
        cleaner::Cleanable,
        gui::{Menu, MenuType},
    },
};
use alloc::sync::Arc;

pub struct EmitterMenu {}
pub struct EmitterMenuType;

impl MenuType for EmitterMenuType {
    fn new_client(&self, data: &[u8]) -> Arc<dyn Menu> { Arc::new(EmitterMenu {}) }
    fn raw(&self, lk: &GlobalMtx) -> usize { lk.emitter_blocks.get().unwrap().menu_type.raw }
}

impl Cleanable for EmitterMenu {
    fn free(self: Arc<Self>, jni: &crate::JNI) {}
}

impl Menu for EmitterMenu {}
