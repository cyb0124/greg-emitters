pub mod cleaner;
pub mod client;
pub mod geometry;
pub mod gui;
pub mod mapping;
pub mod nbt;
pub mod network;
pub mod tessellator;
pub mod tile;

use self::cleaner::Cleanable;
use crate::{
    asm::*,
    global::{GlobalMtx, GlobalObjs},
    jvm::*,
    mapping_base::{CSig, MSig},
    objs,
    registry::MOD_ID,
};
use alloc::{ffi::CString, format, sync::Arc, vec::Vec};
use anyhow::{anyhow, ensure, Result};
use core::{
    ffi::CStr,
    marker::{PhantomData, Unsize},
    mem::transmute_copy,
    sync::atomic::{AtomicUsize, Ordering},
};
use serde::de::DeserializeOwned;

impl<'a, T: JRef<'a>> UtilExt<'a> for T {}
pub trait UtilExt<'a>: JRef<'a> {
    fn static_field_2(&self, msig: &MSig) -> GlobalRef<'a> { self.static_field_1(&msig.name, &msig.sig) }
    fn static_field_1(&self, name: &CStr, sig: &CStr) -> GlobalRef<'a> {
        self.get_static_object_field(self.get_static_field_id(name, sig).unwrap()).unwrap().new_global_ref().unwrap()
    }
}

pub fn strict_deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let (result, remain) = postcard::take_from_bytes(bytes).map_err(|e| anyhow!("{e}"))?;
    ensure!(remain.is_empty());
    Ok(result)
}

#[derive(Default)]
pub struct ClassNamer {
    next: AtomicUsize,
}

impl ClassNamer {
    pub fn next(&self) -> Arc<CSig> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        CSig::new(&format!("cyb0124/{MOD_ID}/{id}").as_bytes()).into()
    }
}

pub struct ThinClass {
    pub cls: GlobalRef<'static>,
    p: usize,
}

impl ThinClass {
    pub fn wrap<T: Cleanable + 'static>(self) -> ThinWrapper<T> { ThinWrapper { cls: self, _p: PhantomData } }
    pub fn read<'a>(&self, obj: &impl JRef<'a>) -> i64 { obj.get_long_field(self.p) }
    pub fn new_obj<'a>(&self, jni: &'a JNI, p: i64) -> LocalRef<'a> {
        let obj = self.cls.with_jni(jni).alloc_object().unwrap();
        obj.set_long_field(self.p, p);
        obj
    }
}

pub struct ThinWrapper<T: Cleanable + 'static> {
    pub cls: ThinClass,
    _p: PhantomData<fn(T) -> T>,
}

impl<T: Cleanable + 'static> ThinWrapper<T> {
    // Borrow GlobalMtx to ensure lock is held.
    pub fn read<'a>(&self, _: &'a GlobalMtx, obj: BorrowedRef<'_, 'a>) -> &'a T { unsafe { &*(self.cls.read(&obj) as *const T) } }
    pub fn new_obj<'a>(&self, jni: &'a JNI, data: Arc<T>) -> LocalRef<'a> {
        let obj = self.cls.new_obj(jni, &*data as *const T as _);
        objs().cleaner.reg(&obj, data);
        obj
    }
}

pub struct FatClass {
    pub cls: GlobalRef<'static>,
    p: usize,
    q: usize,
}

impl FatClass {
    pub fn wrap<T: ?Sized + Send + 'static>(self) -> FatWrapper<T> { FatWrapper { cls: self, _p: PhantomData } }
    pub fn read<'a>(&self, obj: &impl JRef<'a>) -> [i64; 2] { [obj.get_long_field(self.p), obj.get_long_field(self.q)] }
    pub fn new_obj<'a>(&self, jni: &'a JNI, [p, q]: [i64; 2]) -> LocalRef<'a> {
        let obj = self.cls.with_jni(jni).alloc_object().unwrap();
        obj.set_long_field(self.p, p);
        obj.set_long_field(self.q, q);
        obj
    }
}

pub struct FatWrapper<T: ?Sized + Send + 'static> {
    pub cls: FatClass,
    _p: PhantomData<fn(T) -> T>,
}

impl<T: ?Sized + Send + 'static> FatWrapper<T> {
    pub fn new_static<'a>(&self, jni: &'a JNI, data: &'static T) -> LocalRef<'a> {
        let (ptr, meta) = (data as *const T).to_raw_parts();
        self.cls.new_obj(jni, [ptr as _, unsafe { transmute_copy(&meta) }])
    }

    // Borrow GlobalMtx to ensure lock is held.
    pub fn read<'a>(&self, _: &'a GlobalMtx, obj: BorrowedRef<'_, 'a>) -> &'a T {
        let [ptr, meta] = self.cls.read(&obj);
        unsafe { &*core::ptr::from_raw_parts(ptr as *const (), transmute_copy(&meta)) }
    }
}

impl<T: Unsize<dyn Cleanable> + ?Sized + Send + 'static> FatWrapper<T> {
    pub fn new_obj<'a>(&self, jni: &'a JNI, data: Arc<T>) -> LocalRef<'a> {
        let (ptr, meta) = (&*data as *const T).to_raw_parts();
        let obj = self.cls.new_obj(jni, [ptr as _, unsafe { transmute_copy(&meta) }]);
        objs().cleaner.reg(&obj, data);
        obj
    }
}

pub struct ClassBuilder<'a> {
    av: &'a AV<'static>,
    name: Arc<CSig>,
    cls: LocalRef<'static>,
    methods: LocalRef<'static>,
    natives: Vec<NativeMethod>,
}

impl<'a> ClassBuilder<'a> {
    fn new_priv(jni: &'static JNI, av: &'a AV<'static>, namer: &ClassNamer, sup_slash: &CStr) -> Self {
        let name = namer.next();
        let cls = av.new_class_node(jni, &name.slash, &sup_slash).unwrap();
        let methods = cls.class_methods(av).unwrap();
        Self { av, name, cls, methods, natives: Vec::new() }
    }

    pub fn new_1(av: &'a AV<'static>, namer: &ClassNamer, sup_slash: &CStr) -> Self { Self::new_priv(av.ldr.jni, av, namer, sup_slash) }
    pub fn new_2(jni: &'static JNI, sup_slash: &CStr) -> Self {
        let GlobalObjs { av, namer, .. } = objs();
        Self::new_priv(jni, av, namer, sup_slash)
    }

    pub fn interfaces<'b>(&mut self, slashes: impl IntoIterator<Item = &'b CStr>) -> &mut Self {
        self.cls.add_interfaces(self.av, slashes).unwrap();
        self
    }

    pub fn gsig(&mut self, gsig: &CStr) -> &mut Self {
        self.cls.class_set_gsig(self.av, gsig).unwrap();
        self
    }

    pub fn native_1(&mut self, name: &'a CStr, sig: &'a CStr, func: usize) -> &mut Self {
        self.methods.collection_add(&self.av.jv, self.av.new_method_node(self.cls.jni, name, sig, ACC_PUBLIC | ACC_NATIVE).unwrap().raw).unwrap();
        self.natives.push(NativeMethod { name: name.as_ptr(), sig: sig.as_ptr(), func });
        self
    }

    pub fn native_2(&mut self, mn: &'a MSig, func: usize) -> &mut Self {
        self.methods.collection_add(&self.av.jv, mn.new_method_node(self.av, self.cls.jni, ACC_PUBLIC | ACC_NATIVE).unwrap().raw).unwrap();
        self.natives.push(mn.native(func));
        self
    }

    pub fn insns<T: JRef<'a>>(&mut self, mn: &'a MSig, insns: impl IntoIterator<Item = T>) -> &mut Self {
        let method = mn.new_method_node(self.av, &self.cls.jni, ACC_PUBLIC).unwrap();
        method.method_insns(self.av).unwrap().append_insns(self.av, insns).unwrap();
        self.methods.collection_add(&self.av.jv, method.raw).unwrap();
        self
    }

    pub fn stub_name(&self, name: CString, sig: CString) -> MSig { MSig { owner: self.name.clone(), name, sig } }
    pub fn stub(&mut self, mn: &MSig, func: usize) -> &mut Self {
        let method = mn.new_method_node(self.av, self.cls.jni, ACC_PUBLIC | ACC_STATIC | ACC_NATIVE).unwrap();
        self.methods.collection_add(&self.av.jv, method.raw).unwrap();
        self.natives.push(mn.native(func));
        self
    }

    pub fn define_empty(&mut self) -> GlobalRef<'static> {
        let cls = self.cls.write_class_simple(self.av).unwrap();
        let cls = self.av.ldr.with_jni(self.cls.jni).define_class(&self.name.slash, &*cls.byte_elems().unwrap()).unwrap();
        cls.register_natives(&self.natives).unwrap();
        cls.new_global_ref().unwrap()
    }

    pub fn define_thin(&mut self) -> ThinClass {
        let p = self.av.new_field_node(self.cls.jni, c"0", c"J", 0, 0).unwrap();
        self.cls.class_fields(self.av).unwrap().collection_extend(&self.av.jv, [p]).unwrap();
        let cls = self.define_empty();
        ThinClass { p: cls.get_field_id(c"0", c"J").unwrap(), cls }
    }

    pub fn define_fat(&mut self) -> FatClass {
        let p = self.av.new_field_node(self.cls.jni, c"0", c"J", 0, 0).unwrap();
        let q = self.av.new_field_node(self.cls.jni, c"1", c"J", 0, 0).unwrap();
        self.cls.class_fields(self.av).unwrap().collection_extend(&self.av.jv, [p, q]).unwrap();
        let cls = self.define_empty();
        FatClass { p: cls.get_field_id(c"0", c"J").unwrap(), q: cls.get_field_id(c"1", c"J").unwrap(), cls }
    }
}
