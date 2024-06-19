use crate::{
    asm::*,
    emitter_blocks::EmitterBlocks,
    emitter_items::EmitterItems,
    jvm::*,
    mapping_base::*,
    objs,
    registry::{add_greg_dyn_resource, EMITTER_ID},
    ti,
    util::{
        cleaner::Cleaner,
        client::Sprite,
        mapping::{ForgeCN, ForgeMN, ForgeMV, GregCN, GregMN, GregMV, CN, MN, MV},
        tile::TileDefs,
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
    pub tile_defs: TileDefs,
    pub greg_reg_item_stub: MSig,
    pub greg_creative_tab_stub: MSig,
    pub greg_reinit_models_stub: MSig,
    pub mtx: JMutex<'static, GlobalMtx>,
}

pub struct Tier {
    pub volt: i64,
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
        cb.define_empty();

        Self {
            mtx: JMutex::new(av.jv.object.alloc_object().unwrap().new_global_ref().unwrap(), GlobalMtx::default()),
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
        }
    }
}

#[dyn_abi]
fn greg_reg_item_stub(jni: &'static JNI, _: usize, name: usize) -> usize {
    let name = BorrowedRef::new(jni, &name);
    let name = name.utf_chars().unwrap();
    let suffix = b"_emitter";
    let true = name.ends_with(suffix) else { return 0 };
    let tier = str::from_utf8(&name[..name.len() - suffix.len()]).unwrap();
    let lk = objs().mtx.lock(jni).unwrap();
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
            tiers.push(Tier { volt, name, has_emitter: false, emitter_sprite: None, emitter_block: OnceCell::new(), emitter_item: OnceCell::new() })
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
