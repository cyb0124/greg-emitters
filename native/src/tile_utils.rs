use crate::{
    asm::*,
    global::ClassNamer,
    jvm::*,
    mapping::{CN, MN},
    mapping_base::*,
    objs,
};
use alloc::sync::Arc;
use core::mem::transmute;
use macros::dyn_abi;

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
