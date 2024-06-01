use crate::{
    asm::*,
    client_utils::{read_pose, DrawContext},
    geometry::new_voxel_shape,
    global::{GlobalObjs, Tier},
    jvm::*,
    mapping_base::{Functor, MBOptExt},
    objs,
    registry::{forge_reg, EMITTER_ID},
};
use alloc::{format, vec};
use bstr::BStr;
use macros::dyn_abi;
use nalgebra::{point, Translation3};

pub struct EmitterBlocks {
    tile_cls: GlobalRef<'static>,
    pub tile_type: GlobalRef<'static>,
    pub renderer_provider: Option<GlobalRef<'static>>,
}

impl EmitterBlocks {
    pub fn init(jni: &'static JNI, tiers: &mut [Tier], reg_evt: &impl JRef<'static>) -> Self {
        // Tile
        let GlobalObjs { av, cn, mn, mv, namer, tile_utils, .. } = objs();
        let mut name = namer.next();
        let mut cls = av.new_class_node(jni, &name.slash, &cn.tile.slash).unwrap();
        cls = av.ldr.with_jni(jni).define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
        let tile_cls = cls.new_global_ref().unwrap();

        // Block
        name = namer.next();
        cls = av.new_class_node(jni, &name.slash, &cn.base_tile_block.slash).unwrap();
        let methods = vec![
            mn.tile_block_new_tile.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.block_beh_get_render_shape.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
            mn.block_beh_get_shape.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        ];
        let natives = [
            mn.tile_block_new_tile.native(new_tile_dyn()),
            mn.block_beh_get_render_shape.native(get_render_shape_dyn()),
            mn.block_beh_get_shape.native(get_shape_dyn()),
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

        // Others
        let tile_type = tile_utils.define_tile_type(&blocks, |jni, pos, state| new_tile(jni, 0, pos, state));
        Self { tile_cls, tile_type, renderer_provider }
    }
}

#[dyn_abi]
fn get_shape(jni: &JNI, _: usize, state: usize, level: usize, pos: usize, _: usize) -> usize {
    new_voxel_shape(jni, point![0.2, 0.2, 0.2], point![0.8, 0.8, 0.8]).into_raw()
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
fn new_tile(jni: &'static JNI, _this: usize, pos: usize, state: usize) -> usize {
    let GlobalObjs { mv, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.uref();
    let tile = defs.tile_cls.with_jni(jni).new_object(mv.tile_init, &[defs.tile_type.raw, pos, state]).unwrap();
    tile.into_raw()
}
