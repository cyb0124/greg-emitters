use crate::{
    asm::*,
    beams::{ClientState, ServerState, TrackedBlock},
    emitter_blocks::EmitterBlocks,
    emitter_items::EmitterItems,
    jvm::*,
    mapping_base::*,
    objs,
    registry::{add_greg_dyn_resource, EMITTER_ID, MOD_ID},
    ti,
    util::{
        cleaner::Cleaner,
        client::{ClientDefs, Sprite},
        geometry::GeomExt,
        gui::GUIDefs,
        mapping::{ForgeCN, ForgeMN, ForgeMV, GregCN, GregMN, GregMV, CN, MN, MV},
        network::NetworkDefs,
        tile::{TileDefs, TileExt},
        ClassBuilder, ClassNamer,
    },
};
use alloc::{format, sync::Arc, vec::Vec};
use core::{
    cell::{Cell, OnceCell, RefCell},
    str,
};
use hashbrown::HashMap;
use macros::dyn_abi;
use nalgebra::{vector, Vector3};

pub struct GlobalObjs {
    pub av: AV<'static>,
    pub cn: CN<Arc<CSig>>,
    pub mn: MN<MSig>,
    pub mv: MV,
    pub fcn: ForgeCN<Arc<CSig>>,
    pub fmn: ForgeMN,
    pub fmv: ForgeMV,
    pub gcn: GregCN<Arc<CSig>>,
    pub gmn: GregMN,
    pub namer: ClassNamer,
    pub writer_cls: GlobalRef<'static>,
    pub cleaner: Cleaner,
    pub gui_defs: GUIDefs,
    pub tile_defs: TileDefs,
    pub client_defs: Option<ClientDefs>,
    pub greg_reg_item_stub: MSig,
    pub greg_creative_tab_stub: MSig,
    pub greg_reinit_models_stub: MSig,
    pub level_chunk_set_block_state_stub: MSig,
    pub mc_clear_level_stub: MSig,
    pub mtx: JMutex<'static, GlobalMtx>,
    pub net_defs: NetworkDefs,
    logger: GlobalRef<'static>,
    logger_warn: usize,
}

pub struct Tier {
    pub volt: i64,
    pub color: Vector3<f32>,
    pub name: Arc<str>,
    pub has_emitter: bool,
    pub emitter_sprite: Option<Sprite>,
    pub emitter_block: OnceCell<GlobalRef<'static>>,
    pub emitter_item: OnceCell<GlobalRef<'static>>,
}

#[derive(Default)]
pub struct GlobalMtx {
    pub gmv: OnceCell<GregMV>,
    pub sheets_solid: OnceCell<GlobalRef<'static>>,
    pub wire_sprite: Cell<Option<Sprite>>,
    pub emitter_items: OnceCell<EmitterItems>,
    pub emitter_blocks: OnceCell<EmitterBlocks>,
    pub tier_lookup: RefCell<HashMap<Arc<str>, u8>>,
    pub tiers: RefCell<Vec<Tier>>,
    pub server_state: RefCell<ServerState>,
    pub client_state: RefCell<ClientState>,
}

impl GlobalObjs {
    pub fn new(cls: LocalRef<'static>) -> Self {
        let jv = JV::new(cls.jni).unwrap();
        let ldr = ti().with_jni(cls.jni).get_class_loader(cls.raw).unwrap().unwrap();
        let av = AV::new(ldr.new_global_ref().unwrap(), jv).unwrap();
        let namer = ClassNamer::default();
        let cn = CN::new();
        let fcn = ForgeCN::new();
        let fmn = ForgeMN::new(&cn, &fcn);
        let fmv = ForgeMV::new(&av, &cn, &fcn, &fmn);
        let mn = MN::new(&av, &cn, &fmv);
        let mv = MV::new(&av, &cn, &mn, fmv.client.is_some());
        let gcn = GregCN::new();

        // Class Writer
        let name = namer.next();
        let mut cls = av.new_class_node(ldr.jni, &name.slash, c"org/objectweb/asm/ClassWriter").unwrap();
        cls = ldr.define_class(&name.slash, &*cls.write_class_simple(&av).unwrap().byte_elems().unwrap()).unwrap();
        let writer_cls = cls.new_global_ref().unwrap();

        // Stubs
        let mut cb = ClassBuilder::new_1(&av, &namer, c"java/lang/Object");
        let greg_reg_item_stub = cb.stub_name(cs("0"), msig([b"Ljava/lang/String;".as_slice()], gcn.non_null_fn.sig.to_bytes()));
        cb.stub(&greg_reg_item_stub, greg_reg_item_stub_dyn());
        let greg_creative_tab_stub = cb.stub_name(cs("0"), msig([cn.item.sig.to_bytes()], b"Z"));
        cb.stub(&greg_creative_tab_stub, greg_creative_tab_stub_dyn());
        let greg_reinit_models_stub = cb.stub_name(cs("0"), cs("()V"));
        cb.stub(&greg_reinit_models_stub, greg_reinit_models_stub_dyn());
        let mc_clear_level_stub = cb.stub_name(cs("1"), cs("()V"));
        cb.stub(&mc_clear_level_stub, mc_clear_level_stub_dyn());
        let level_chunk_set_block_state_stub = cb.stub_name(cs("0"), msig([cn.level.sig.to_bytes(), cn.block_pos.sig.to_bytes()], b"V"));
        cb.stub(&level_chunk_set_block_state_stub, level_chunk_set_block_state_stub_dyn());
        cb.define_empty();

        // Logger
        let logger_factory = av.ldr.load_class(&av.jv, c"org.slf4j.LoggerFactory").unwrap();
        let get_logger = logger_factory.get_static_method_id(c"getLogger", c"(Ljava/lang/String;)Lorg/slf4j/Logger;").unwrap();
        let logger = logger_factory.call_static_object_method(get_logger, &[av.ldr.jni.new_utf(&cs(MOD_ID)).unwrap().raw]).unwrap().unwrap();

        Self {
            mtx: JMutex::new(av.jv.object.alloc_object().unwrap().new_global_ref().unwrap(), GlobalMtx::default()),
            client_defs: mv.client.fmap(|_| ClientDefs::init(&av, &namer, &cn, &mn)),
            net_defs: NetworkDefs::init(&av, &namer, &mv, &fmv),
            gui_defs: GUIDefs::init(&av, &cn, &mn, &fcn, &fmn, &namer),
            tile_defs: TileDefs::init(&av, &cn, &mn, &fmn, &namer),
            cleaner: Cleaner::new(&av, &namer),
            gmn: GregMN::new(&cn, &gcn),
            namer,
            fcn,
            fmn,
            fmv,
            gcn,
            cn,
            av,
            mn,
            mv,
            writer_cls,
            greg_reg_item_stub,
            greg_creative_tab_stub,
            greg_reinit_models_stub,
            mc_clear_level_stub,
            level_chunk_set_block_state_stub,
            logger: logger.new_global_ref().unwrap(),
            logger_warn: logger.get_object_class().get_method_id(c"warn", c"(Ljava/lang/String;)V").unwrap(),
        }
    }
}

pub fn warn(jni: &JNI, text: impl Into<Vec<u8>>) {
    objs().logger.with_jni(jni).call_void_method(objs().logger_warn, &[jni.new_utf(&cs(text.into())).unwrap().raw]).unwrap()
}

#[dyn_abi]
fn greg_reg_item_stub(jni: &'static JNI, _: usize, name: usize) -> usize {
    let GlobalObjs { av, mv, mtx, .. } = objs();
    let name = BorrowedRef::new(jni, &name);
    let name = name.utf_chars().unwrap();
    let suffix = b"_emitter";
    let true = name.ends_with(suffix) else { return 0 };
    let tier = str::from_utf8(&name[..name.len() - suffix.len()]).unwrap();
    let lk = mtx.lock(jni).unwrap();
    lk.gmv.get_or_init(|| {
        let gmv = GregMV::new(jni);
        let tier_volts = gmv.tier_volts.long_elems().unwrap();
        let mut tiers = lk.tiers.borrow_mut();
        let mut lookup = lk.tier_lookup.borrow_mut();
        tiers.reserve_exact(tier_volts.len());
        for (tier, &volt) in tier_volts.iter().enumerate() {
            let name = gmv.tier_names.get_object_elem(tier as _).unwrap().unwrap();
            let name = Arc::<str>::from(str::from_utf8(&*name.utf_chars().unwrap()).unwrap().to_lowercase());
            lookup.insert(name.clone(), tier as _);
            let code = gmv.tier_fmt_names.get_object_elem(tier as _).unwrap().unwrap().chars().unwrap()[1];
            let fmt = mv.chat_fmt.with_jni(jni).call_static_object_method(mv.chat_fmt_from_code, &[code as _]).unwrap().unwrap();
            let color = fmt.get_object_field(mv.chat_fmt_color).unwrap().int_value(&av.jv).unwrap();
            tiers.push(Tier {
                volt,
                color: vector![(color >> 16) as f32 / 255., ((color >> 8) & 255) as f32 / 255., (color & 255) as f32 / 255.],
                name,
                has_emitter: false,
                emitter_sprite: None,
                emitter_block: OnceCell::new(),
                emitter_item: OnceCell::new(),
            })
        }
        drop(tier_volts);
        gmv
    });
    let &tier = lk.tier_lookup.borrow().get(tier).unwrap();
    lk.tiers.borrow_mut()[tier as usize].has_emitter = true;
    lk.emitter_items.get_or_init(|| EmitterItems::new(jni)).new_item_factory(jni, tier).into_raw()
}

#[dyn_abi]
fn greg_creative_tab_stub(jni: &'static JNI, _: usize, item: usize) -> bool {
    // Return if item should be hidden from creative tab (conjunctive with other conditions)
    !BorrowedRef::new(jni, &item).is_instance_of(objs().mtx.lock(jni).unwrap().emitter_items.get().unwrap().item.raw)
}

#[dyn_abi]
fn greg_reinit_models_stub(jni: &JNI, _: usize) {
    let lk = objs().mtx.lock(jni).unwrap();
    for tier in &*lk.tiers.borrow() {
        let true = tier.has_emitter else { continue };
        let id = format!("blockstates/{EMITTER_ID}_{}.json", tier.name);
        let json = format!("{{\"variants\":{{\"\":{{\"model\":\"gtceu:item/{}_emitter\"}}}}}}", tier.name);
        add_greg_dyn_resource(jni, lk.gmv.get().unwrap(), id, &json)
    }
}

#[dyn_abi]
fn mc_clear_level_stub(jni: &JNI, _: usize) { objs().mtx.lock(jni).unwrap().client_state.borrow_mut().beams.clear() }

#[dyn_abi]
fn level_chunk_set_block_state_stub(jni: &JNI, _: usize, level: usize, pos: usize) {
    let level = BorrowedRef::new(jni, &level);
    let false = level.level_is_client() else { return };
    let mtx = objs().mtx.lock(jni).unwrap();
    let mut srv = mtx.server_state.borrow_mut();
    let srv = &mut *srv;
    let Some(dim) = srv.dims.find(ti().id_hash(level.raw).unwrap() as _, |x| level.is_same_object(x.level.0.raw)) else { return };
    let Some(block) = dim.blocks.get(&BorrowedRef::new(jni, &pos).read_vec3i()) else { return };
    match block {
        TrackedBlock::ByOne(id) => srv.beams.get_mut(id).unwrap().dirty = true,
        TrackedBlock::ByMany(beams) => {
            for id in beams {
                srv.beams.get_mut(id).unwrap().dirty = true
            }
        }
    }
}
