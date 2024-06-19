use crate::{jvm::*, objs};
use core::ffi::CStr;

pub const KEY_COMMON: &CStr = c"c";
pub const KEY_SERVER: &CStr = c"s";

impl<'a, T: JRef<'a>> NBTExt<'a> for T {}
pub trait NBTExt<'a>: JRef<'a> {
    fn compound_put_byte_array(&self, key: &CStr, data: &[u8]) {
        let buf = self.jni().new_byte_array(data.len() as _).unwrap();
        buf.write_byte_array(&data, 0).unwrap();
        self.call_void_method(objs().mv.nbt_compound_put_byte_array, &[buf.jni.new_utf(key).unwrap().raw, buf.raw]).unwrap()
    }

    fn compound_get_byte_array(&self, key: &CStr) -> LocalRef<'a> {
        self.call_object_method(objs().mv.nbt_compound_get_byte_array, &[self.jni().new_utf(key).unwrap().raw]).unwrap().unwrap()
    }
}

pub fn new_compound(jni: &JNI) -> LocalRef {
    let mv = &objs().mv;
    mv.nbt_compound.with_jni(jni).new_object(mv.nbt_compound_init, &[]).unwrap()
}
