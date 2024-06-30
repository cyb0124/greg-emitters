use crate::util::client::ClientExt;
use crate::util::geometry::GeomExt;
use crate::util::ClassBuilder;
use crate::util::{client::Sprite, mapping::GregMV};
use crate::{asm::*, emitter_blocks::EmitterBlocks, global::GlobalObjs, jvm::*, mapping_base::*, objs, ti};
use alloc::boxed::Box;
use alloc::{format, vec::Vec};
use core::ffi::{c_char, CStr};
use macros::dyn_abi;
use nalgebra::Point2;

pub const MOD_ID: &str = "greg_emitters";
pub const PROTOCOL_VERSION: &CStr = c"0";
pub const EMITTER_ID: &str = "emitter";

pub fn init() {
    let GlobalObjs { av, mv, fcn, fmv, gcn, .. } = objs();
    ti().add_capabilities(CAN_RETRANSFORM_CLASSES | CAN_RETRANSFORM_ANY_CLASSES).unwrap();
    ti().set_event_callbacks(&[0, 0, 0, 0, class_file_load_hook_dyn()]).unwrap();
    ti().set_event_notification_mode(true, JVMTI_EVENT_CLASS_FILE_LOAD_HOOK, 0).unwrap();
    // GTRegistrate may already be loaded at this point. If so, retransform it.
    let gt_reg = av.ldr.load_class(&av.jv, &gcn.reg.dot).ok();
    let list = [gt_reg.fmap(|x| x.raw), mv.client.fmap(|x| x.mc.raw)].into_iter().flatten().chain([mv.level_chunk.raw]);
    ti().retransform_classes(&Box::from_iter(list)).unwrap();
    add_forge_listener(&fmv.mod_evt_bus, fcn.reg_evt.sig.to_bytes(), on_forge_reg_dyn());
    add_forge_listener(&fmv.com_evt_bus, fcn.chunk_watch_evt.sig.to_bytes(), on_chunk_watch_dyn());
    add_forge_listener(&fmv.com_evt_bus, fcn.chunk_unwatch_evt.sig.to_bytes(), on_chunk_unwatch_dyn());
    add_forge_listener(&fmv.com_evt_bus, fcn.chunk_load_evt.sig.to_bytes(), on_chunk_load_or_unload_dyn());
    add_forge_listener(&fmv.com_evt_bus, fcn.chunk_unload_evt.sig.to_bytes(), on_chunk_load_or_unload_dyn());
    if fmv.client.is_some() {
        add_forge_listener(&fmv.mod_evt_bus, fcn.atlas_evt.sig.to_bytes(), on_forge_atlas_dyn());
        add_forge_listener(&fmv.mod_evt_bus, fcn.renderers_evt.sig.to_bytes(), on_forge_renderers_dyn());
        add_forge_listener(&fmv.mod_evt_bus, fcn.fml_client_setup_evt.sig.to_bytes(), on_forge_client_setup_dyn());
        add_forge_listener(&fmv.com_evt_bus, fcn.render_lvl_stg_evt.sig.to_bytes(), on_level_render_dyn())
    }
}

pub fn add_forge_listener(bus: &GlobalRef<'static>, evt_sig: &[u8], func: usize) {
    let GlobalObjs { av, fmv, .. } = objs();
    let cls = ClassBuilder::new_2(av.ldr.jni, c"java/lang/Object")
        .interfaces([c"java/util/function/Consumer"])
        .gsig(&cs(Vec::from_iter([b"Ljava/lang/Object;Ljava/util/function/Consumer<", evt_sig, b">;"].into_iter().flatten().copied())))
        .native_1(c"accept", c"(Ljava/lang/Object;)V", func)
        .define_empty();
    bus.call_void_method(fmv.evt_bus_add_listener, &[cls.alloc_object().unwrap().raw]).unwrap()
}

pub fn make_resource_loc<'a>(jni: &'a JNI, ns: &CStr, id: &CStr) -> LocalRef<'a> {
    let mv = &objs().mv;
    let (ns, id) = (jni.new_utf(ns).unwrap(), jni.new_utf(id).unwrap());
    mv.resource_loc.with_jni(jni).new_object(mv.resource_loc_init, &[ns.raw, id.raw]).unwrap()
}

pub fn add_greg_dyn_resource(jni: &JNI, gmv: &GregMV, id: impl Into<Vec<u8>>, json: &str) {
    let data = gmv.dyn_resource_pack_data.with_jni(jni);
    let key = make_resource_loc(jni, &cs(MOD_ID), &cs(id));
    let ba = jni.new_byte_array(json.len() as _).unwrap();
    ba.write_byte_array(json.as_bytes(), 0).unwrap();
    data.map_put(&objs().av.jv, key.raw, ba.raw).unwrap();
}

pub fn forge_reg<'a>(evt: &impl JRef<'a>, id: &str, value: usize) {
    let fmv = &objs().fmv;
    let reg = evt.call_object_method(fmv.reg_evt_forge_reg, &[]).unwrap().unwrap();
    let key = reg.jni.new_utf(&cs(format!("{MOD_ID}:{id}"))).unwrap();
    reg.call_void_method(fmv.forge_reg_reg, &[key.raw, value]).unwrap()
}

#[dyn_abi]
fn on_forge_reg(jni: &'static JNI, _: usize, evt: usize) {
    let GlobalObjs { fmv, av, mtx, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let evt = BorrowedRef::new(jni, &evt);
    let key = evt.call_object_method(fmv.reg_evt_key, &[]).unwrap().unwrap();
    if key.equals(&av.jv, fmv.reg_key_blocks.raw).unwrap() {
        lk.emitter_blocks.get_or_init(|| EmitterBlocks::init(jni, &lk, &evt));
    } else if key.equals(&av.jv, fmv.reg_key_tile_types.raw).unwrap() {
        forge_reg(&evt, EMITTER_ID, lk.emitter_blocks.get().unwrap().tile_type.raw)
    } else if key.equals(&av.jv, fmv.reg_key_menu_types.raw).unwrap() {
        forge_reg(&evt, EMITTER_ID, lk.emitter_blocks.get().unwrap().menu_type.raw)
    }
}

fn read_chunk_watch_base<'a>(evt: &impl JRef<'a>) -> (LocalRef<'a>, Point2<i32>) {
    let fmv = &objs().fmv;
    let pos = evt.get_object_field(fmv.chunk_watch_pos).unwrap().read_chunk_pos();
    let player = evt.get_object_field(fmv.chunk_watch_player).unwrap();
    (player, pos)
}

#[dyn_abi]
fn on_chunk_watch(jni: &'static JNI, _: usize, evt: usize) {
    let evt = BorrowedRef::new(jni, &evt);
    let (player, pos) = read_chunk_watch_base(&evt);
    let level = evt.get_object_field(objs().fmv.chunk_watch_level).unwrap();
    crate::beams::on_chunk_watch(&player, &level, pos)
}

#[dyn_abi]
fn on_chunk_unwatch(jni: &JNI, _: usize, evt: usize) {
    let (player, pos) = read_chunk_watch_base(&BorrowedRef::new(jni, &evt));
    crate::beams::on_chunk_unwatch(&player, pos)
}

#[dyn_abi]
fn on_chunk_load_or_unload(jni: &'static JNI, _: usize, evt: usize) {
    let GlobalObjs { mv, fmv, .. } = objs();
    let evt = BorrowedRef::new(jni, &evt);
    let chunk = evt.get_object_field(fmv.chunk_evt_chunk).unwrap();
    let pos = chunk.get_object_field(mv.chunk_access_pos).unwrap().read_chunk_pos();
    let level = evt.get_object_field(fmv.level_evt_level).unwrap();
    crate::beams::on_chunk_load_or_unload(&level, pos)
}

#[dyn_abi]
fn on_level_render(jni: &JNI, _: usize, evt: usize) {
    let fmvc = objs().fmv.client.uref();
    let mvc = objs().mv.client.uref();
    let evt = BorrowedRef::new(jni, &evt);
    let stg = evt.get_object_field(fmvc.render_lvl_stg_evt_stage).unwrap();
    let true = stg.is_same_object(fmvc.render_lvl_stg_after_tiles.raw) else { return };
    let pose = evt.get_object_field(fmvc.render_lvl_stg_evt_pose).unwrap().last_pose();
    let camera = evt.get_object_field(fmvc.render_lvl_stg_evt_camera).unwrap();
    let camera_pos = camera.get_object_field(mvc.camera_pos).unwrap().read_vec3d().cast();
    let tick = evt.get_int_field(fmvc.render_lvl_stg_evt_tick);
    let sub_tick = evt.get_float_field(fmvc.render_lvl_stg_evt_sub_tick);
    let renderer = evt.get_object_field(fmvc.render_lvl_stg_evt_renderer).unwrap();
    let buffers = renderer.get_object_field(mvc.level_renderer_buffers).unwrap();
    let source = buffers.get_object_field(mvc.render_buffers_buffer_source).unwrap();
    let vb = source.call_object_method(mvc.multi_buffer_source_get_buffer, &[mvc.render_type_lightning.raw]).unwrap().unwrap();
    let lk = objs().mtx.lock(jni).unwrap();
    let tiers = lk.tiers.borrow();
    for beam in objs().mtx.lock(jni).unwrap().client_state.borrow().beams.values() {
        beam.render(&*tiers, &vb, &pose, camera_pos, tick, sub_tick)
    }
    source.call_void_method(mvc.buffer_source_end_batch, &[mvc.render_type_lightning.raw]).unwrap()
}

#[dyn_abi]
fn on_forge_renderers(jni: &JNI, _: usize, evt: usize) {
    let evt = BorrowedRef::new(jni, &evt);
    let fmvc = objs().fmv.client.uref();
    let provider = objs().client_defs.uref().tile_renderer.raw;
    let lk = objs().mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.get().unwrap();
    evt.call_void_method(fmvc.renderers_evt_reg, &[defs.tile_type.raw, provider]).unwrap()
}

#[dyn_abi]
fn on_forge_atlas(jni: &'static JNI, _: usize, evt: usize) {
    let evt = BorrowedRef::new(jni, &evt);
    let GlobalObjs { av, cn, mn, mv, fmv, mtx, .. } = objs();
    let mvc = mv.client.uref();
    let atlas = evt.call_object_method(fmv.client.uref().atlas_evt_get_atlas, &[]).unwrap().unwrap();
    let loc = atlas.call_object_method(mvc.atlas_loc, &[]).unwrap().unwrap();
    if loc.equals(&av.jv, mvc.atlas_loc_blocks.raw).unwrap() {
        let lk = mtx.lock(jni).unwrap();
        lk.sheets_solid.get_or_init(|| {
            let sheets = av.ldr.with_jni(atlas.jni()).load_class(&av.jv, &cn.sheets.dot).unwrap();
            let sheets_solid = mn.sheets_solid.get_static_method_id(&sheets).unwrap();
            sheets.call_static_object_method(sheets_solid, &[]).unwrap().unwrap().new_global_ref().unwrap()
        });
        lk.wire_sprite.set(Some(Sprite::new(&atlas, c"gtceu", c"block/cable/wire")));
        for tier in &mut *lk.tiers.borrow_mut() {
            tier.emitter_sprite = Some(Sprite::new(&atlas, c"gtceu", &cs(format!("item/{}_emitter", tier.name))))
        }
    }
}

#[dyn_abi]
fn on_forge_client_setup(jni: &'static JNI, _: usize, evt: usize) {
    let task = ClassBuilder::new_2(jni, c"java/lang/Object")
        .interfaces([c"java/lang/Runnable"])
        .native_1(c"run", c"()V", client_setup_task_run_dyn())
        .define_empty()
        .alloc_object()
        .unwrap();
    BorrowedRef::new(jni, &evt).call_object_method(objs().fmv.parallel_dispatch_evt_enqueue, &[task.raw]).unwrap();
}

#[dyn_abi]
fn client_setup_task_run(jni: &JNI, _: usize) {
    let mvc = objs().mv.client.uref();
    let client = objs().client_defs.uref();
    let lk = objs().mtx.lock(jni).unwrap();
    let defs = lk.emitter_blocks.get().unwrap();
    mvc.menu_screens.with_jni(jni).call_static_void_method(mvc.menu_screens_reg, &[defs.menu_type.raw, client.screen_constructor.raw]).unwrap();
}

fn patch_greg_reg<'a>(jni: &'a JNI, data: &[u8]) -> LocalRef<'a> {
    let GlobalObjs { av, gmn, greg_reg_item_stub, .. } = objs();
    let cls = av.read_class(jni, data).unwrap();
    let mut found = false;
    for method in cls.class_methods_iter(av).unwrap() {
        let method = method.unwrap().expect_some().unwrap();
        if gmn.reg_item.matches_node(av, &method).unwrap() {
            let skip = av.new_label(jni).unwrap();
            let stub = [
                av.new_var_insn(jni, OP_ALOAD, 1).unwrap(),
                greg_reg_item_stub.new_method_insn(av, jni, OP_INVOKESTATIC).unwrap(),
                av.new_insn(jni, OP_DUP).unwrap(),
                av.new_jump_insn(OP_IFNULL, &skip).unwrap(),
                av.new_insn(jni, OP_DUP).unwrap(),
                av.new_var_insn(jni, OP_ASTORE, 2).unwrap(),
                skip,
                av.new_insn(jni, OP_POP).unwrap(),
            ];
            method.method_insns(av).unwrap().prepend_insns(av, stub).unwrap();
            found = true;
            break;
        }
    }
    assert!(found);
    cls
}

fn patch_greg_creative_tab_items_gen<'a>(jni: &'a JNI, data: &[u8]) -> LocalRef<'a> {
    let GlobalObjs { av, cn, mn, greg_creative_tab_stub, .. } = objs();
    let cls = av.read_class(jni, data).unwrap();
    let mut found = false;
    for method in cls.class_methods_iter(av).unwrap() {
        let method = method.unwrap().expect_some().unwrap();
        if mn.creative_tab_items_gen_accept.matches_node(av, &method).unwrap() {
            let insns = method.method_insns(av).unwrap();
            let mut node = insns.insn_list_first(av).unwrap();
            while let Some(insn) = node {
                node = insn.insn_next(av).unwrap();
                if insn.insn_opcode(av) == OP_INSTANCEOF && &*insn.insn_t_slash(av).unwrap().utf_chars().unwrap() == cn.block_item.slash.as_bytes() {
                    insns.insert_insns_before(av, insn.raw, [av.new_insn(jni, OP_DUP).unwrap()]).unwrap();
                    let not_block_item = av.new_label(jni).unwrap();
                    let end = av.new_label(jni).unwrap();
                    let stub = [
                        /* [Item] [IsBlockItem] */ av.new_jump_insn(OP_IFEQ, &not_block_item).unwrap(),
                        /* [Item] */ greg_creative_tab_stub.new_method_insn(av, jni, OP_INVOKESTATIC).unwrap(),
                        /* [IsNotEmitter] */ av.new_jump_insn(OP_GOTO, &end).unwrap(),
                        not_block_item,
                        /* [Item] */ av.new_insn(jni, OP_POP).unwrap(),
                        /* (empty) */ av.new_ldc_insn(jni, av.jv.wrap_bool(jni, false).unwrap().raw).unwrap(),
                        /* [IsBlockItem && IsNotEmitter] */ end,
                    ];
                    insns.insert_insns_after(av, insn.raw, stub).unwrap()
                }
            }
            found = true;
            break;
        }
    }
    assert!(found);
    cls
}

fn patch_greg_material_block_renderer<'a>(jni: &'a JNI, data: &[u8]) -> LocalRef<'a> {
    let GlobalObjs { av, greg_reinit_models_stub, .. } = objs();
    let cls = av.read_class(jni, data).unwrap();
    let mut found = false;
    for method in cls.class_methods_iter(av).unwrap() {
        let method = method.unwrap().expect_some().unwrap();
        if &*method.method_name(av).unwrap().utf_chars().unwrap() == b"reinitModels" {
            method.method_insns(av).unwrap().prepend_insns(av, [greg_reinit_models_stub.new_method_insn(av, jni, OP_INVOKESTATIC).unwrap()]).unwrap();
            found = true;
            break;
        }
    }
    assert!(found);
    cls
}

fn patch_mc_level<'a>(jni: &'a JNI, data: &[u8]) -> LocalRef<'a> {
    let GlobalObjs { av, mn, mc_clear_level_stub, .. } = objs();
    let cls = av.read_class(jni, data).unwrap();
    let mut found = false;
    for method in cls.class_methods_iter(av).unwrap() {
        let method = method.unwrap().expect_some().unwrap();
        if mn.mc_clear_level.matches_node(av, &method).unwrap() {
            let insns = method.method_insns(av).unwrap();
            let stub = || [mc_clear_level_stub.new_method_insn(av, jni, OP_INVOKESTATIC).unwrap()];
            insns.for_each_return_insn(av, |x| insns.insert_insns_before(av, x.raw, stub())).unwrap();
            found = true;
            break;
        }
    }
    assert!(found);
    cls
}

fn patch_level_chunk<'a>(jni: &'a JNI, data: &[u8]) -> LocalRef<'a> {
    let GlobalObjs { av, mn, level_chunk_set_block_state_stub, .. } = objs();
    let cls = av.read_class(jni, data).unwrap();
    let mut found = false;
    for method in cls.class_methods_iter(av).unwrap() {
        let method = method.unwrap().expect_some().unwrap();
        if mn.level_chunk_set_block_state.matches_node(av, &method).unwrap() {
            let insns = method.method_insns(av).unwrap();
            let stub = [
                av.new_var_insn(jni, OP_ALOAD, 0).unwrap(),
                mn.level_chunk_level.new_field_insn(av, jni, OP_GETFIELD).unwrap(),
                av.new_var_insn(jni, OP_ALOAD, 1).unwrap(),
                level_chunk_set_block_state_stub.new_method_insn(av, jni, OP_INVOKESTATIC).unwrap(),
            ];
            insns.prepend_insns(av, stub).unwrap();
            found = true;
            break;
        }
    }
    assert!(found);
    cls
}

#[dyn_abi]
fn class_file_load_hook(
    ti: &JVMTI,
    jni: &JNI,
    _class: usize,
    _loader: usize,
    slash: *const c_char,
    _protection_domain: usize,
    len: i32,
    data: *const u8,
    new_len: *mut i32,
    new_data: *mut *mut u8,
) {
    let false = slash.is_null() else { return };
    let slash = unsafe { CStr::from_ptr(slash) };
    let data = unsafe { core::slice::from_raw_parts(data, len as _) };
    let GlobalObjs { av, cn, gcn, writer_cls, .. } = objs();
    let cls = if slash == &*gcn.reg.slash {
        patch_greg_reg(jni, data)
    } else if slash == &*gcn.creative_tab_items_gen.slash {
        patch_greg_creative_tab_items_gen(jni, data)
    } else if slash == &*gcn.material_block_renderer.slash {
        patch_greg_material_block_renderer(jni, data)
    } else if slash == &*cn.mc.slash {
        patch_mc_level(jni, data)
    } else if slash == &*cn.level_chunk.slash {
        patch_level_chunk(jni, data)
    } else {
        return;
    };
    let writer = writer_cls.with_jni(jni).new_object(av.class_writer_init, &[COMPUTE_ALL as _]).unwrap();
    let data = cls.write_class(av, writer).unwrap();
    let len = data.array_len();
    let buf = ti.allocate(len as _).unwrap();
    data.read_byte_array(buf, 0, len).unwrap();
    unsafe { *new_data = buf }
    unsafe { *new_len = len }
}
