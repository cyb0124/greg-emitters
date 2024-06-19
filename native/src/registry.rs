use crate::util::ClassBuilder;
use crate::util::{client::Sprite, mapping::GregMV};
use crate::{asm::*, emitter_blocks::EmitterBlocks, global::GlobalObjs, jvm::*, mapping_base::*, objs, ti};
use alloc::{format, vec::Vec};
use core::ffi::{c_char, CStr};
use macros::dyn_abi;

pub const MOD_ID: &str = "greg_emitters";
pub const EMITTER_ID: &str = "emitter";

pub fn init() {
    ti().add_capabilities(CAN_RETRANSFORM_CLASSES).unwrap();
    ti().set_event_callbacks(&[0, 0, 0, 0, class_file_load_hook_dyn()]).unwrap();
    ti().set_event_notification_mode(true, JVMTI_EVENT_CLASS_FILE_LOAD_HOOK, 0).unwrap();
    let GlobalObjs { fcn, fmv, .. } = objs();
    add_forge_listener(&fmv.mod_evt_bus, fcn.reg_evt.sig.to_bytes(), on_forge_reg_dyn());
    if fmv.client.is_some() {
        add_forge_listener(&fmv.mod_evt_bus, fcn.atlas_evt.sig.to_bytes(), on_forge_atlas_dyn());
        add_forge_listener(&fmv.mod_evt_bus, fcn.renderers_evt.sig.to_bytes(), on_forge_renderers_dyn())
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
    }
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
    let GlobalObjs { av, gcn, writer_cls, .. } = objs();
    let cls = if slash == &*gcn.reg.slash {
        patch_greg_reg(jni, data)
    } else if slash == &*gcn.creative_tab_items_gen.slash {
        patch_greg_creative_tab_items_gen(jni, data)
    } else if slash == &*gcn.material_block_renderer.slash {
        patch_greg_material_block_renderer(jni, data)
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
