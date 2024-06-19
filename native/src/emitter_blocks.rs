use crate::{
    asm::*,
    global::{GlobalMtx, GlobalObjs, Tier},
    jvm::*,
    mapping_base::*,
    objs,
    registry::{forge_reg, EMITTER_ID},
    util::{
        cleaner::Cleanable,
        client::{ClientExt, DrawContext},
        geometry::{lerp, new_voxel_shape, GeomExt, DIR_ATTS},
        tile::{Tile, TileExt, TileSupplier},
        ClassBuilder, ThinWrapper,
    },
};
use alloc::{format, sync::Arc, vec::Vec};
use core::{
    any::Any,
    array,
    cell::{OnceCell, RefCell},
    f32::consts::{PI, TAU},
};
use macros::dyn_abi;
use nalgebra::{point, vector, Affine3, Point, Scale3, Translation3, UnitQuaternion};
use postcard::{de_flavors::Slice, Deserializer, Result};
use serde::{Deserialize, Serialize};
use simba::scalar::SupersetOf;

const RADIUS: f32 = 0.25;

pub struct EmitterBlocks {
    block_tier: usize,
    pub tile_type: GlobalRef<'static>,
    energy_container: ThinWrapper<EnergyContainer>,
    pub renderer_provider: Option<GlobalRef<'static>>,
    shapes: [GlobalRef<'static>; 6],
    shape_fallback: GlobalRef<'static>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct CommonData {
    pub dir: Option<u8>,
    polar: f32,
    azimuth: f32,
}

#[derive(Default, Serialize, Deserialize)]
struct ServerData {
    energy: i64,
}

pub struct Emitter {
    tier: u8,
    energy_cap: GlobalRef<'static>,
    pub common: RefCell<CommonData>,
    server: RefCell<ServerData>,
}

impl Cleanable for Emitter {
    fn free(self: Arc<Self>, jni: &JNI) { Arc::into_inner(self).unwrap().energy_cap.replace_jni(jni); }
}

impl Emitter {
    fn volts(&self, tiers: &[Tier]) -> i64 { tiers[self.tier as usize].volt }
    fn eu_capacity(&self, tiers: &[Tier]) -> i64 { self.volts(tiers) * 2 }
}

struct EnergyContainer {
    tile: OnceCell<WeakGlobalRef<'static>>,
}

impl Cleanable for EnergyContainer {
    fn free(self: Arc<Self>, jni: &JNI) { Arc::into_inner(self).unwrap().tile.into_inner().unwrap().replace_jni(jni); }
}

struct EmitterSupplier;

impl EmitterBlocks {
    pub fn init(jni: &'static JNI, lk: &GlobalMtx, reg_evt: &impl JRef<'static>) -> Self {
        let GlobalObjs { av, cn, mn, mv, fcn, fmn, gcn, gmn, namer, tile_defs, .. } = objs();
        let energy_container = ClassBuilder::new_2(jni, c"java/lang/Object")
            .interfaces([&*fcn.non_null_supplier.slash, &*gcn.energy_container.slash])
            .insns(&fmn.non_null_supplier_get, [av.new_var_insn(jni, OP_ALOAD, 0).unwrap(), av.new_insn(jni, OP_ARETURN).unwrap()])
            .insns(&gmn.get_input_amps, [av.new_ldc_insn(jni, av.jv.wrap_long(jni, 1).unwrap().raw).unwrap(), av.new_insn(jni, OP_LRETURN).unwrap()])
            .native_2(&gmn.can_input_eu_from_side, can_input_eu_from_side_dyn())
            .native_2(&gmn.accept_eu, accept_eu_dyn())
            .native_2(&gmn.change_eu, change_eu_dyn())
            .native_2(&gmn.get_eu_stored, get_eu_stored_dyn())
            .native_2(&gmn.get_eu_capacity, get_eu_capacity_dyn())
            .native_2(&gmn.get_input_volts, get_input_volts_dyn())
            .define_thin()
            .wrap::<EnergyContainer>();

        // Block
        let mut name = namer.next();
        let mut cls = av.new_class_node(jni, &name.slash, &cn.base_tile_block.slash).unwrap();
        cls.class_fields(av).unwrap().collection_extend(&av.jv, [av.new_field_node(jni, c"0", c"B", 0, 0).unwrap()]).unwrap();
        let methods = [
            mn.tile_block_new_tile.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.block_beh_get_render_shape.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.block_beh_get_shape.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.block_beh_get_drops.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.block_beh_on_place.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        ];
        let natives = [
            mn.tile_block_new_tile.native(new_tile_dyn()),
            mn.block_beh_get_render_shape.native(get_render_shape_dyn()),
            mn.block_beh_get_shape.native(get_shape_dyn()),
            mn.block_beh_get_drops.native(get_drops_dyn()),
            mn.block_beh_on_place.native(on_place_dyn()),
        ];
        (cls.class_methods(av).unwrap()).collection_extend(&av.jv, methods).unwrap();
        cls = av.ldr.with_jni(jni).define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
        cls.register_natives(&natives).unwrap();
        let block_tier = cls.get_field_id(c"0", c"B").unwrap();
        let mut props = mv.block_beh_props.with_jni(jni).call_static_object_method(mv.block_beh_props_of, &[]).unwrap().unwrap();
        props = props.call_object_method(mv.block_beh_props_strength, &[f_raw(0.25), f_raw(1E6)]).unwrap().unwrap();
        props = props.call_object_method(mv.block_beh_props_dyn_shape, &[]).unwrap().unwrap();
        props = props.call_object_method(mv.block_beh_props_sound, &[mv.sound_type_metal.raw]).unwrap().unwrap();
        let tiers = lk.tiers.borrow();
        let n_emitter_tiers = tiers.iter().filter(|x| x.has_emitter).count();
        let mut blocks = cls.new_object_array(n_emitter_tiers as _, 0).unwrap();
        for (block_i, (tier_i, tier)) in tiers.iter().enumerate().filter(|(_, x)| x.has_emitter).enumerate() {
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
            tile_type: tile_defs.new_tile_type(jni, &EmitterSupplier, &blocks),
            energy_container,
            renderer_provider,
            shapes,
            shape_fallback: new_voxel_shape(jni, center.map(|x| x - RADIUS), center.map(|x| x + RADIUS)),
        }
    }
}

impl Tile for Emitter {
    fn any(&self) -> &dyn Any { self }
    fn invalidate_caps(&self, jni: &JNI) { self.energy_cap.with_jni(jni).lazy_opt_invalidate() }
    fn save_common(&self) -> Vec<u8> { postcard::to_allocvec(&*self.common.borrow()).unwrap() }
    fn save_server(&self) -> Vec<u8> { postcard::to_allocvec(&*self.server.borrow()).unwrap() }

    fn load_common<'a>(&self, de: &mut Deserializer<'a, Slice<'a>>) -> Result<()> {
        CommonData::deserialize_in_place(de, &mut *self.common.borrow_mut())
    }

    fn load_server<'a>(&self, de: &mut Deserializer<'a, Slice<'a>>) -> Result<()> {
        ServerData::deserialize_in_place(de, &mut *self.server.borrow_mut())
    }

    fn get_cap(&self, cap: BorrowedRef) -> Option<usize> {
        cap.is_same_object(objs().mtx.lock(cap.jni).unwrap().gmv.get().unwrap().energy_container_cap.raw).then(|| self.energy_cap.raw)
    }
}

impl TileSupplier for EmitterSupplier {
    fn new_tile(&self, lk: &GlobalMtx, pos: BorrowedRef<'static, '_>, state: BorrowedRef<'static, '_>) -> LocalRef<'static> {
        let defs = lk.emitter_blocks.get().unwrap();
        let block = state.block_state_get_block();
        let tier = block.get_byte_field(defs.block_tier);
        let energy_container = Arc::new(EnergyContainer { tile: OnceCell::new() });
        let energy_cap = defs.energy_container.new_obj(pos.jni, energy_container.clone()).lazy_opt_of().new_global_ref().unwrap();
        let emitter = Arc::new(Emitter { tier, energy_cap, common: <_>::default(), server: <_>::default() });
        let tile = objs().tile_defs.new_tile(pos.jni, defs.tile_type.raw, pos.raw, state.raw, emitter);
        energy_container.tile.set(tile.new_weak_global_ref().unwrap()).ok().unwrap();
        tile
    }
}

#[dyn_abi]
fn get_drops(jni: &JNI, this: usize, _state: usize, _loot_builder: usize) -> usize {
    let GlobalObjs { mtx, av, mv, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let tiers = lk.tiers.borrow();
    let tier = BorrowedRef::new(jni, &this).get_byte_field(lk.emitter_blocks.get().unwrap().block_tier);
    let item = tiers[tier as usize].emitter_item.get().unwrap();
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
        Ok(Some(tile)) => lk.read_tile::<Emitter>(tile.borrow()).common.borrow().dir.map_or(defs.shape_fallback.raw, |i| defs.shapes[i as usize].raw),
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
    let emitter = lk.read_tile::<Emitter>(BorrowedRef::new(jni, &tile));
    let common = emitter.common.borrow();
    let Some(dir) = common.dir else { return };
    let mut dc = DrawContext::new(&*lk, &BorrowedRef::new(jni, &buffer_source), light, overlay);
    let tf = BorrowedRef::new(jni, &pose_stack).last_pose() * Translation3::new(0.5, 0.5, 0.5) * DIR_ATTS[dir as usize] * DIR_ATTS[0];
    // Legs
    const LEG_LEN: f32 = 0.3;
    const LEG_DIA: f32 = 0.05;
    const LEG_POS: f32 = RADIUS * 0.6;
    let greg_wire = lk.wire_sprite.get().unwrap();
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
    let spr = lk.tiers.borrow()[emitter.tier as usize].emitter_sprite.uref().sub(0.4, 0.2, 0.6, 0.4);
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
fn new_tile(jni: &'static JNI, _this: usize, pos: usize, state: usize) -> usize {
    EmitterSupplier.new_tile(&objs().mtx.lock(jni).unwrap(), BorrowedRef::new(jni, &pos), BorrowedRef::new(jni, &state)).into_raw()
}

#[dyn_abi]
fn on_place(jni: &JNI, this: usize, _state: usize, level: usize, pos: usize, _old_state: usize, _moved_by_piston: bool) {
    BorrowedRef::new(jni, &level).call_void_method(objs().mv.level_update_neighbors_for_out_signal, &[pos, this]).unwrap()
}

//////////////////////////////////////
// Energy Container Implementations //
//////////////////////////////////////

fn energy_container_tile<'a>(lk: &GlobalMtx, this: BorrowedRef<'a, '_>) -> LocalRef<'a> {
    lk.emitter_blocks.get().unwrap().energy_container.read(lk, this).tile.get().unwrap().with_jni(&this.jni).new_local_ref().unwrap()
}

#[dyn_abi]
fn get_eu_stored(jni: &JNI, this: usize) -> i64 {
    let lk = objs().mtx.lock(jni).unwrap();
    let result = lk.read_tile::<Emitter>(energy_container_tile(&lk, BorrowedRef::new(jni, &this)).borrow()).server.borrow().energy;
    result
}

#[dyn_abi]
fn get_eu_capacity(jni: &JNI, this: usize) -> i64 {
    let lk = objs().mtx.lock(jni).unwrap();
    let result = lk.read_tile::<Emitter>(energy_container_tile(&lk, BorrowedRef::new(jni, &this)).borrow()).eu_capacity(&*lk.tiers.borrow());
    result
}

#[dyn_abi]
fn get_input_volts(jni: &JNI, this: usize) -> i64 {
    let lk = objs().mtx.lock(jni).unwrap();
    let result = lk.read_tile::<Emitter>(energy_container_tile(&lk, BorrowedRef::new(jni, &this)).borrow()).volts(&*lk.tiers.borrow());
    result
}

#[dyn_abi]
fn can_input_eu_from_side(jni: &JNI, this: usize, in_side: usize) -> bool {
    let lk = objs().mtx.lock(jni).unwrap();
    let tile = energy_container_tile(&lk, BorrowedRef::new(jni, &this));
    let emitter = lk.read_tile::<Emitter>(tile.borrow());
    let result = Some(BorrowedRef::new(jni, &in_side).read_dir() ^ 1) == emitter.common.borrow().dir;
    result
}

#[dyn_abi]
fn accept_eu(jni: &JNI, this: usize, in_side: usize, volts: i64, amps: i64) -> i64 {
    if amps < 1 {
        return 0;
    }
    let GlobalObjs { mv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let tile = energy_container_tile(&lk, BorrowedRef::new(jni, &this));
    let emitter = lk.read_tile::<Emitter>(tile.borrow());
    if in_side != 0 && Some(BorrowedRef::new(jni, &in_side).read_dir() ^ 1) != emitter.common.borrow().dir {
        return 0;
    }
    let tiers = lk.tiers.borrow();
    if volts > emitter.volts(&tiers) {
        let state = mv.blocks_fire.with_jni(jni).call_object_method(mv.block_default_state, &[]).unwrap().unwrap();
        tile.tile_level().call_bool_method(mv.level_set_block_and_update, &[tile.tile_pos().raw, state.raw]).unwrap();
        return 1;
    }
    let mut data = emitter.server.borrow_mut();
    if data.energy + volts > emitter.eu_capacity(&tiers) {
        return 0;
    }
    data.energy += volts;
    1
}

#[dyn_abi]
fn change_eu(jni: &JNI, this: usize, delta: i64) -> i64 {
    let lk = objs().mtx.lock(jni).unwrap();
    let tile = energy_container_tile(&lk, BorrowedRef::new(jni, &this));
    let emitter = lk.read_tile::<Emitter>(tile.borrow());
    let mut data = emitter.server.borrow_mut();
    let old = data.energy;
    data.energy = (old + delta).clamp(0, emitter.eu_capacity(&lk.tiers.borrow()));
    data.energy - old
}
