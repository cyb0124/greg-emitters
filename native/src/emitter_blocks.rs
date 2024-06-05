use crate::{
    asm::*,
    cleaner::Cleanable,
    client_utils::{read_pose, DrawContext},
    geometry::{lerp, new_voxel_shape, read_dir, DIR_ATTS},
    global::{GlobalObjs, Tier},
    jvm::*,
    mapping_base::*,
    objs,
    registry::{forge_reg, EMITTER_ID},
    tile_utils::{const_long_impl, non_null_supplier_get_self_impl, read_tag, tile_get_update_packet_impl, write_tag, TAG_COMMON, TAG_SERVER},
};
use alloc::{format, sync::Arc};
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
    tile_energy_container_cap: usize,
    pub tile_type: GlobalRef<'static>,
    pub renderer_provider: Option<GlobalRef<'static>>,
    shapes: [GlobalRef<'static>; 6],
    shape_fallback: GlobalRef<'static>,
}

impl EmitterBlocks {
    pub fn init(jni: &'static JNI, tiers: &mut [Tier], reg_evt: &impl JRef<'static>) -> Self {
        // Tile
        let GlobalObjs { av, cn, mn, mv, fcn, fmn, gcn, gmn, namer, tile_utils, .. } = objs();
        let mut name = namer.next();
        let mut cls = av.new_class_node(jni, &name.slash, &cn.tile.slash).unwrap();
        cls.add_interfaces(av, [&*fcn.non_null_supplier.slash, &*gcn.energy_container.slash]).unwrap();
        let fields = [av.new_field_node(jni, c"0", c"J", 0, 0).unwrap(), av.new_field_node(jni, c"0", &fcn.lazy_opt.sig, 0, 0).unwrap()];
        cls.class_fields(av).unwrap().collection_extend(&av.jv, fields).unwrap();
        let methods = [
            tile_get_update_packet_impl(jni),
            non_null_supplier_get_self_impl(jni),
            mn.tile_get_update_tag.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.tile_load.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.tile_save_additional.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            fmn.get_cap.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            fmn.invalidate_caps.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            gmn.can_input_eu_from_side.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            gmn.accept_eu.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            gmn.change_eu.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            gmn.get_eu_stored.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            gmn.get_eu_capacity.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            const_long_impl(jni, &gmn.get_input_amps, 1),
            gmn.get_input_volts.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        ];
        let natives = [
            mn.tile_get_update_tag.native(get_update_tag_dyn()),
            mn.tile_load.native(on_load_dyn()),
            mn.tile_save_additional.native(save_additional_dyn()),
            fmn.get_cap.native(get_cap_dyn()),
            fmn.invalidate_caps.native(invalidate_caps_dyn()),
            gmn.can_input_eu_from_side.native(can_input_eu_from_side_dyn()),
            gmn.accept_eu.native(accept_eu_dyn()),
            gmn.change_eu.native(change_eu_dyn()),
            gmn.get_eu_stored.native(get_eu_stored_dyn()),
            gmn.get_eu_capacity.native(get_eu_capacity_dyn()),
            gmn.get_input_volts.native(get_input_volts_dyn()),
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
            mn.block_beh_get_drops.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        ];
        let natives = [
            mn.tile_block_new_tile.native(new_tile_dyn()),
            mn.block_beh_get_render_shape.native(get_render_shape_dyn()),
            mn.block_beh_get_shape.native(get_shape_dyn()),
            mn.block_set_placed_by.native(set_placed_by_dyn()),
            mn.block_beh_get_drops.native(get_drops_dyn()),
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
            blocks.set_object_elem(block_i as _, block.raw).unwrap();
            block.set_byte_field(block_tier, tier_i as _);
            tier.emitter_block.set(block.new_global_ref().unwrap()).ok().unwrap();
            forge_reg(reg_evt, &format!("{EMITTER_ID}_{}", tier.name), block.raw);
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
            tile_energy_container_cap: tile_cls.get_field_id(c"0", &fcn.lazy_opt.sig).unwrap(),
            tile_cls,
            tile_type: tile_utils.define_tile_type(&blocks, |jni, pos, state| new_tile(jni, 0, pos, state)),
            renderer_provider,
            shapes,
            shape_fallback: new_voxel_shape(jni, center.map(|x| x - RADIUS), center.map(|x| x + RADIUS)),
        }
    }

    // Borrowing self to ensure lock is held.
    fn from_tile<'a>(&'a self, tile: &impl JRef<'a>) -> &'a Emitter { unsafe { &*(tile.get_long_field(self.tile_p) as *const Emitter) } }
}

#[dyn_abi]
fn get_drops(jni: &JNI, this: usize, _state: usize, _loot_builder: usize) -> usize {
    let GlobalObjs { mtx, av, mv, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.get().unwrap();
    let tier = BorrowedRef::new(jni, &this).get_byte_field(defs.block_tier);
    let item = lk.tiers[tier as usize].emitter_item.get().unwrap();
    let stack = mv.item_stack.with_jni(jni).new_object(mv.item_stack_init, &[item.raw, 1, 0]).unwrap();
    mv.item_stack.with_jni(jni).new_object_array(1, stack.raw).unwrap().array_as_list(&av.jv).unwrap().into_raw()
}

#[dyn_abi]
fn get_shape(jni: &JNI, _this: usize, _state: usize, level: usize, pos: usize, _collision_ctx: usize) -> usize {
    let GlobalObjs { mv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.get().unwrap();
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
    let tile = BorrowedRef::new(jni, &tile);
    let lk = objs().mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.get().unwrap();
    let emitter = defs.from_tile(&tile);
    let common = emitter.common.borrow();
    let Some(dir) = common.dir else { return };
    let mut dc = DrawContext::new(&*lk, &BorrowedRef::new(jni, &buffer_source), light, overlay);
    let tf = read_pose(&BorrowedRef::new(jni, &pose_stack)) * Translation3::new(0.5, 0.5, 0.5) * DIR_ATTS[dir as usize] * DIR_ATTS[0];
    // Legs
    const LEG_LEN: f32 = 0.3;
    const LEG_DIA: f32 = 0.05;
    const LEG_POS: f32 = RADIUS * 0.6;
    let greg_wire = lk.wire_sprite.uref();
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
    let spr = lk.tiers[emitter.tier as usize].emitter_sprite.uref().sub(0.4, 0.2, 0.6, 0.4);
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
fn get_cap(jni: &JNI, this: usize, cap: usize, side: usize) -> usize {
    let GlobalObjs { fmv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let this = BorrowedRef::new(jni, &this);
    if BorrowedRef::new(jni, &cap).is_same_object(lk.gmv.get().unwrap().energy_container_cap.raw) {
        this.get_object_field(lk.emitter_blocks.get().unwrap().tile_energy_container_cap).unwrap().into_raw()
    } else {
        this.call_nonvirtual_object_method(fmv.cap_provider.raw, fmv.get_cap, &[cap, side]).unwrap().unwrap().into_raw()
    }
}

#[dyn_abi]
fn invalidate_caps(jni: &JNI, this: usize) {
    let GlobalObjs { fmv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.get().unwrap();
    let this = BorrowedRef::new(jni, &this);
    this.call_nonvirtual_void_method(fmv.cap_provider.raw, fmv.invalidate_caps, &[]).unwrap();
    this.get_object_field(defs.tile_energy_container_cap).unwrap().call_void_method(fmv.lazy_opt_invalidate, &[]).unwrap()
}

#[dyn_abi]
fn get_eu_stored(jni: &JNI, this: usize) -> i64 {
    objs().mtx.lock(jni).unwrap().emitter_blocks.get().unwrap().from_tile(&BorrowedRef::new(jni, &this)).server.borrow().energy
}

#[dyn_abi]
fn get_eu_capacity(jni: &JNI, this: usize) -> i64 {
    let lk = objs().mtx.lock(jni).unwrap();
    lk.emitter_blocks.get().unwrap().from_tile(&BorrowedRef::new(jni, &this)).eu_capacity(&lk.tiers)
}

#[dyn_abi]
fn get_input_volts(jni: &JNI, this: usize) -> i64 {
    let lk = objs().mtx.lock(jni).unwrap();
    lk.emitter_blocks.get().unwrap().from_tile(&BorrowedRef::new(jni, &this)).volts(&lk.tiers)
}

#[dyn_abi]
fn can_input_eu_from_side(jni: &JNI, this: usize, in_side: usize) -> bool {
    let lk = objs().mtx.lock(jni).unwrap();
    let emitter = lk.emitter_blocks.get().unwrap().from_tile(&BorrowedRef::new(jni, &this));
    let result = Some(read_dir(&BorrowedRef::new(jni, &in_side)) ^ 1) == emitter.common.borrow().dir;
    result
}

#[dyn_abi]
fn accept_eu(jni: &JNI, this: usize, in_side: usize, volts: i64, amps: i64) -> i64 {
    if amps < 1 {
        return 0;
    }
    let GlobalObjs { mv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let this = BorrowedRef::new(jni, &this);
    let emitter = lk.emitter_blocks.get().unwrap().from_tile(&this);
    if in_side != 0 && Some(read_dir(&BorrowedRef::new(jni, &in_side)) ^ 1) != emitter.common.borrow().dir {
        return 0;
    }
    if volts > emitter.volts(&lk.tiers) {
        let level = this.get_object_field(mv.tile_level).unwrap();
        let pos = this.get_object_field(mv.tile_pos).unwrap();
        let state = mv.blocks_fire.with_jni(jni).call_object_method(mv.block_default_state, &[]).unwrap().unwrap();
        level.call_bool_method(mv.level_set_block_and_update, &[pos.raw, state.raw]).unwrap();
        return 1;
    }
    let mut data = emitter.server.borrow_mut();
    if data.energy + volts > emitter.eu_capacity(&lk.tiers) {
        return 0;
    }
    data.energy += volts;
    1
}

#[dyn_abi]
fn change_eu(jni: &JNI, this: usize, delta: i64) -> i64 {
    let lk = objs().mtx.lock(jni).unwrap();
    let emitter = lk.emitter_blocks.get().unwrap().from_tile(&BorrowedRef::new(jni, &this));
    let mut data = emitter.server.borrow_mut();
    let base = data.energy;
    data.energy = (base + delta).clamp(0, emitter.eu_capacity(&lk.tiers));
    data.energy - base
}

#[dyn_abi]
fn new_tile(jni: &JNI, _this: usize, pos: usize, state: usize) -> usize {
    let GlobalObjs { mv, fmv, mtx, cleaner, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.get().unwrap();
    let block = BorrowedRef::new(jni, &state).call_object_method(mv.block_state_get_block, &[]).unwrap().unwrap();
    let tier = block.get_byte_field(defs.block_tier);
    let tile = defs.tile_cls.with_jni(jni).new_object(mv.tile_init, &[defs.tile_type.raw, pos, state]).unwrap();
    let cap = fmv.lazy_opt.with_jni(jni).call_static_object_method(fmv.lazy_opt_of, &[tile.raw]).unwrap().unwrap();
    tile.set_object_field(defs.tile_energy_container_cap, cap.raw);
    let emitter = Arc::new(Emitter { tier, common: <_>::default(), server: <_>::default() });
    tile.set_long_field(defs.tile_p, &*emitter as *const _ as _);
    cleaner.reg(&tile, emitter);
    tile.into_raw()
}

#[dyn_abi]
fn set_placed_by(jni: &JNI, _this: usize, level: usize, pos: usize, _state: usize, _placer: usize, _item_stack: usize) {
    let GlobalObjs { mv, mtx, .. } = objs();
    let mut lk = mtx.lock(jni).unwrap();
    let use_on_ctx = lk.emitter_items.get_mut().unwrap().use_on_ctx.take().unwrap().replace_jni(jni);
    let dir = read_dir(&use_on_ctx.call_object_method(mv.use_on_ctx_get_clicked_face, &[]).unwrap().unwrap());
    let tile = BorrowedRef::new(jni, &level).call_object_method(mv.block_getter_get_tile, &[pos]).unwrap().unwrap();
    lk.emitter_blocks.get().unwrap().from_tile(&tile).common.borrow_mut().dir = Some(dir);
}

#[dyn_abi]
fn get_update_tag(jni: &JNI, tile: usize) -> usize {
    let GlobalObjs { mv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let emitter = lk.emitter_blocks.get().unwrap().from_tile(&BorrowedRef::new(jni, &tile));
    let tag = mv.nbt_compound.with_jni(jni).new_object(mv.nbt_compound_init, &[]).unwrap();
    write_tag(&tag, TAG_COMMON, &emitter.common);
    tag.into_raw()
}

#[dyn_abi]
fn save_additional(jni: &JNI, tile: usize, nbt: usize) {
    let tile = BorrowedRef::new(jni, &tile);
    let tag = BorrowedRef::new(jni, &nbt);
    let GlobalObjs { mv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let emitter = lk.emitter_blocks.get().unwrap().from_tile(&tile);
    tile.call_nonvirtual_void_method(mv.tile.raw, mv.tile_save_additional, &[tag.raw]).unwrap();
    write_tag(&tag, TAG_COMMON, &emitter.common);
    write_tag(&tag, TAG_SERVER, &emitter.server)
}

#[dyn_abi]
fn on_load(jni: &JNI, tile: usize, nbt: usize) {
    let GlobalObjs { mv, mtx, .. } = objs();
    let tile = BorrowedRef::new(jni, &tile);
    let tag = BorrowedRef::new(jni, &nbt);
    tile.call_nonvirtual_void_method(mv.tile.raw, mv.tile_load, &[tag.raw]).unwrap();
    let lk = mtx.lock(jni).unwrap();
    let emitter = lk.emitter_blocks.get().unwrap().from_tile(&tile);
    let _ = read_tag(&tag, TAG_COMMON, &mut *emitter.common.borrow_mut()) && read_tag(&tag, TAG_SERVER, &mut *emitter.server.borrow_mut());
}

#[derive(Default, Serialize, Deserialize)]
struct CommonData {
    dir: Option<u8>,
    polar: f32,
    azimuth: f32,
}

#[derive(Default, Serialize, Deserialize)]
struct ServerData {
    energy: i64,
}

struct Emitter {
    tier: u8,
    common: RefCell<CommonData>,
    server: RefCell<ServerData>,
}

impl Cleanable for Emitter {
    fn free(self: Arc<Self>, _: &JNI) {}
}

impl Emitter {
    fn volts(&self, tiers: &[Tier]) -> i64 { tiers[self.tier as usize].volt }
    fn eu_capacity(&self, tiers: &[Tier]) -> i64 { self.volts(tiers) * 2 }
}
