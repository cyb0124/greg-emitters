use super::{
    cleaner::Cleanable,
    geometry::Rect,
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
use core::{any::Any, cell::RefCell};
use macros::dyn_abi;
use nalgebra::{Point2, Vector2};

impl<'a, T: JRef<'a>> GUIExt<'a> for T {}
pub trait GUIExt<'a>: JRef<'a> {
    fn player_container_menu(&self) -> Option<LocalRef<'a>> { self.get_object_field(objs().mv.player_container_menu) }
    fn menu_id(&self) -> i32 { self.get_int_field(objs().mv.container_menu_id) }

    // Called on Tile
    fn still_valid(&self, player: &impl JRef<'a>) -> bool {
        objs().mv.container.with_jni(self.jni()).call_static_bool_method(objs().mv.container_still_valid, &[self.raw(), player.raw()]).unwrap()
    }

    // Called on chat.Component
    fn to_formatted(&self) -> LocalRef<'a> { self.call_object_method(objs().mv.chat_component_to_formatted, &[]).unwrap().unwrap() }

    // Called on String
    fn literal(&self) -> LocalRef<'a> {
        let mv = &objs().mv;
        mv.chat_component.with_jni(self.jni()).call_static_object_method(mv.chat_component_literal, &[self.raw()]).unwrap().unwrap()
    }

    fn translatable(&self) -> LocalRef<'a> {
        let mv = &objs().mv;
        mv.chat_component.with_jni(self.jni()).call_static_object_method(mv.chat_component_translatable, &[self.raw()]).unwrap().unwrap()
    }
}

pub trait Menu: Cleanable {
    fn any(&self) -> &dyn Any;
    fn get_size(&self) -> Vector2<i32>;
    fn get_offset(&self) -> Vector2<i32>;
    fn render_bg(&self, lk: &GlobalMtx, gui: BorrowedRef, rect: Rect);
    fn mouse_clicked(&self, lk: &GlobalMtx, menu: BorrowedRef, rect: Rect, pos: Point2<f32>, button: i32) -> bool;
    fn still_valid(&self, player: BorrowedRef) -> bool;
}

pub trait MenuType: Send {
    fn new_client(&self, lk: &GlobalMtx, level: BorrowedRef<'static, '_>, data: &[u8]) -> Option<Arc<dyn Menu>>;
    fn raw(&self, lk: &GlobalMtx) -> usize;
}

pub struct GUIDefs {
    pub menu: FatWrapper<dyn Menu>,
    container_factory: FatWrapper<dyn MenuType>,
    menu_provider: ThinWrapper<MenuProvider>,
}

// Access to MenuType is guarded by the global lock, and the Arc is never shared.
unsafe impl Send for MenuProvider {}
struct MenuProvider {
    title: GlobalRef<'static>,
    menu_type: &'static dyn MenuType,
    menu: RefCell<Option<Arc<dyn Menu>>>,
    data: Vec<u8>,
}

impl Cleanable for MenuProvider {
    fn free(self: Arc<Self>, jni: &JNI) {
        let MenuProvider { title, menu, .. } = Arc::into_inner(self).unwrap();
        title.replace_jni(jni);
        menu.into_inner().map(|x| x.free(jni));
    }
}

impl GUIDefs {
    pub fn init(av: &AV<'static>, cn: &CN<Arc<CSig>>, mn: &MN<MSig>, fcn: &ForgeCN<Arc<CSig>>, fmn: &ForgeMN, namer: &ClassNamer) -> Self {
        let menu = ClassBuilder::new_1(av, namer, &cn.container_menu.slash)
            .native_2(&mn.container_menu_still_valid, still_valid_dyn())
            .native_2(&mn.container_menu_quick_move_stack, quick_move_stack_dyn())
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

    pub fn open_menu(
        &self,
        player: &impl JRef<'static>,
        menu_type: &'static dyn MenuType,
        menu: Arc<dyn Menu>,
        title: &impl JRef<'static>, // String
        data: Vec<u8>,
    ) {
        let title = title.translatable().new_global_ref().unwrap();
        let provider = MenuProvider { title, menu_type, menu: RefCell::new(Some(menu)), data };
        let provider = self.menu_provider.new_obj(player.jni(), Arc::new(provider));
        let network_hooks = objs().fmv.network_hooks.with_jni(player.jni());
        network_hooks.call_static_void_method(objs().fmv.network_hooks_open_screen, &[player.raw(), provider.raw, provider.raw]).unwrap()
    }
}

#[dyn_abi]
fn container_factory_create(jni: &'static JNI, this: usize, id: i32, inv: usize, data: usize) -> usize {
    let GlobalObjs { mtx, gui_defs, mv, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let this = gui_defs.container_factory.read(&lk, BorrowedRef::new(jni, &this));
    let player = BorrowedRef::new(jni, &inv).get_object_field(mv.inventory_player).unwrap();
    let level = player.call_object_method(mv.entity_level, &[]).unwrap().unwrap();
    let data = BorrowedRef::new(jni, &data).call_object_method(mv.friendly_byte_buf_read_byte_array, &[]).unwrap().unwrap();
    let Some(menu) = this.new_client(&lk, level.borrow(), &data.byte_elems().unwrap()) else { return 0 };
    let menu = gui_defs.menu.new_obj(jni, menu);
    menu.call_void_method(mv.container_menu_init, &[this.raw(&lk), id as _]).unwrap();
    menu.into_raw()
}

#[dyn_abi]
fn menu_provider_create_menu(jni: &JNI, this: usize, id: i32, _inv: usize, _player: usize) -> usize {
    let GlobalObjs { mtx, gui_defs, mv, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let this = gui_defs.menu_provider.read(&lk, BorrowedRef::new(jni, &this));
    let menu = gui_defs.menu.new_obj(jni, this.menu.borrow_mut().take().unwrap());
    menu.call_void_method(mv.container_menu_init, &[this.menu_type.raw(&lk), id as _]).unwrap();
    menu.into_raw()
}

#[dyn_abi]
fn menu_provider_get_display_name(jni: &JNI, this: usize) -> usize {
    let GlobalObjs { mtx, gui_defs, .. } = objs();
    gui_defs.menu_provider.read(&mtx.lock(jni).unwrap(), BorrowedRef::new(jni, &this)).title.raw
}

#[dyn_abi]
fn menu_provider_accept(jni: &JNI, this: usize, byte_buf: usize) {
    let GlobalObjs { mtx, gui_defs, mv, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let this = gui_defs.menu_provider.read(&lk, BorrowedRef::new(jni, &this));
    let ba = jni.new_byte_array(this.data.len() as _).unwrap();
    ba.write_byte_array(&this.data, 0).unwrap();
    BorrowedRef::new(jni, &byte_buf).call_object_method(mv.friendly_byte_buf_write_byte_array, &[ba.raw]).unwrap();
}

#[dyn_abi]
fn still_valid(jni: &JNI, this: usize, player: usize) -> bool {
    let lk = objs().mtx.lock(jni).unwrap();
    let this = objs().gui_defs.menu.read(&lk, BorrowedRef::new(jni, &this));
    this.still_valid(BorrowedRef::new(jni, &player))
}

#[dyn_abi]
fn quick_move_stack(_jni: &JNI, _this: usize, _player: usize, _index: i32) -> usize { 0 }
