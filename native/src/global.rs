use crate::{
    asm::*,
    cleaner::Cleaner,
    client_utils::Sprite,
    emitter_blocks::EmitterBlocks,
    emitter_items::EmitterItems,
    jvm::*,
    mapping::{ForgeCN, ForgeMN, ForgeMV, GregCN, GregMN, GregMV, CN, MN, MV},
    mapping_base::*,
    objs,
    registry::{add_greg_dyn_resource, EMITTER_ID, MOD_ID},
    ti,
    tile_utils::TileUtils,
};
use alloc::{format, sync::Arc, vec::Vec};
use core::{
    cell::OnceCell,
    str,
    sync::atomic::{AtomicUsize, Ordering},
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
    pub tile_utils: TileUtils,
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
    pub wire_sprite: Option<Sprite>,
    pub emitter_items: OnceCell<EmitterItems>,
    pub emitter_blocks: OnceCell<EmitterBlocks>,
    pub tier_lookup: HashMap<Arc<str>, u8>,
    pub tiers: Vec<Tier>,
}

impl GlobalObjs {
    pub fn new(cls: LocalRef<'static>) -> Self {
        let jv = JV::new(cls.jni).unwrap();
        let ldr = ti().with_jni(cls.jni).get_class_loader(cls.raw).unwrap().unwrap();
        let av = AV::new(ldr.new_global_ref().unwrap(), jv).unwrap();
        let namer = ClassNamer { next: 0.into() };
        let cn = CN::new();
        let fcn = ForgeCN::new();
        let fmn = ForgeMN::new(&cn, &fcn);
        let fmv = ForgeMV::new(&av, &cn, &fcn, &fmn);
        let mn = MN::new(&av, &cn, &fmv);
        let mv = MV::new(&av, &cn, &mn, fmv.client.is_some());
        let gcn = GregCN::new();

        // Class Writer
        let mut name = namer.next();
        let mut cls = av.new_class_node(ldr.jni, &name.slash, c"org/objectweb/asm/ClassWriter").unwrap();
        cls = ldr.define_class(&name.slash, &*cls.write_class_simple(&av).unwrap().byte_elems().unwrap()).unwrap();
        let writer_cls = cls.new_global_ref().unwrap();

        // Stubs
        name = namer.next();
        cls = av.new_class_node(ldr.jni, &name.slash, c"java/lang/Object").unwrap();
        let greg_reg_item_stub =
            MSig { owner: name.clone(), name: cs("0"), sig: msig([b"Ljava/lang/String;".as_slice()], gcn.non_null_fn.sig.to_bytes()) };
        let greg_creative_tab_stub = MSig { owner: name.clone(), name: cs("0"), sig: msig([cn.item.sig.to_bytes()], b"Z") };
        let greg_reinit_models_stub = MSig { owner: name.clone(), name: cs("0"), sig: cs("()V") };
        let methods = [
            greg_reg_item_stub.new_method_node(&av, ldr.jni, ACC_PUBLIC | ACC_STATIC | ACC_NATIVE).unwrap(),
            greg_creative_tab_stub.new_method_node(&av, ldr.jni, ACC_PUBLIC | ACC_STATIC | ACC_NATIVE).unwrap(),
            greg_reinit_models_stub.new_method_node(&av, ldr.jni, ACC_PUBLIC | ACC_STATIC | ACC_NATIVE).unwrap(),
        ];
        let natives = [
            greg_reg_item_stub.native(greg_reg_item_stub_dyn()),
            greg_creative_tab_stub.native(greg_creative_tab_stub_dyn()),
            greg_reinit_models_stub.native(greg_reinit_models_stub_dyn()),
        ];
        cls.class_methods(&av).unwrap().collection_extend(&av.jv, methods).unwrap();
        cls = ldr.define_class(&name.slash, &*cls.write_class_simple(&av).unwrap().byte_elems().unwrap()).unwrap();
        cls.register_natives(&natives).unwrap();

        Self {
            mtx: JMutex::new(av.jv.object.alloc_object().unwrap().new_global_ref().unwrap(), GlobalMtx::default()),
            tile_utils: TileUtils::new(&av, &cn, &mn, &namer),
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

pub struct ClassNamer {
    next: AtomicUsize,
}

impl ClassNamer {
    pub fn next(&self) -> Arc<CSig> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        CSig::new(&format!("cyb0124/{MOD_ID}/{id}").as_bytes()).into()
    }
}

#[dyn_abi]
fn greg_reg_item_stub(jni: &'static JNI, _: usize, name: usize) -> usize {
    let name = BorrowedRef::new(jni, &name);
    let name = name.utf_chars().unwrap();
    let suffix = b"_emitter";
    let true = name.ends_with(suffix) else { return 0 };
    let tier = str::from_utf8(&name[..name.len() - suffix.len()]).unwrap();
    let mut lk = objs().mtx.lock(jni).unwrap();
    let lk = &mut *lk;
    lk.gmv.get_or_init(|| {
        let gmv = GregMV::new(jni);
        let tier_volts = gmv.tier_volts.long_elems().unwrap();
        lk.tiers.reserve_exact(tier_volts.len());
        for (tier, &volt) in tier_volts.iter().enumerate() {
            let name = gmv.tier_names.get_object_elem(tier as _).unwrap().unwrap();
            let name = Arc::<str>::from(str::from_utf8(&*name.utf_chars().unwrap()).unwrap().to_lowercase());
            lk.tier_lookup.insert(name.clone(), tier as _);
            lk.tiers.push(Tier {
                volt,
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
    let &tier = lk.tier_lookup.get(tier).unwrap();
    lk.tiers[tier as usize].has_emitter = true;
    lk.emitter_items.get_or_init(|| EmitterItems::new(jni)).make_item_maker(jni, tier).into_raw()
}

#[dyn_abi]
fn greg_creative_tab_stub(jni: &'static JNI, _: usize, item: usize) -> bool {
    // Return if item should be hidden from creative tab (conjunctive with other conditions)
    !BorrowedRef::new(jni, &item).is_instance_of(objs().mtx.lock(jni).unwrap().emitter_items.get().unwrap().item.raw)
}

#[dyn_abi]
fn greg_reinit_models_stub(jni: &JNI, _: usize) {
    let lk = objs().mtx.lock(jni).unwrap();
    let gmv = lk.gmv.get().unwrap();
    for tier in &lk.tiers {
        let true = tier.has_emitter else { continue };
        let id = format!("blockstates/{EMITTER_ID}_{}.json", tier.name);
        let json = format!("{{\"variants\":{{\"\":{{\"model\":\"gtceu:item/{}_emitter\"}}}}}}", tier.name);
        add_greg_dyn_resource(jni, gmv, id, &json)
    }
}
