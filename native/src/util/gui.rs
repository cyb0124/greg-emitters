use super::{
    cleaner::Cleanable,
    mapping::{ForgeCN, ForgeMN, CN, MN},
    ClassBuilder, ClassNamer, FatWrapper, ThinWrapper,
};
use crate::{
    asm::*,
    global::{GlobalMtx, GlobalObjs},
    jvm::*,
    mapping_base::*,
    objs,
};
use alloc::{sync::Arc, vec::Vec};
use core::{cell::RefCell, ffi::CStr};
use macros::dyn_abi;

pub trait Menu: Cleanable {}

pub trait MenuType: Send {
    fn new_client(&self, data: &[u8]) -> Arc<dyn Menu>;
    fn title(&self) -> &CStr;
    fn raw(&self, lk: &GlobalMtx) -> usize;
}

pub struct GUIDefs {
    menu: FatWrapper<dyn Menu>,
    container_factory: FatWrapper<dyn MenuType>,
    menu_provider: ThinWrapper<MenuProvider>,
}

// Access to MenuType is guarded by the global lock, and the Arc is never shared.
unsafe impl Send for MenuProvider {}
struct MenuProvider {
    menu_type: &'static dyn MenuType,
    menu: RefCell<Option<Arc<dyn Menu>>>,
    data: RefCell<Option<Vec<u8>>>,
}

impl Cleanable for MenuProvider {
    fn free(self: Arc<Self>, jni: &JNI) { Arc::into_inner(self).unwrap().menu.into_inner().map(|x| x.free(jni)); }
}

impl GUIDefs {
    pub fn init(av: &AV<'static>, cn: &CN<Arc<CSig>>, mn: &MN<MSig>, fcn: &ForgeCN<Arc<CSig>>, fmn: &ForgeMN, namer: &ClassNamer) -> Self {
        let menu = ClassBuilder::new_1(av, namer, &cn.abstract_container_menu.slash)
            .native_2(&mn.abstract_container_menu_still_valid, still_valid_dyn())
            .native_2(&mn.abstract_container_menu_quick_move_stack, quick_move_stack_dyn())
            .define_fat()
            .wrap::<dyn Menu>();
        let container_factory = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([&*fcn.container_factory.slash])
            .native_2(&fmn.container_factory_create, container_factory_create_dyn())
            .define_fat()
            .wrap::<dyn MenuType>();
        let menu_provider = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([&*cn.menu_provider.slash, c"java/util/function/Consumer"])
            .native_2(&mn.menu_provider_create_menu, menu_provider_create_menu_dyn())
            .native_2(&mn.menu_provider_get_display_name, menu_provider_get_display_name_dyn())
            .native_1(c"accept", c"(Ljava/lang/Object;)V", menu_provider_accept_dyn())
            .define_thin()
            .wrap::<MenuProvider>();
        Self { menu, container_factory, menu_provider }
    }

    pub fn new_menu_type<'a>(&self, jni: &'a JNI, menu_type: &'static dyn MenuType) -> LocalRef<'a> {
        let fmv = &objs().fmv;
        let container_factory = self.container_factory.new_static(jni, menu_type);
        fmv.forge_menu_type.with_jni(jni).call_static_object_method(fmv.forge_menu_type_create, &[container_factory.raw]).unwrap().unwrap()
    }

    pub fn open_menu<'a>(&self, player: &impl JRef<'a>, menu_type: &'static dyn MenuType, menu: Arc<dyn Menu>, data: Vec<u8>) {
        let fmv = &objs().fmv;
        let provider = MenuProvider { menu_type, menu: RefCell::new(Some(menu)), data: RefCell::new(Some(data)) };
        let provider = self.menu_provider.new_obj(player.jni(), Arc::new(provider));
        let network_hooks = fmv.network_hooks.with_jni(player.jni());
        network_hooks.call_static_void_method(fmv.network_hooks_open_screen, &[player.raw(), provider.raw, provider.raw]).unwrap()
    }
}

#[dyn_abi]
fn container_factory_create(jni: &JNI, this: usize, id: i32, _inv: usize, data: usize) -> usize {
    let GlobalObjs { mtx, gui_defs, mv, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let this = gui_defs.container_factory.read(&lk, BorrowedRef::new(jni, &this));
    let data = BorrowedRef::new(jni, &data).call_object_method(mv.friendly_byte_buf_read_byte_array, &[]).unwrap().unwrap();
    let menu = this.new_client(&data.crit_elems().unwrap());
    let menu = gui_defs.menu.new_obj(jni, menu);
    menu.call_void_method(mv.abstract_container_menu_init, &[this.raw(&lk), id as _]).unwrap();
    menu.into_raw()
}

#[dyn_abi]
fn menu_provider_create_menu(jni: &JNI, this: usize, id: i32, _inv: usize, _player: usize) -> usize {
    let GlobalObjs { mtx, gui_defs, mv, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let this = gui_defs.menu_provider.read(&lk, BorrowedRef::new(jni, &this));
    let menu = gui_defs.menu.new_obj(jni, this.menu.borrow_mut().take().unwrap());
    menu.call_void_method(mv.abstract_container_menu_init, &[this.menu_type.raw(&lk), id as _]).unwrap();
    menu.into_raw()
}

#[dyn_abi]
fn menu_provider_get_display_name(jni: &JNI, this: usize) -> usize {
    let GlobalObjs { mtx, gui_defs, mv, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let title = jni.new_utf(gui_defs.menu_provider.read(&lk, BorrowedRef::new(jni, &this)).menu_type.title()).unwrap();
    mv.chat_component.with_jni(jni).call_static_object_method(mv.chat_component_translatable, &[title.raw]).unwrap().unwrap().into_raw()
}

#[dyn_abi]
fn menu_provider_accept(jni: &JNI, this: usize, byte_buf: usize) {
    let GlobalObjs { mtx, gui_defs, mv, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let this = gui_defs.menu_provider.read(&lk, BorrowedRef::new(jni, &this));
    let data = this.data.borrow_mut().take().unwrap();
    let ba = jni.new_byte_array(data.len() as _).unwrap();
    ba.write_byte_array(&data, 0).unwrap();
    BorrowedRef::new(jni, &byte_buf).call_object_method(mv.friendly_byte_buf_write_byte_array, &[ba.raw]).unwrap();
}

#[dyn_abi]
fn still_valid(jni: &JNI, this: usize, player: usize) -> bool { true }

#[dyn_abi]
fn quick_move_stack(jni: &JNI, this: usize, player: usize, index: i32) -> usize { 0 }
