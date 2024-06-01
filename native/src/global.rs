use crate::{
    asm::*,
    client_utils::Sprites,
    emitter_blocks::EmitterBlocks,
    emitter_items::EmitterItems,
    jvm::*,
    mapping::{Forge, GregCN, GregMN, GregMV, CN, MN, MV},
    mapping_base::*,
    objs,
    registry::MOD_ID,
    ti,
    tile_utils::TileUtils,
};
use alloc::{boxed::Box, format, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};
use hashbrown::HashMap;
use macros::dyn_abi;

pub struct GlobalObjs {
    pub av: AV<'static>,
    pub fg: Forge,
    pub cn: CN<Arc<CSig>>,
    pub mn: MN<MSig>,
    pub gcn: GregCN<Arc<CSig>>,
    pub gmn: GregMN,
    pub mv: MV,
    pub namer: ClassNamer,
    pub writer_cls: GlobalRef<'static>,
    pub tile_utils: TileUtils,
    pub greg_reg_item_stub: MSig,
    pub greg_creative_tab_stub: MSig,
    pub mtx: JMutex<'static, GlobalMtx>,
}

pub struct Tier {
    pub volt: i64,
    pub name: Box<[u8]>,
    pub has_emitter: bool,
    pub emitter_block: Option<GlobalRef<'static>>,
}

#[derive(Default)]
pub struct GlobalMtx {
    pub gmv: Option<GregMV>,
    pub sprites: Option<Sprites>,
    pub emitter_items: Option<EmitterItems>,
    pub emitter_blocks: Option<EmitterBlocks>,
    pub tier_lookup: HashMap<Box<[u8]>, u8>,
    pub tiers: Vec<Tier>,
}

impl GlobalObjs {
    pub fn new(cls: LocalRef<'static>) -> Self {
        let jv = JV::new(cls.jni).unwrap();
        let ldr = ti().with_jni(cls.jni).get_class_loader(cls.raw).unwrap().unwrap();
        let av = AV::new(ldr.new_global_ref().unwrap(), jv).unwrap();
        let namer = ClassNamer { next: 0.into() };
        let cn = CN::new();
        let fg = Forge::new(&av, &cn);
        let mut mn = MN::new(&cn);
        if !fg.fml_naming_is_srg {
            fg.map_mn(&av, &mut mn)
        }
        let mv = MV::new(&av, &cn, &mn, fg.client.is_some());
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
        let methods = [
            greg_reg_item_stub.new_method_node(&av, ldr.jni, ACC_PUBLIC | ACC_STATIC | ACC_NATIVE).unwrap(),
            greg_creative_tab_stub.new_method_node(&av, ldr.jni, ACC_PUBLIC | ACC_STATIC | ACC_NATIVE).unwrap(),
        ];
        let natives = [greg_reg_item_stub.native(greg_reg_item_stub_dyn()), greg_creative_tab_stub.native(greg_creative_tab_stub_dyn())];
        cls.class_methods(&av).unwrap().collection_extend(&av.jv, methods).unwrap();
        cls = ldr.define_class(&name.slash, &*cls.write_class_simple(&av).unwrap().byte_elems().unwrap()).unwrap();
        cls.register_natives(&natives).unwrap();

        Self {
            mtx: JMutex::new(av.jv.object.alloc_object().unwrap().new_global_ref().unwrap(), GlobalMtx::default()),
            tile_utils: TileUtils::new(&av, &cn, &mn, &namer),
            gmn: GregMN::new(&gcn),
            cn,
            namer,
            gcn,
            av,
            fg,
            mn,
            mv,
            writer_cls,
            greg_reg_item_stub,
            greg_creative_tab_stub,
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
    let name: Box<[u8]> = name[..name.len() - suffix.len()].into();
    let mut lk = objs().mtx.lock(jni).unwrap();
    if lk.gmv.is_none() {
        let gmv = GregMV::new(jni);
        let tier_volts = gmv.tier_volts.long_elems().unwrap();
        lk.tiers.reserve_exact(tier_volts.len());
        for (tier, &volt) in tier_volts.iter().enumerate() {
            let name: Box<[u8]> = gmv.tier_names.get_object_elem(tier as _).unwrap().unwrap().utf_chars().unwrap().to_ascii_lowercase().into();
            lk.tier_lookup.insert(name.clone(), tier as _);
            lk.tiers.push(Tier { volt, name, has_emitter: false, emitter_block: None })
        }
        drop(tier_volts);
        lk.gmv = Some(gmv)
    }
    let &tier = lk.tier_lookup.get(&name).unwrap();
    lk.tiers[tier as usize].has_emitter = true;
    lk.emitter_items.get_or_insert_with(|| EmitterItems::new(jni)).make_item_maker(jni, tier).into_raw()
}

#[dyn_abi]
fn greg_creative_tab_stub(jni: &'static JNI, _: usize, item: usize) -> bool {
    let GlobalObjs { mv, mtx, .. } = objs();
    let item = BorrowedRef::new(jni, &item);
    item.is_instance_of(mv.block_item.raw) && !item.is_instance_of(mtx.lock(jni).unwrap().emitter_items.uref().item.raw)
}
