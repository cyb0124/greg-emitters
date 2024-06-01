use crate::{
    asm::*,
    global::{GlobalMut, GlobalObjs},
    jvm::*,
    mapping::Forge,
    mapping_base::*,
    objs,
};
use alloc::vec::Vec;
use macros::dyn_abi;

pub const MOD_ID: &str = "greg_emitters";

pub fn init(lk: &GlobalMut) {
    let fg = &lk.m.uref().fg;
    add_forge_listener(fg, &fg.mod_evt_bus, &fg.reg_evt_sig, on_forge_reg_dyn());
}

pub fn add_forge_listener<'a>(fg: &Forge, bus: &impl JRef<'a>, evt_sig: &CSig, func: usize) {
    let GlobalObjs { av, namer, .. } = objs();
    let name = namer.next();
    let mut cls = av.new_class_node(bus.jni(), &name.slash, c"java/lang/Object").unwrap();
    cls.add_interfaces(&av, [c"java/util/function/Consumer"]).unwrap();
    let gsig = Vec::from_iter([b"Ljava/lang/Object;Ljava/util/function/Consumer<", evt_sig.sig.to_bytes(), b">;"].into_iter().flatten().copied());
    cls.class_set_gsig(&av, &cs(gsig)).unwrap();
    let accept = MSig { owner: name.clone(), name: cs("accept"), sig: cs("(Ljava/lang/Object;)V") };
    cls.class_methods(&av).unwrap().collection_add(&av.jv, accept.new_method_node(&av, cls.jni, ACC_PUBLIC | ACC_NATIVE).unwrap().raw).unwrap();
    cls = av.ldr.with_jni(cls.jni).define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
    cls.register_natives(&[accept.native(func)]).unwrap();
    bus.call_void_method(fg.evt_bus_add_listener, &[cls.alloc_object().unwrap().raw]).unwrap()
}

#[dyn_abi]
fn on_forge_reg(jni: &'static JNI, _: usize, evt: usize) {
    panic!("on_forge_reg");
}
