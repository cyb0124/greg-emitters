use crate::{
    asm::*,
    greg::{GregCN, GregMN},
    jvm::*,
    mapping::{make_cn, Mapping, CN},
    mapping_base::*,
    registry::MOD_ID,
    ti,
    transformer::TfDefs,
};
use alloc::{format, sync::Arc};
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct GlobalObjs {
    pub av: AV<'static>,
    pub namer: ClassNamer,
    pub cn: CN<Arc<CSig>>,
    pub gcn: GregCN<Arc<CSig>>,
    pub gmn: GregMN,
    pub mtx: JMutex<'static, GlobalMut>,
}

#[derive(Default)]
pub struct GlobalMut {
    pub tf: Option<TfDefs>,
    pub m: Option<Mapping>,
}

impl GlobalObjs {
    pub fn new(tf_srv_cls: &impl JRef<'static>) -> Self {
        let jni = tf_srv_cls.jni();
        let jv = JV::new(jni).unwrap();
        let ldr = ti().with_jni(jni).get_class_loader(tf_srv_cls.raw()).unwrap().unwrap();
        let av = AV::new(ldr.new_global_ref().unwrap(), jv).unwrap();
        let namer = ClassNamer { next: 0.into() };
        let gcn = GregCN::new();
        Self {
            mtx: JMutex::new(av.jv.object.alloc_object().unwrap().new_global_ref().unwrap(), GlobalMut::default()),
            gmn: GregMN::new(&gcn),
            cn: make_cn(),
            namer,
            gcn,
            av,
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
