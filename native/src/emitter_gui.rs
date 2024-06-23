use crate::{
    global::GlobalMtx,
    util::{
        cleaner::Cleanable,
        gui::{Menu, MenuType},
    },
};
use alloc::sync::Arc;
use nalgebra::{vector, Vector2};

pub struct EmitterMenuType;
pub struct EmitterMenu {}

impl MenuType for EmitterMenuType {
    fn new_client(&self, data: &[u8]) -> Arc<dyn Menu> { Arc::new(EmitterMenu {}) }
    fn raw(&self, lk: &GlobalMtx) -> usize { lk.emitter_blocks.get().unwrap().menu_type.raw }
}

impl Cleanable for EmitterMenu {
    fn free(self: Arc<Self>, jni: &crate::JNI) {}
}

impl Menu for EmitterMenu {
    fn get_size(&self) -> Vector2<i32> { vector![120, 120] }
    fn get_offset(&self) -> Vector2<i32> { vector![120, 0] }
}
