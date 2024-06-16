use crate::{
    global::GlobalObjs,
    jvm::*,
    objs,
    util::{
        cleaner::Cleanable,
        geometry::{write_block_pos, write_dir, GeomExt, DIR_STEPS},
        ClassBuilder, ThinWrapper,
    },
};
use alloc::sync::Arc;
use macros::dyn_abi;

pub struct EmitterItems {
    pub item: GlobalRef<'static>,
    item_factory: ThinWrapper<ItemFactory>,
}

struct ItemFactory {
    tier: u8,
}

impl Cleanable for ItemFactory {
    fn free(self: Arc<Self>, _: &JNI) {}
}

impl EmitterItems {
    pub fn new(jni: &'static JNI) -> Self {
        let GlobalObjs { cn, mn, gcn, .. } = objs();
        let item = ClassBuilder::new_2(jni, &cn.block_item.slash)
            .native_2(&mn.item_get_desc_id, get_desc_id_dyn())
            .native_2(&mn.block_item_place_block, place_block_dyn())
            .define_empty();
        let item_factory = ClassBuilder::new_2(jni, c"java/lang/Object")
            .interfaces([&*gcn.non_null_fn.slash])
            .native_1(c"apply", c"(Ljava/lang/Object;)Ljava/lang/Object;", build_item_dyn())
            .define_thin()
            .wrap::<ItemFactory>();
        Self { item, item_factory }
    }

    pub fn new_item_factory<'a>(&self, jni: &'a JNI, tier: u8) -> LocalRef<'a> { self.item_factory.new_obj(jni, ItemFactory { tier }.into()) }
}

#[dyn_abi]
fn build_item(jni: &'static JNI, this: usize, props: usize) -> usize {
    let lk = objs().mtx.lock(jni).unwrap();
    let defs = lk.emitter_items.get().unwrap();
    let tiers = lk.tiers.borrow();
    let tier = &tiers[defs.item_factory.read(&BorrowedRef::new(jni, &this)).tier as usize];
    let item = defs.item.with_jni(jni).new_object(objs().mv.block_item_init, &[tier.emitter_block.get().unwrap().raw, props]).unwrap();
    tier.emitter_item.set(item.new_global_ref().unwrap()).ok().unwrap();
    item.into_raw()
}

#[dyn_abi]
fn get_desc_id(jni: &JNI, this: usize) -> usize {
    let mv = &objs().mv;
    BorrowedRef::new(jni, &this).call_nonvirtual_object_method(mv.item.raw, mv.item_get_desc_id, &[]).unwrap().unwrap().into_raw()
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
    let mut dir = dir_obj.read_dir();
    let tile = level.call_object_method(mv.block_getter_get_tile, &[pos.raw]).unwrap().unwrap();
    let lk = mtx.lock(jni).unwrap();
    lk.emitter_blocks.get().unwrap().from_tile(&tile).common.borrow_mut().dir = Some(dir);
    dir ^= 1;
    pos = write_block_pos(jni, pos.read_vec3i() + DIR_STEPS[dir as usize]);
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
