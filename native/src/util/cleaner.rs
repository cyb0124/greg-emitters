use super::{BaseExt, ClassBuilder, ClassNamer, FatClass};
use crate::{asm::*, jvm::*, mapping_base::*, objs};
use alloc::sync::Arc;
use core::mem::transmute;
use macros::dyn_abi;

pub struct Cleaner {
    cleaner: GlobalRef<'static>,
    reg: usize,
    task: FatClass,
}

pub trait Cleanable {
    fn free(self: Arc<Self>, jni: &JNI);
}

impl Cleaner {
    pub fn new(av: &AV<'static>, namer: &ClassNamer) -> Self {
        let sig = CSig::new(b"com.sun.jna.internal.Cleaner");
        let cls = av.ldr.load_class(&av.jv, &sig.dot).unwrap();
        let cleaner = cls.static_field_1(c"INSTANCE", &sig.sig);
        let reg = cls.get_method_id(c"register", c"(Ljava/lang/Object;Ljava/lang/Runnable;)Lcom/sun/jna/internal/Cleaner$Cleanable;").unwrap();
        let task = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([c"java/lang/Runnable"])
            .native_1(c"run", c"()V", task_run_dyn())
            .define_fat();
        Self { cleaner, reg, task }
    }

    pub fn reg<'a>(&self, obj: &impl JRef<'a>, share: Arc<dyn Cleanable>) {
        let task = self.task.new_obj(obj.jni(), unsafe { transmute(share) });
        self.cleaner.with_jni(task.jni).call_object_method(self.reg, &[obj.raw(), task.raw]).unwrap();
    }
}

#[dyn_abi]
fn task_run(jni: &JNI, task: usize) {
    let share: Arc<dyn Cleanable> = unsafe { transmute(objs().cleaner.task.read(&BorrowedRef::new(jni, &task))) };
    share.free(jni)
}
