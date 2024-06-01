use crate::{asm::*, global::GlobalObjs, jvm::*, mapping_base::*, objs};
use alloc::sync::Arc;
use mapping_macros::Functor;

#[derive(Functor)]
pub struct CN<T> {
    pub base_tile_block: T,
    pub block_beh_props: T,
}

pub fn make_cn() -> CN<Arc<CSig>> {
    let names = CN::<&[u8]> {
        base_tile_block: b"net.minecraft.world.level.block.BaseEntityBlock",
        block_beh_props: b"net.minecraft.world.level.block.state.BlockBehaviour$Properties",
    };
    names.fmap(|x| Arc::new(CSig::new(x)))
}

pub struct Forge {
    pub cml: GlobalRef<'static>,
    pub cml_get_mapper: usize,
    pub dom_f: GlobalRef<'static>,
    pub dom_m: GlobalRef<'static>,
    pub fml_naming_is_srg: bool,
    pub mod_evt_bus: GlobalRef<'static>,
    pub evt_bus_add_listener: usize,
    pub reg_evt_sig: CSig,
}

impl Forge {
    fn new(jni: &'static JNI) -> Self {
        let av = &objs().av;
        let ldr = av.ldr.with_jni(jni);
        let cml_sig = CSig::new(b"cpw.mods.modlauncher.Launcher");
        let cml_cls = ldr.load_class(&av.jv, &cml_sig.dot).unwrap();
        let dom_sig = CSig::new(b"cpw.mods.modlauncher.api.INameMappingService$Domain");
        let dom_cls = ldr.load_class(&av.jv, &dom_sig.dot).unwrap();
        let fml_cls = ldr.load_class(&av.jv, c"net.minecraftforge.fml.loading.FMLLoader").unwrap().new_global_ref().unwrap();
        let fml_naming = fml_cls.get_static_field_id(c"naming", c"Ljava/lang/String;").unwrap();
        let fml_naming_is_srg = &*fml_cls.get_static_object_field(fml_naming).unwrap().utf_chars().unwrap() == b"srg";
        let fml_ctx_sig = CSig::new(b"net.minecraftforge.fml.javafmlmod.FMLJavaModLoadingContext");
        let fml_ctx_cls = ldr.load_class(&av.jv, &fml_ctx_sig.dot).unwrap();
        let fml_ctx = fml_ctx_cls.get_static_method_id(c"get", &msig([], fml_ctx_sig.sig.to_bytes())).unwrap();
        let fml_ctx = fml_ctx_cls.call_static_object_method(fml_ctx, &[]).unwrap().unwrap();
        let evt_bus_sig = CSig::new(b"net.minecraftforge.eventbus.api.IEventBus");
        let evt_bus = ldr.load_class(&av.jv, &evt_bus_sig.dot).unwrap();
        let mod_evt_bus = fml_ctx_cls.get_method_id(c"getModEventBus", &msig([], evt_bus_sig.sig.to_bytes())).unwrap();
        let mod_evt_bus = fml_ctx.call_object_method(mod_evt_bus, &[]).unwrap().unwrap().new_global_ref().unwrap();
        Self {
            cml: cml_cls.get_static_object_field(cml_cls.get_static_field_id(c"INSTANCE", &cml_sig.sig).unwrap()).unwrap().new_global_ref().unwrap(),
            cml_get_mapper: cml_cls.get_method_id(c"findNameMapping", c"(Ljava/lang/String;)Ljava/util/Optional;").unwrap(),
            dom_f: dom_cls.get_static_object_field(dom_cls.get_static_field_id(c"FIELD", &dom_sig.sig).unwrap()).unwrap().new_global_ref().unwrap(),
            dom_m: dom_cls.get_static_object_field(dom_cls.get_static_field_id(c"METHOD", &dom_sig.sig).unwrap()).unwrap().new_global_ref().unwrap(),
            fml_naming_is_srg,
            mod_evt_bus,
            evt_bus_add_listener: evt_bus.get_method_id(c"addListener", c"(Ljava/util/function/Consumer;)V").unwrap(),
            reg_evt_sig: CSig::new(b"net.minecraftforge.registries.RegisterEvent"),
        }
    }

    fn map_mn(&self, mn: &mut MN<MSig>) {
        let av = &objs().av;
        let from = self.cml.jni.new_utf(c"srg").unwrap();
        let mapper = self.cml.call_object_method(self.cml_get_mapper, &[from.raw]).unwrap().unwrap().opt_get(&av.jv).unwrap().unwrap();
        mn.apply(|x| {
            let name = mapper.jni.new_utf(&x.name).unwrap();
            let name = mapper.bifunc_apply(&av.jv, if x.is_method() { self.dom_m.raw } else { self.dom_f.raw }, name.raw);
            x.name = cs(name.unwrap().unwrap().utf_chars().unwrap().to_vec())
        })
    }
}

#[derive(Functor)]
pub struct MN<T> {
    pub base_tile_block_init: T,
    pub block_beh_props_of: T,
}

fn make_mn() -> MN<MSig> {
    let cn = &objs().cn;
    MN {
        base_tile_block_init: MSig { owner: cn.base_tile_block.clone(), name: cs("<init>"), sig: msig([cn.block_beh_props.sig.to_bytes()], b"V") },
        block_beh_props_of: MSig { owner: cn.block_beh_props.clone(), name: cs("m_284310_"), sig: msig([], cn.block_beh_props.sig.to_bytes()) },
    }
}

pub struct MV {
    pub base_tile_block_init: usize,
    pub block_beh_props: GlobalRef<'static>,
    pub block_beh_props_of: usize,
}

fn make_mv(jni: &'static JNI, mn: &MN<MSig>) -> MV {
    let GlobalObjs { av, cn, .. } = objs();
    let load = |csig: &Arc<CSig>| av.ldr.with_jni(jni).load_class(&av.jv, &csig.dot).unwrap().new_global_ref().unwrap();
    let base_tile_block = load(&cn.base_tile_block);
    let block_beh_props = load(&cn.block_beh_props);
    MV {
        base_tile_block_init: mn.base_tile_block_init.get_method_id(&base_tile_block).unwrap(),
        block_beh_props_of: mn.block_beh_props_of.get_static_method_id(&block_beh_props).unwrap(),
        block_beh_props,
    }
}

pub struct Mapping {
    pub fg: Forge,
    pub mn: MN<MSig>,
    pub mv: MV,
}

impl Mapping {
    pub fn new(jni: &'static JNI) -> Self {
        let fg = Forge::new(jni);
        let mut mn = make_mn();
        if !fg.fml_naming_is_srg {
            fg.map_mn(&mut mn)
        }
        Self { mv: make_mv(jni, &mn), fg, mn }
    }
}
