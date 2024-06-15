use crate::{
    asm::*,
    geometry::{read_dir, read_vec3i, write_block_pos, write_dir, DIR_STEPS},
    global::GlobalObjs,
    jvm::*,
    mapping_base::*,
    objs,
};
use macros::dyn_abi;

pub struct EmitterItems {
    pub item: GlobalRef<'static>,
    item_maker: GlobalRef<'static>,
    item_maker_tier: usize,
}

impl EmitterItems {
    pub fn new(jni: &'static JNI) -> Self {
        // Item Class
        let GlobalObjs { av, cn, mn, gcn, namer, .. } = objs();
        let mut name = namer.next();
        let mut cls = av.new_class_node(jni, &name.slash, &cn.block_item.slash).unwrap();
        let methods = [
            mn.item_get_desc_id.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.block_item_place_block.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        ];
        cls.class_methods(av).unwrap().collection_extend(&av.jv, methods).unwrap();
        cls = av.ldr.with_jni(jni).define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
        cls.register_natives(&[mn.item_get_desc_id.native(get_desc_id_dyn()), mn.block_item_place_block.native(place_block_dyn())]).unwrap();
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

        Self { item, item_maker: cls.new_global_ref().unwrap(), item_maker_tier: cls.get_field_id(c"0", c"B").unwrap() }
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
fn make_item(jni: &'static JNI, this: usize, props: usize) -> usize {
    let lk = objs().mtx.lock(jni).unwrap();
    let defs = lk.emitter_items.get().unwrap();
    let tiers = lk.tiers.borrow();
    let tier = &tiers[BorrowedRef::new(jni, &this).get_byte_field(defs.item_maker_tier) as usize];
    let item = defs.item.with_jni(jni).new_object(objs().mv.block_item_init, &[tier.emitter_block.get().unwrap().raw, props]).unwrap();
    tier.emitter_item.set(item.new_global_ref().unwrap()).ok().unwrap();
    item.into_raw()
}

#[dyn_abi]
fn place_block(jni: &'static JNI, this: usize, ctx: usize, state: usize) -> bool {
    let GlobalObjs { mtx, mv, .. } = objs();
    if !BorrowedRef::new(jni, &this).call_nonvirtual_bool_method(mv.block_item.raw, mv.block_item_place_block, &[ctx, state]).unwrap() {
        return false;
    }
    let ctx = BorrowedRef::new(jni, &ctx);
    let level = ctx.call_object_method(mv.use_on_ctx_get_level, &[]).unwrap().unwrap();
    if level.get_bool_field(mv.level_is_client) {
        return true;
    }
    let mut pos = ctx.call_object_method(mv.use_on_ctx_get_clicked_pos, &[]).unwrap().unwrap();
    let dir_obj = ctx.call_object_method(mv.use_on_ctx_get_clicked_face, &[]).unwrap().unwrap();
    let mut dir = read_dir(&dir_obj);
    let tile = level.call_object_method(mv.block_getter_get_tile, &[pos.raw]).unwrap().unwrap();
    let lk = mtx.lock(jni).unwrap();
    lk.emitter_blocks.get().unwrap().from_tile(&tile).common.borrow_mut().dir = Some(dir);
    dir ^= 1;
    pos = write_block_pos(jni, read_vec3i(&pos) + DIR_STEPS[dir as usize]);
    let mut pipe_block = level.call_object_method(mv.block_getter_get_block_state, &[pos.raw]).unwrap().unwrap();
    pipe_block = pipe_block.call_object_method(mv.block_state_get_block, &[]).unwrap().unwrap();
    let gmv = lk.gmv.get().unwrap();
    if !pipe_block.is_instance_of(gmv.pipe_block.raw) {
        return true;
    }
    let Some(pipe_node) = pipe_block.call_object_method(gmv.pipe_block_get_node, &[level.raw, pos.raw]).unwrap() else { return true };
    if !pipe_block.call_bool_method(gmv.pipe_block_can_connect, &[pipe_node.raw, write_dir(jni, dir).raw, tile.raw]).unwrap() {
        return true;
    }
    pipe_node.call_void_method(gmv.pipe_node_set_connection, &[dir_obj.raw, 1, 0]).unwrap();
    true
}
