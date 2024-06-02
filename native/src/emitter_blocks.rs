use core::cell::RefCell;

use crate::{
    asm::*,
    cleaner::Cleanable,
    client_utils::{read_pose, DrawContext},
    geometry::{new_voxel_shape, DIR_ATTS},
    global::{GlobalObjs, Tier},
    jvm::*,
    mapping_base::{Functor, MBOptExt},
    objs,
    registry::{forge_reg, EMITTER_ID},
    tile_utils::{tile_get_update_packet_impl, TAG_CLIENT},
};
use alloc::{format, sync::Arc};
use bstr::BStr;
use macros::dyn_abi;
use nalgebra::{point, vector, Point, Translation3};
use serde::{Deserialize, Serialize};

pub struct EmitterBlocks {
    tile_cls: GlobalRef<'static>,
    tile_p: usize,
    pub tile_type: GlobalRef<'static>,
    pub renderer_provider: Option<GlobalRef<'static>>,
    shapes: [GlobalRef<'static>; 6],
    shape_fallback: GlobalRef<'static>,
}

impl EmitterBlocks {
    pub fn init(jni: &'static JNI, tiers: &mut [Tier], reg_evt: &impl JRef<'static>) -> Self {
        // Tile
        let GlobalObjs { av, cn, mn, mv, namer, tile_utils, .. } = objs();
        let mut name = namer.next();
        let mut cls = av.new_class_node(jni, &name.slash, &cn.tile.slash).unwrap();
        cls.class_fields(av).unwrap().collection_extend(&av.jv, [av.new_field_node(jni, c"0", c"J", 0, 0).unwrap()]).unwrap();
        let methods = [
            tile_get_update_packet_impl(jni),
            mn.tile_get_update_tag.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.tile_load.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        ];
        cls.class_methods(av).unwrap().collection_extend(&av.jv, methods).unwrap();
        cls = av.ldr.with_jni(jni).define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
        cls.register_natives(&[mn.tile_get_update_tag.native(get_update_tag_dyn()), mn.tile_load.native(on_load_dyn())]).unwrap();
        let tile_cls = cls.new_global_ref().unwrap();

        // Block
        name = namer.next();
        cls = av.new_class_node(jni, &name.slash, &cn.base_tile_block.slash).unwrap();
        let methods = [
            mn.tile_block_new_tile.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.block_beh_get_render_shape.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.block_beh_get_shape.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.block_set_placed_by.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        ];
        let natives = [
            mn.tile_block_new_tile.native(new_tile_dyn()),
            mn.block_beh_get_render_shape.native(get_render_shape_dyn()),
            mn.block_beh_get_shape.native(get_shape_dyn()),
            mn.block_set_placed_by.native(set_placed_by_dyn()),
        ];
        (cls.class_methods(av).unwrap()).collection_extend(&av.jv, methods).unwrap();
        cls = av.ldr.with_jni(jni).define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
        cls.register_natives(&natives).unwrap();
        let mut props = mv.block_beh_props.with_jni(jni).call_static_object_method(mv.block_beh_props_of, &[]).unwrap().unwrap();
        props = props.call_object_method(mv.block_beh_props_strength, &[f_raw(0.25), f_raw(1E6)]).unwrap().unwrap();
        props = props.call_object_method(mv.block_beh_props_dyn_shape, &[]).unwrap().unwrap();
        props = props.call_object_method(mv.block_beh_props_sound, &[mv.sound_type_metal.raw]).unwrap().unwrap();
        let n_emitter_tiers = tiers.iter().filter(|x| x.has_emitter).count();
        let mut blocks = cls.new_object_array(n_emitter_tiers as _, 0).unwrap();
        for (i, tier) in tiers.iter_mut().filter(|x| x.has_emitter).enumerate() {
            let true = tier.has_emitter else { continue };
            let block = cls.new_object(mv.base_tile_block_init, &[props.raw]).unwrap();
            tier.emitter_block = Some(block.new_global_ref().unwrap());
            blocks.set_object_elem(i as _, block.raw).unwrap();
            forge_reg(reg_evt, &format!("{EMITTER_ID}_{}", BStr::new(&*tier.name)), block.raw);
        }
        blocks = blocks.set_of(&av.jv).unwrap();

        // Renderer
        let renderer_provider = mv.client.fmap(|_| {
            name = namer.next();
            cls = av.new_class_node(jni, &name.slash, c"java/lang/Object").unwrap();
            cls.add_interfaces(av, [&*cn.tile_renderer_provider.slash, &cn.tile_renderer.slash]).unwrap();
            let create = mn.tile_renderer_provider_create.new_method_node(av, jni, ACC_PUBLIC).unwrap();
            let insns = [av.new_var_insn(jni, OP_ALOAD, 0).unwrap(), av.new_insn(jni, OP_ARETURN).unwrap()];
            create.method_insns(av).unwrap().append_insns(av, insns).unwrap();
            let methods = [create, mn.tile_renderer_render.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap()];
            cls.class_methods(av).unwrap().collection_extend(&av.jv, methods).unwrap();
            cls = av.ldr.with_jni(jni).define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
            cls.register_natives(&[mn.tile_renderer_render.native(render_tile_dyn())]).unwrap();
            cls.alloc_object().unwrap().new_global_ref().unwrap()
        });

        // Shapes
        const RADIUS: f32 = 0.3;
        let center = point![0.5, 0.5, 0.5];
        let shapes = DIR_ATTS.map(|at| {
            let p0 = (center + at * vector![-RADIUS, -RADIUS, -0.5]).coords;
            let p1 = (center + at * vector![RADIUS, RADIUS, RADIUS]).coords;
            new_voxel_shape(jni, Point { coords: p0.zip_map(&p1, f32::min) }, Point { coords: p0.zip_map(&p1, f32::max) })
        });

        Self {
            tile_p: tile_cls.get_field_id(c"0", c"J").unwrap(),
            tile_cls,
            tile_type: tile_utils.define_tile_type(&blocks, |jni, pos, state| new_tile(jni, 0, pos, state)),
            renderer_provider,
            shapes,
            shape_fallback: new_voxel_shape(jni, center.map(|x| x - RADIUS), center.map(|x| x + RADIUS)),
        }
    }

    fn from_tile<'a>(&self, tile: &impl JRef<'a>) -> &'a Emitter { unsafe { &*(tile.get_long_field(self.tile_p) as *const Emitter) } }
}

#[dyn_abi]
fn get_shape(jni: &JNI, _this: usize, _state: usize, level: usize, pos: usize, _collision_ctx: usize) -> usize {
    let GlobalObjs { mv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.uref();
    // Rethrow is needed for lithium's SingleBlockBlockView.
    match BorrowedRef::new(jni, &level).call_object_method(mv.block_getter_get_tile, &[pos]) {
        Ok(Some(tile)) => defs.from_tile(&tile).common.borrow().dir.map_or(defs.shape_fallback.raw, |i| defs.shapes[i as usize].raw),
        Ok(None) => defs.shape_fallback.raw,
        Err(JVMError::Throwable(e)) => e.throw().map(|_| 0).unwrap(),
        Err(e) => panic!("{e}"),
    }
}

#[dyn_abi]
fn get_render_shape(_: &JNI, _this: usize, _state: usize) -> usize { objs().mv.render_shape_tile.raw }

#[dyn_abi]
fn render_tile(jni: &JNI, _: usize, tile: usize, _: f32, pose_stack: usize, buffer_source: usize, light: i32, overlay: i32) {
    let lk = objs().mtx.lock(jni).unwrap();
    let sprites = lk.sprites.uref();
    let mut dc = DrawContext::new(sprites, &BorrowedRef::new(jni, &buffer_source), light, overlay);
    let tf = read_pose(&BorrowedRef::new(jni, &pose_stack));
    let tf = tf * Translation3::new(0.5, 0.5, 0.5);
    dc.square(&sprites.greg_wire, &tf)
    // TODO:
}

#[dyn_abi]
fn new_tile(jni: &JNI, _this: usize, pos: usize, state: usize) -> usize {
    let GlobalObjs { mv, mtx, cleaner, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.uref();
    let tile = defs.tile_cls.with_jni(jni).new_object(mv.tile_init, &[defs.tile_type.raw, pos, state]).unwrap();
    let emitter = Arc::new(Emitter { common: <_>::default() });
    tile.set_long_field(defs.tile_p, &*emitter as *const _ as _);
    cleaner.reg(&tile, emitter);
    tile.into_raw()
}

#[dyn_abi]
fn set_placed_by(jni: &JNI, _this: usize, level: usize, pos: usize, _state: usize, _placer: usize, _item_stack: usize) {
    let GlobalObjs { mv, mtx, .. } = objs();
    let mut lk = mtx.lock(jni).unwrap();
    let use_on_ctx = lk.emitter_items.as_mut().unwrap().use_on_ctx.take().unwrap().replace_jni(jni);
    let dir = use_on_ctx.call_object_method(mv.use_on_ctx_get_clicked_face, &[]).unwrap().unwrap();
    let dir = dir.call_int_method(mv.dir_get_3d_value, &[]).unwrap() as u8;
    let tile = BorrowedRef::new(jni, &level).call_object_method(mv.block_getter_get_tile, &[pos]).unwrap().unwrap();
    lk.emitter_blocks.uref().from_tile(&tile).common.borrow_mut().dir = Some(dir)
}

#[dyn_abi]
fn get_update_tag(jni: &JNI, tile: usize) -> usize {
    let GlobalObjs { mv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.uref();
    let nbt = mv.nbt_compound.with_jni(jni).new_object(mv.nbt_compound_init, &[]).unwrap();
    let emitter = defs.from_tile(&BorrowedRef::new(jni, &tile));
    let data = postcard::to_allocvec(&emitter.common).unwrap();
    let ba = jni.new_byte_array(data.len() as _).unwrap();
    ba.write_byte_array(&data, 0).unwrap();
    nbt.call_void_method(mv.nbt_compound_put_byte_array, &[jni.new_utf(TAG_CLIENT).unwrap().raw, ba.raw]).unwrap();
    nbt.into_raw()
}

#[dyn_abi]
fn on_load(jni: &JNI, tile: usize, nbt: usize) {
    let GlobalObjs { mv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.uref();
    let tile = BorrowedRef::new(jni, &tile);
    let nbt = BorrowedRef::new(jni, &nbt);
    tile.call_nonvirtual_void_method(mv.tile.raw, mv.tile_load, &[nbt.raw]).unwrap();
    let emitter = defs.from_tile(&tile);
    let blob = nbt.call_object_method(mv.nbt_compound_get_byte_array, &[jni.new_utf(TAG_CLIENT).unwrap().raw]).unwrap().unwrap();
    let data = blob.byte_elems().unwrap();
    if data.len() != 0 {
        Common::deserialize_in_place(&mut postcard::Deserializer::from_bytes(&*data), &mut *emitter.common.borrow_mut()).unwrap()
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Common {
    dir: Option<u8>,
}

struct Emitter {
    common: RefCell<Common>,
}

impl Cleanable for Emitter {
    fn free(self: Arc<Self>, jni: &JNI) {
        // TODO:
    }
}
