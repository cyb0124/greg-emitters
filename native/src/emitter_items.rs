use crate::{asm::*, global::GlobalObjs, jvm::*, mapping_base::*, objs};
use macros::dyn_abi;

pub struct EmitterItems {
    pub item: GlobalRef<'static>,
    item_maker: GlobalRef<'static>,
    item_maker_tier: usize,
    pub use_on_ctx: Option<GlobalRef<'static>>,
}

impl EmitterItems {
    pub fn new(jni: &'static JNI) -> Self {
        // Item Class
        let GlobalObjs { av, cn, mn, gcn, namer, .. } = objs();
        let mut name = namer.next();
        let mut cls = av.new_class_node(jni, &name.slash, &cn.block_item.slash).unwrap();
        let methods = [
            mn.item_get_desc_id.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.item_use_on.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        ];
        cls.class_methods(av).unwrap().collection_extend(&av.jv, methods).unwrap();
        cls = av.ldr.with_jni(jni).define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
        cls.register_natives(&[mn.item_get_desc_id.native(get_desc_id_dyn()), mn.item_use_on.native(item_use_on_dyn())]).unwrap();
        let item = cls.new_global_ref().unwrap();

        // Item Maker
        name = namer.next();
        cls = av.new_class_node(jni, &name.slash, c"java/lang/Object").unwrap();
        cls.add_interfaces(av, [&*gcn.non_null_fn.slash]).unwrap();
        cls.class_fields(av).unwrap().collection_extend(&av.jv, [av.new_field_node(jni, c"0", c"B", 0, 0).unwrap()]).unwrap();
        let apply = MSig { owner: name.clone(), name: cs("apply"), sig: cs("(Ljava/lang/Object;)Ljava/lang/Object;") };
        cls.class_methods(av).unwrap().collection_extend(&av.jv, [apply.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap()]).unwrap();
        cls = av.ldr.with_jni(jni).define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
        cls.register_natives(&[apply.native(make_item_dyn())]).unwrap();

        Self { item, item_maker: cls.new_global_ref().unwrap(), item_maker_tier: cls.get_field_id(c"0", c"B").unwrap(), use_on_ctx: None }
    }

    pub fn make_item_maker<'a>(&self, jni: &'a JNI, tier: u8) -> LocalRef<'a> {
        let obj = self.item_maker.with_jni(jni).alloc_object().unwrap();
        obj.set_byte_field(self.item_maker_tier, tier);
        obj
    }
}

#[dyn_abi]
fn get_desc_id(jni: &JNI, this: usize) -> usize {
    let GlobalObjs { mv, .. } = objs();
    BorrowedRef::new(jni, &this).call_nonvirtual_object_method(mv.item.raw, mv.item_get_desc_id, &[]).unwrap().unwrap().into_raw()
}

#[dyn_abi]
fn make_item(jni: &JNI, this: usize, props: usize) -> usize {
    let lk = objs().mtx.lock(jni).unwrap();
    let defs = lk.emitter_items.uref();
    let tier = BorrowedRef::new(jni, &this).get_byte_field(defs.item_maker_tier);
    let block = lk.tiers[tier as usize].emitter_block.uref();
    defs.item.with_jni(jni).new_object(objs().mv.block_item_init, &[block.raw, props]).unwrap().into_raw()
}

#[dyn_abi]
fn item_use_on(jni: &'static JNI, this: usize, ctx: usize) -> usize {
    let GlobalObjs { mtx, mv, .. } = objs();
    let mut lk = mtx.lock(jni).unwrap();
    let saved = BorrowedRef::new(jni, &ctx).new_global_ref().unwrap();
    lk.emitter_items.as_mut().unwrap().use_on_ctx.replace(saved).map(|x| x.replace_jni(jni));
    BorrowedRef::new(jni, &this).call_nonvirtual_object_method(mv.block_item.raw, mv.block_item_use_on, &[ctx]).unwrap().unwrap().into_raw()
}
