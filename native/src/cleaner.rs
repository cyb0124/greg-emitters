use crate::{asm::*, global::ClassNamer, jvm::*, mapping_base::*, objs};
use alloc::sync::Arc;
use core::mem::transmute;
use macros::dyn_abi;

pub struct Cleaner {
    inst: GlobalRef<'static>,
    task: GlobalRef<'static>,
    reg: usize,
    p: usize,
    q: usize,
}

pub trait Cleanable {
    fn free(self: Arc<Self>, jni: &JNI);
}

impl Cleaner {
    pub fn new(av: &AV<'static>, namer: &ClassNamer) -> Self {
        let jni = av.ldr.jni;
        let sig = CSig::new(b"com.sun.jna.internal.Cleaner");
        let cls = av.ldr.load_class(&av.jv, &sig.dot).unwrap();
        let inst = cls.get_static_field_id(c"INSTANCE", &sig.sig).unwrap();
        let inst = cls.get_static_object_field(inst).unwrap().new_global_ref().unwrap();
        let reg = cls.get_method_id(c"register", c"(Ljava/lang/Object;Ljava/lang/Runnable;)Lcom/sun/jna/internal/Cleaner$Cleanable;").unwrap();
        let name = namer.next();
        let task = av.new_class_node(jni, &name.slash, c"java/lang/Object").unwrap();
        task.add_interfaces(av, [c"java/lang/Runnable"]).unwrap();
        let p = av.new_field_node(jni, c"0", c"J", 0, 0).unwrap();
        let q = av.new_field_node(jni, c"1", c"J", 0, 0).unwrap();
        task.class_fields(av).unwrap().collection_extend(&av.jv, [p, q]).unwrap();
        let run = MSig { owner: name.clone(), name: cs("run"), sig: cs("()V") };
        task.class_methods(av).unwrap().collection_add(&av.jv, run.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap().raw).unwrap();
        let task = av.ldr.define_class(&name.slash, &*task.write_class_simple(&av).unwrap().byte_elems().unwrap()).unwrap().new_global_ref().unwrap();
        task.register_natives(&[run.native(task_run_dyn())]).unwrap();
        Self { p: task.get_field_id(c"0", c"J").unwrap(), q: task.get_field_id(c"1", c"J").unwrap(), inst, task, reg }
    }

    pub fn reg<'a>(&self, obj: &impl JRef<'a>, share: Arc<dyn Cleanable>) {
        let task = self.task.with_jni(obj.jni()).alloc_object().unwrap();
        self.inst.with_jni(task.jni).call_object_method(self.reg, &[obj.raw(), task.raw]).unwrap();
        let [p, q] = unsafe { transmute(share) };
        task.set_long_field(self.p, p);
        task.set_long_field(self.q, q);
    }
}

#[dyn_abi]
fn task_run(jni: &JNI, task: usize) {
    let this = &objs().cleaner;
    let task = BorrowedRef::new(jni, &task);
    let p = task.get_long_field(this.p);
    let q = task.get_long_field(this.q);
    let share: Arc<dyn Cleanable> = unsafe { transmute([p, q]) };
    share.free(jni)
}
