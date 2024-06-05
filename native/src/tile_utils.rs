use crate::{
    asm::*,
    global::{ClassNamer, GlobalObjs},
    jvm::*,
    mapping::{CN, MN},
    mapping_base::*,
    objs,
};
use alloc::{format, sync::Arc};
use core::{ffi::CStr, mem::transmute};
use macros::dyn_abi;
use serde::{de::DeserializeOwned, Serialize};

pub const TAG_SERVER: &CStr = c"s";
pub const TAG_COMMON: &CStr = c"c";

pub fn write_tag<'a>(tag: &impl JRef<'a>, key: &CStr, value: &impl Serialize) {
    let data = postcard::to_allocvec(value).unwrap();
    let ba = tag.jni().new_byte_array(data.len() as _).unwrap();
    ba.write_byte_array(&data, 0).unwrap();
    tag.call_void_method(objs().mv.nbt_compound_put_byte_array, &[ba.jni.new_utf(key).unwrap().raw, ba.raw]).unwrap()
}

pub fn read_tag<'a, T: DeserializeOwned>(tag: &impl JRef<'a>, key: &CStr, value: &mut T) -> bool {
    let blob = tag.call_object_method(objs().mv.nbt_compound_get_byte_array, &[tag.jni().new_utf(key).unwrap().raw]).unwrap().unwrap();
    let data = blob.crit_elems().unwrap();
    let false = data.is_empty() else { return true };
    let Err(e) = T::deserialize_in_place(&mut postcard::Deserializer::from_bytes(&*data), value) else { return true };
    drop(data);
    objs().av.jv.runtime_exception.with_jni(blob.jni).throw_new(&cs(format!("{e}"))).unwrap();
    false
}

pub type TileMakerFn = fn(&'static JNI, pos: usize, state: usize) -> usize;
pub struct TileUtils {
    supplier_cls: GlobalRef<'static>,
    supplier_p: usize,
}

impl TileUtils {
    pub fn new(av: &AV<'static>, cn: &CN<Arc<CSig>>, mn: &MN<MSig>, namer: &ClassNamer) -> Self {
        let jni = av.ldr.jni;
        let name = namer.next();
        let mut cls = av.new_class_node(jni, &name.slash, c"java/lang/Object").unwrap();
        cls.add_interfaces(av, [&*cn.tile_supplier.slash]).unwrap();
        let create = mn.tile_supplier_create.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap();
        cls.class_methods(av).unwrap().collection_extend(&av.jv, [create]).unwrap();
        cls.class_fields(av).unwrap().collection_extend(&av.jv, [av.new_field_node(jni, c"0", c"J", 0, 0).unwrap()]).unwrap();
        cls = av.ldr.define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
        cls.register_natives(&[mn.tile_supplier_create.native(tile_supplier_create_dyn())]).unwrap();
        Self { supplier_cls: cls.new_global_ref().unwrap(), supplier_p: cls.get_field_id(c"0", c"J").unwrap() }
    }

    pub fn define_tile_type<'a>(&self, blocks: &impl JRef<'a>, func: TileMakerFn) -> GlobalRef<'a> {
        let inst = self.supplier_cls.with_jni(blocks.jni()).alloc_object().unwrap();
        inst.set_long_field(self.supplier_p, func as _);
        let mv = &objs().mv;
        mv.tile_type.with_jni(inst.jni).new_object(mv.tile_type_init, &[inst.raw, blocks.raw(), 0]).unwrap().new_global_ref().unwrap()
    }
}

#[dyn_abi]
fn tile_supplier_create(jni: &'static JNI, this: usize, pos: usize, state: usize) -> usize {
    let p = objs().tile_utils.supplier_p;
    let func: TileMakerFn = unsafe { transmute(BorrowedRef::new(jni, &this).get_long_field(p)) };
    func(jni, pos, state)
}

pub fn tile_get_update_packet_impl<'a>(jni: &'a JNI) -> LocalRef<'a> {
    let GlobalObjs { av, mn, .. } = objs();
    let insns = [
        av.new_var_insn(jni, OP_ALOAD, 0).unwrap(),
        mn.s2c_tile_data_create.new_method_insn(av, jni, OP_INVOKESTATIC).unwrap(),
        av.new_insn(jni, OP_ARETURN).unwrap(),
    ];
    let method = mn.tile_get_update_packet.new_method_node(av, jni, ACC_PUBLIC).unwrap();
    method.method_insns(av).unwrap().append_insns(av, insns).unwrap();
    method
}

pub fn non_null_supplier_get_self_impl<'a>(jni: &'a JNI) -> LocalRef<'a> {
    let GlobalObjs { av, fmn, .. } = objs();
    let method = fmn.non_null_supplier_get.new_method_node(av, jni, ACC_PUBLIC).unwrap();
    let insns = [av.new_var_insn(jni, OP_ALOAD, 0).unwrap(), av.new_insn(jni, OP_ARETURN).unwrap()];
    method.method_insns(av).unwrap().append_insns(av, insns).unwrap();
    method
}

pub fn const_long_impl<'a>(jni: &'a JNI, mn: &MSig, value: i64) -> LocalRef<'a> {
    let av = &objs().av;
    let method = mn.new_method_node(av, jni, ACC_PUBLIC).unwrap();
    let insns = [av.new_ldc_insn(jni, av.jv.wrap_long(jni, value).unwrap().raw).unwrap(), av.new_insn(jni, OP_LRETURN).unwrap()];
    method.method_insns(av).unwrap().append_insns(av, insns).unwrap();
    method
}
