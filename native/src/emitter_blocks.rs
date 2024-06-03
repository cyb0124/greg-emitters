use crate::{
    asm::*,
    cleaner::Cleanable,
    client_utils::{read_pose, DrawContext},
    geometry::{lerp, new_voxel_shape, DIR_ATTS},
    global::{GlobalObjs, Tier},
    jvm::*,
    mapping_base::{Functor, MBOptExt},
    objs,
    registry::{forge_reg, EMITTER_ID},
    tile_utils::{tile_get_update_packet_impl, TAG_COMMON},
};
use alloc::{format, sync::Arc};
use bstr::BStr;
use core::{
    array,
    cell::RefCell,
    f32::consts::{PI, TAU},
};
use macros::dyn_abi;
use nalgebra::{point, vector, Affine3, Point, Scale3, Translation3, UnitQuaternion};
use serde::{Deserialize, Serialize};
use simba::scalar::SupersetOf;

const RADIUS: f32 = 0.25;

pub struct EmitterBlocks {
    block_tier: usize,
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
            mn.tile_save_additional.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        ];
        let natives = [
            mn.tile_get_update_tag.native(get_update_tag_dyn()),
            mn.tile_load.native(on_load_dyn()),
            mn.tile_save_additional.native(save_additional_dyn()),
        ];
        cls.class_methods(av).unwrap().collection_extend(&av.jv, methods).unwrap();
        cls = av.ldr.with_jni(jni).define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
        cls.register_natives(&natives).unwrap();
        let tile_cls = cls.new_global_ref().unwrap();

        // Block
        name = namer.next();
        cls = av.new_class_node(jni, &name.slash, &cn.base_tile_block.slash).unwrap();
        cls.class_fields(av).unwrap().collection_extend(&av.jv, [av.new_field_node(jni, c"0", c"B", 0, 0).unwrap()]).unwrap();
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
        let block_tier = cls.get_field_id(c"0", c"B").unwrap();
        let mut props = mv.block_beh_props.with_jni(jni).call_static_object_method(mv.block_beh_props_of, &[]).unwrap().unwrap();
        props = props.call_object_method(mv.block_beh_props_strength, &[f_raw(0.25), f_raw(1E6)]).unwrap().unwrap();
        props = props.call_object_method(mv.block_beh_props_dyn_shape, &[]).unwrap().unwrap();
        props = props.call_object_method(mv.block_beh_props_sound, &[mv.sound_type_metal.raw]).unwrap().unwrap();
        let n_emitter_tiers = tiers.iter().filter(|x| x.has_emitter).count();
        let mut blocks = cls.new_object_array(n_emitter_tiers as _, 0).unwrap();
        for (block_i, (tier_i, tier)) in tiers.iter_mut().enumerate().filter(|(_, x)| x.has_emitter).enumerate() {
            let true = tier.has_emitter else { continue };
            let block = cls.new_object(mv.base_tile_block_init, &[props.raw]).unwrap();
            block.set_byte_field(block_tier, tier_i as _);
            tier.emitter_block = Some(block.new_global_ref().unwrap());
            blocks.set_object_elem(block_i as _, block.raw).unwrap();
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
        let center = point![0.5, 0.5, 0.5];
        let shapes = DIR_ATTS.map(|at| {
            let p0 = (center + at * vector![-RADIUS, -RADIUS, -0.5]).coords;
            let p1 = (center + at * vector![RADIUS, RADIUS, RADIUS]).coords;
            new_voxel_shape(jni, Point { coords: p0.zip_map(&p1, f32::min) }, Point { coords: p0.zip_map(&p1, f32::max) })
        });

        Self {
            block_tier,
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
    let GlobalObjs { mtx, mv, .. } = objs();
    let tile = BorrowedRef::new(jni, &tile);
    let state = tile.call_object_method(mv.tile_get_block_state, &[]).unwrap().unwrap();
    let block = state.call_object_method(mv.block_state_get_block, &[]).unwrap().unwrap();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.uref();
    let tier = block.get_byte_field(defs.block_tier);
    let emitter = defs.from_tile(&tile);
    let common = emitter.common.borrow();
    let Some(dir) = common.dir else { return };
    let mut dc = DrawContext::new(&*lk, &BorrowedRef::new(jni, &buffer_source), light, overlay);
    let tf = read_pose(&BorrowedRef::new(jni, &pose_stack)) * Translation3::new(0.5, 0.5, 0.5) * DIR_ATTS[dir as usize] * DIR_ATTS[0];
    // Legs
    const LEG_LEN: f32 = 0.3;
    const LEG_DIA: f32 = 0.05;
    const LEG_POS: f32 = RADIUS * 0.6;
    let greg_wire = lk.greg_wire.uref();
    let leg_side = greg_wire.sub(0., 0., LEG_DIA, LEG_LEN);
    let leg_bot = greg_wire.sub(0., 0., LEG_DIA, LEG_DIA);
    for x in [-LEG_POS, LEG_POS] {
        let tf = tf * Translation3::new(x, 0., 0.);
        dc.square(&leg_bot, &(tf * Translation3::new(0., -0.5, 0.) * DIR_ATTS[0] * Affine3::from_subset(&Scale3::new(LEG_DIA, LEG_DIA, 1.))));
        let mut face = Translation3::new(0., LEG_LEN * 0.5 - 0.5, LEG_DIA * 0.5) * Affine3::from_subset(&Scale3::new(LEG_DIA, LEG_LEN, 1.));
        for _ in 0..4 {
            dc.square(&leg_side, &(tf * face));
            face = DIR_ATTS[4] * face;
        }
    }
    // Cylinder (r, h, v)
    const CONTOUR: [(f32, f32, f32); 4] = [(1., 0., 0.), (1., 1., 1.), (0.9, 1., 0.8), (0.6, 0.8, 0.6)];
    const N_SEGS: usize = 8;
    let base = vector![RADIUS, libm::tanf(PI / N_SEGS as f32) * RADIUS];
    let bot_y = LEG_LEN - 0.5;
    let bot_q = tf * point![0., bot_y, 0.];
    let bot_m = tf * vector![0., -1., 0.];
    let top_y = RADIUS;
    let top_p = point![0., lerp(bot_y, top_y, 0.7), 0.];
    let top_q = tf * top_p;
    let mut p0 = CONTOUR.map(|(r, h, _)| point![base.x * r, lerp(bot_y, top_y, h), base.y * r]);
    let mut q0 = p0.map(|p| tf * p);
    let mut n0: [_; 4] = array::from_fn(|i| (p0.get(i + 1).unwrap_or(&top_p) - p0[i]).cross(&vector![-base.y, 0., base.x]).normalize());
    let mut m0 = n0.map(|n| tf * n);
    let rot = UnitQuaternion::from_euler_angles(0., TAU / N_SEGS as f32, 0.);
    let spr = lk.tiers[tier as usize].emitter_sprite.uref().sub(0.4, 0.2, 0.6, 0.4);
    for _ in 0..N_SEGS / 2 {
        let (p1, n1) = (p0.map(|p| rot * p), n0.map(|n| rot * n));
        let (p2, n2) = (p1.map(|p| rot * p), n1.map(|n| rot * n));
        let (q1, m1) = (p1.map(|p| tf * p), n1.map(|n| tf * n));
        let (q2, m2) = (p2.map(|p| tf * p), n2.map(|n| tf * n));
        // Side Contour
        for i in 0..CONTOUR.len() - 1 {
            let v0 = spr.lerp_v(CONTOUR[i].2);
            let v1 = spr.lerp_v(CONTOUR[i + 1].2);
            dc.vertex(q0[i], m0[i], spr.uv0.x, v0);
            dc.vertex(q1[i], m1[i], spr.uv1.x, v0);
            dc.vertex(q1[i + 1], m1[i], spr.uv1.x, v1);
            dc.vertex(q0[i + 1], m0[i], spr.uv0.x, v1);
            dc.vertex(q1[i], m1[i], spr.uv1.x, v0);
            dc.vertex(q2[i], m2[i], spr.uv0.x, v0);
            dc.vertex(q2[i + 1], m2[i], spr.uv0.x, v1);
            dc.vertex(q1[i + 1], m1[i], spr.uv1.x, v1);
        }
        // Bottom Cap
        let v = spr.lerp_v(CONTOUR[0].2);
        dc.vertex(bot_q, bot_m, spr.uv1.x, spr.uv1.y);
        dc.vertex(q2[0], bot_m, spr.uv0.x, v);
        dc.vertex(q1[0], bot_m, spr.uv1.x, v);
        dc.vertex(q0[0], bot_m, spr.uv0.x, v);
        // Top Cap
        let v = spr.lerp_v(CONTOUR.last().unwrap().2);
        dc.vertex(top_q, *m1.last().unwrap(), spr.uv1.x, spr.uv1.y);
        dc.vertex(*q0.last().unwrap(), *m0.last().unwrap(), spr.uv0.x, v);
        dc.vertex(*q1.last().unwrap(), *m1.last().unwrap(), spr.uv1.x, v);
        dc.vertex(*q2.last().unwrap(), *m2.last().unwrap(), spr.uv1.x, v);
        (p0 = p2, q0 = q2, n0 = n2, m0 = m2);
    }
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
    nbt.call_void_method(mv.nbt_compound_put_byte_array, &[jni.new_utf(TAG_COMMON).unwrap().raw, ba.raw]).unwrap();
    nbt.into_raw()
}

#[dyn_abi]
fn save_additional(jni: &JNI, tile: usize, nbt: usize) {
    let tile = BorrowedRef::new(jni, &tile);
    let nbt = BorrowedRef::new(jni, &nbt);
    let GlobalObjs { mv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.uref();
    tile.call_nonvirtual_void_method(mv.tile.raw, mv.tile_save_additional, &[nbt.raw]).unwrap();
    let emitter = defs.from_tile(&tile);
    let data = postcard::to_allocvec(&emitter.common).unwrap();
    let ba = jni.new_byte_array(data.len() as _).unwrap();
    ba.write_byte_array(&data, 0).unwrap();
    nbt.call_void_method(mv.nbt_compound_put_byte_array, &[jni.new_utf(TAG_COMMON).unwrap().raw, ba.raw]).unwrap()
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
    let blob = nbt.call_object_method(mv.nbt_compound_get_byte_array, &[jni.new_utf(TAG_COMMON).unwrap().raw]).unwrap().unwrap();
    let data = blob.byte_elems().unwrap();
    if data.len() != 0 {
        Common::deserialize_in_place(&mut postcard::Deserializer::from_bytes(&*data), &mut *emitter.common.borrow_mut()).unwrap()
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Common {
    dir: Option<u8>,
    polar: f32,
    azimuth: f32,
}

struct Emitter {
    common: RefCell<Common>,
}

impl Cleanable for Emitter {
    fn free(self: Arc<Self>, jni: &JNI) {
        // TODO:
    }
}
