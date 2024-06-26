use super::{
    geometry::{lerp, Rect},
    gui::{GUIExt, Menu},
    mapping::{CN, MN},
    tessellator::Mesh,
    ClassBuilder, ClassNamer, AV, OP_ALOAD, OP_ARETURN,
};
use crate::{global::GlobalMtx, jvm::*, mapping_base::*, objs, registry::make_resource_loc};
use alloc::sync::Arc;
use core::{ffi::CStr, mem::MaybeUninit};
use macros::dyn_abi;
use nalgebra::{point, vector, Affine3, ArrayStorage, Matrix4, Point2, Point3, Vector3};

impl<'a, T: JRef<'a>> ClientExt<'a> for T {}
pub trait ClientExt<'a>: JRef<'a> {
    fn font_width(&self, formatted: &impl JRef<'a>) -> i32 { self.call_int_method(objs().mv.client.uref().font_width, &[formatted.raw()]).unwrap() }
    fn screen_font(&self) -> LocalRef<'a> { self.get_object_field(objs().mv.client.uref().screen_font).unwrap() }
    fn screen_pos(&self) -> Point2<i32> {
        let mvc = objs().mv.client.uref();
        point![self.get_int_field(mvc.container_screen_left), self.get_int_field(mvc.container_screen_top)]
    }

    fn gui_draw_formatted(&self, font: &impl JRef<'a>, formatted: &impl JRef<'a>, x: i32, y: i32, color: i32, drop_shadow: bool) {
        self.call_int_method(
            objs().mv.client.uref().gui_graphics_draw_formatted,
            &[font.raw(), formatted.raw(), x as _, y as _, color as _, drop_shadow as _],
        )
        .unwrap();
    }

    fn gui_draw_mesh(&self, mesh: &mut Mesh) {
        let mvc = objs().mv.client.uref();
        let pose = self.get_object_field(mvc.gui_graphics_pose).unwrap();
        let pose = pose.call_object_method(mvc.pose_stack_last, &[]).unwrap().unwrap().get_object_field(mvc.pose_pose).unwrap();
        let render_sys = mvc.render_sys.with_jni(self.jni());
        render_sys.call_static_void_method(mvc.render_sys_set_shader, &[objs().client_defs.uref().pos_color_shader_supplier.raw]).unwrap();
        render_sys.call_static_void_method(mvc.render_sys_enable_blend, &[]).unwrap();
        render_sys.call_static_void_method(mvc.render_sys_disable_cull, &[]).unwrap();
        let tess = mvc.tesselator.with_jni(self.jni()).call_static_object_method(mvc.tesselator_get_inst, &[]).unwrap().unwrap();
        let vb = tess.call_object_method(mvc.tesselator_get_builder, &[]).unwrap().unwrap();
        vb.call_void_method(mvc.buffer_builder_begin, &[mvc.vertex_mode_tris.raw, mvc.default_vertex_fmt_pos_color.raw]).unwrap();
        for &idx in &mesh.indices {
            let v = &mesh.vertices[idx as usize];
            vb.call_object_method(mvc.vertex_consumer_pos, &[pose.raw, f_raw(v.pos.x), f_raw(v.pos.y), f_raw(0.)]).unwrap();
            vb.call_object_method(mvc.vertex_consumer_color, &[f_raw(v.color.x), f_raw(v.color.y), f_raw(v.color.z), f_raw(v.color.w)]).unwrap();
            vb.call_void_method(mvc.vertex_consumer_end_vertex, &[]).unwrap()
        }
        mesh.indices.clear();
        mesh.vertices.clear();
        tess.call_void_method(mvc.tesselator_end, &[]).unwrap();
        render_sys.call_static_void_method(mvc.render_sys_enable_cull, &[]).unwrap();
        render_sys.call_static_void_method(mvc.render_sys_disable_blend, &[]).unwrap()
    }

    // Called on PoseStack
    fn last_pose(&self) -> Affine3<f32> {
        let mvc = objs().mv.client.uref();
        let pose = self.call_object_method(mvc.pose_stack_last, &[]).unwrap().unwrap().get_object_field(mvc.pose_pose).unwrap();
        let mut pose_data = MaybeUninit::<ArrayStorage<f32, 4, 4>>::uninit();
        pose.call_object_method(mvc.matrix4fc_read, &[pose_data.as_mut_ptr() as _]).unwrap();
        Affine3::from_matrix_unchecked(Matrix4::from_data(unsafe { pose_data.assume_init() }))
    }
}

pub fn play_btn_click_sound(jni: &JNI) {
    let mvc = objs().mv.client.uref();
    let args = [objs().mv.sound_evts_ui_btn_click.raw, f_raw(1.)];
    let inst = mvc.simple_sound_inst.with_jni(jni).call_static_object_method(mvc.simple_sound_inst_for_ui_holder, &args).unwrap().unwrap();
    let mgr = mvc.mc_inst.with_jni(jni).call_object_method(mvc.mc_get_sound_mgr, &[]).unwrap().unwrap();
    mgr.call_void_method(mvc.sound_mgr_play, &[inst.raw]).unwrap()
}

pub struct ClientDefs {
    pub tile_renderer: GlobalRef<'static>,
    pub screen_constructor: GlobalRef<'static>,
    container_screen: GlobalRef<'static>,
    pos_color_shader_supplier: GlobalRef<'static>,
}

impl ClientDefs {
    pub fn init(av: &AV<'static>, namer: &ClassNamer, cn: &CN<Arc<CSig>>, mn: &MN<MSig>) -> Self {
        let jni = av.ldr.jni;
        let tile_renderer = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([&*cn.tile_renderer_provider.slash, &cn.tile_renderer.slash])
            .native_2(&mn.tile_renderer_render, render_tile_dyn())
            .insns(&mn.tile_renderer_provider_create, [av.new_var_insn(jni, OP_ALOAD, 0).unwrap(), av.new_insn(jni, OP_ARETURN).unwrap()])
            .define_empty();
        let screen_constructor = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([&*cn.screen_constructor.slash])
            .native_2(&mn.screen_constructor_create, screen_constructor_create_dyn())
            .define_empty();
        let container_screen = ClassBuilder::new_1(av, namer, &cn.container_screen.slash)
            .native_2(&mn.container_screen_render_bg, container_screen_render_bg_dyn())
            .native_2(&mn.container_screen_render_labels, container_screen_render_labels_dyn())
            .native_2(&mn.container_screen_minit, container_screen_minit_dyn())
            .native_2(&mn.container_screen_mouse_clicked, container_screen_mouse_clicked_dyn())
            .native_2(&mn.container_screen_mouse_dragged, container_screen_mouse_dragged_dyn())
            .native_2(&mn.container_screen_mouse_released, container_screen_mouse_released_dyn())
            .define_empty();
        let pos_color_shader_supplier = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([c"java/util/function/Supplier"])
            .native_1(c"get", c"()Ljava/lang/Object;", pos_color_shader_supplier_dyn())
            .define_empty();
        Self {
            tile_renderer: tile_renderer.alloc_object().unwrap().new_global_ref().unwrap(),
            screen_constructor: screen_constructor.alloc_object().unwrap().new_global_ref().unwrap(),
            container_screen,
            pos_color_shader_supplier: pos_color_shader_supplier.alloc_object().unwrap().new_global_ref().unwrap(),
        }
    }
}

#[dyn_abi]
fn pos_color_shader_supplier(jni: &JNI, _: usize) -> usize {
    let mvc = objs().mv.client.uref();
    mvc.game_renderer.with_jni(jni).call_static_object_method(mvc.game_renderer_get_pos_color_shader, &[]).unwrap().unwrap().into_raw()
}

fn container_screen_rect(screen: BorrowedRef, menu: &dyn Menu) -> Rect {
    let min = screen.screen_pos();
    Rect { min: min.cast(), max: (min + menu.get_size()).cast() }
}

#[dyn_abi]
fn container_screen_render_bg(jni: &JNI, this: usize, gui_graphics: usize, _partial_tick: f32, _mx: i32, _my: i32) {
    let mvc = objs().mv.client.uref();
    let this = BorrowedRef::new(jni, &this);
    let menu = this.get_object_field(mvc.container_screen_menu).unwrap();
    let lk = objs().mtx.lock(jni).unwrap();
    let menu = objs().gui_defs.menu.read(&lk, menu.borrow());
    if menu.should_draw_dark_bg() {
        this.call_void_method(mvc.screen_render_background, &[gui_graphics]).unwrap()
    }
    menu.render_bg(&lk, this, BorrowedRef::new(jni, &gui_graphics), container_screen_rect(this, menu))
}

#[dyn_abi]
fn container_screen_mouse_clicked(jni: &JNI, this: usize, mx: f64, my: f64, button: i32) -> bool {
    let mvc = objs().mv.client.uref();
    let this = BorrowedRef::new(jni, &this);
    let j_menu = this.get_object_field(mvc.container_screen_menu).unwrap();
    let lk = objs().mtx.lock(jni).unwrap();
    let menu = objs().gui_defs.menu.read(&lk, j_menu.borrow());
    let false = menu.mouse_clicked(&lk, j_menu.borrow(), container_screen_rect(this, menu), point![mx, my].cast(), button) else { return true };
    this.call_nonvirtual_bool_method(mvc.container_screen.raw, mvc.container_screen_mouse_clicked, &[d_raw(mx), d_raw(my), button as _]).unwrap()
}

#[dyn_abi]
fn container_screen_mouse_dragged(jni: &JNI, this: usize, mx: f64, my: f64, button: i32, dx: f64, dy: f64) -> bool {
    let mvc = objs().mv.client.uref();
    let this = BorrowedRef::new(jni, &this);
    let j_menu = this.get_object_field(mvc.container_screen_menu).unwrap();
    let lk = objs().mtx.lock(jni).unwrap();
    let menu = objs().gui_defs.menu.read(&lk, j_menu.borrow());
    let false = menu.mouse_dragged(&lk, j_menu.borrow(), container_screen_rect(this, menu), point![mx, my].cast()) else { return true };
    let args = [d_raw(mx), d_raw(my), button as _, d_raw(dx), d_raw(dy)];
    this.call_nonvirtual_bool_method(mvc.container_screen.raw, mvc.container_screen_mouse_dragged, &args).unwrap()
}

#[dyn_abi]
fn container_screen_mouse_released(jni: &JNI, this: usize, mx: f64, my: f64, button: i32) -> bool {
    let mvc = objs().mv.client.uref();
    let this = BorrowedRef::new(jni, &this);
    let menu = this.get_object_field(mvc.container_screen_menu).unwrap();
    let lk = objs().mtx.lock(jni).unwrap();
    let menu = objs().gui_defs.menu.read(&lk, menu.borrow());
    let false = menu.mouse_released(&lk, button) else { return true };
    this.call_nonvirtual_bool_method(mvc.container_screen.raw, mvc.container_screen_mouse_released, &[d_raw(mx), d_raw(my), button as _]).unwrap()
}

#[dyn_abi]
fn container_screen_minit(jni: &JNI, this: usize) {
    let mvc = objs().mv.client.uref();
    let lk = objs().mtx.lock(jni).unwrap();
    let this = BorrowedRef::new(jni, &this);
    let menu = this.get_object_field(mvc.container_screen_menu).unwrap();
    let menu = objs().gui_defs.menu.read(&lk, menu.borrow());
    let max_size = vector![this.get_int_field(mvc.screen_width), this.get_int_field(mvc.screen_height)];
    let pos = (max_size - menu.get_size()) / 2 + menu.get_offset();
    this.set_int_field(mvc.container_screen_left, pos.x);
    this.set_int_field(mvc.container_screen_top, pos.y)
}

#[dyn_abi]
fn container_screen_render_labels(jni: &JNI, this: usize, gui_graphics: usize, _mx: i32, _my: i32) {
    let mvc = objs().mv.client.uref();
    let this = BorrowedRef::new(jni, &this);
    BorrowedRef::new(jni, &gui_graphics).gui_draw_formatted(
        &this.screen_font(),
        &this.get_object_field(mvc.screen_title).unwrap().to_formatted(),
        this.get_int_field(mvc.container_screen_title_x),
        this.get_int_field(mvc.container_screen_title_y),
        0x404040,
        false,
    )
}

#[dyn_abi]
fn screen_constructor_create(jni: &JNI, _: usize, menu: usize, inv: usize, title: usize) -> usize {
    let defs = objs().client_defs.uref();
    let mvc = objs().mv.client.uref();
    let screen = defs.container_screen.with_jni(jni).new_object(mvc.container_screen_init, &[menu, inv, title]).unwrap();
    let lk = objs().mtx.lock(jni).unwrap();
    let size = objs().gui_defs.menu.read(&lk, BorrowedRef::new(jni, &menu)).get_size();
    screen.set_int_field(mvc.container_screen_img_width, size.x);
    screen.set_int_field(mvc.container_screen_img_height, size.y);
    screen.into_raw()
}

#[dyn_abi]
fn render_tile(jni: &JNI, _: usize, tile: usize, _: f32, pose_stack: usize, buffer_source: usize, light: i32, overlay: i32) {
    let lk = objs().mtx.lock(jni).unwrap();
    let dc = DrawContext::new(&lk, &BorrowedRef::new(jni, &buffer_source), light, overlay);
    let pose = BorrowedRef::new(jni, &pose_stack).last_pose();
    objs().tile_defs.tile.read(&lk, BorrowedRef::new(jni, &tile)).render(&lk, dc, pose)
}

#[derive(Clone, Copy)]
pub struct Sprite {
    pub uv0: Point2<f32>,
    pub uv1: Point2<f32>,
}

impl Sprite {
    pub fn new<'a>(atlas: &impl JRef<'a>, namespace: &CStr, id: &CStr) -> Self {
        let mvc = &objs().mv.client.uref();
        let loc = make_resource_loc(atlas.jni(), namespace, id);
        let sprite = atlas.call_object_method(mvc.atlas_get_sprite, &[loc.raw]).unwrap().unwrap();
        Self {
            uv0: point![sprite.get_float_field(mvc.sprite_u0), sprite.get_float_field(mvc.sprite_v0)],
            uv1: point![sprite.get_float_field(mvc.sprite_u1), sprite.get_float_field(mvc.sprite_v1)],
        }
    }

    pub fn lerp_u(&self, t: f32) -> f32 { lerp(self.uv0.x, self.uv1.x, t) }
    pub fn lerp_v(&self, t: f32) -> f32 { lerp(self.uv0.y, self.uv1.y, t) }
    pub fn sub(&self, u0: f32, v0: f32, u1: f32, v1: f32) -> Self {
        Self { uv0: point![self.lerp_u(u0), self.lerp_v(v0)], uv1: point![self.lerp_u(u1), self.lerp_v(v1)] }
    }
}

pub struct DrawContext<'a> {
    buffer: LocalRef<'a>,
    light: i32,
    overlay: i32,
}

impl<'a> DrawContext<'a> {
    pub fn new(lk: &GlobalMtx, buffer_source: &impl JRef<'a>, light: i32, overlay: i32) -> Self {
        let sheets = lk.sheets_solid.get().unwrap().raw;
        let buffer = buffer_source.call_object_method(objs().mv.client.uref().buffer_source_get_buffer, &[sheets]).unwrap().unwrap();
        Self { buffer, light, overlay }
    }

    pub fn vertex(&mut self, p: Point3<f32>, n: Vector3<f32>, u: f32, v: f32) {
        let args = [
            f_raw(p.x),
            f_raw(p.y),
            f_raw(p.z),
            f_raw(1.),
            f_raw(1.),
            f_raw(1.),
            f_raw(1.),
            f_raw(u),
            f_raw(v),
            self.overlay as _,
            self.light as _,
            f_raw(n.x),
            f_raw(n.y),
            f_raw(n.z),
        ];
        self.buffer.call_void_method(objs().mv.client.uref().vertex_consumer_vertex, &args).unwrap()
    }

    pub fn square(&mut self, sprite: &Sprite, att: &Affine3<f32>) {
        let n = (att * vector![0., 0., 1.]).normalize();
        self.vertex(att * point![-0.5, -0.5, 0.], n, sprite.uv0.x, sprite.uv0.y);
        self.vertex(att * point![0.5, -0.5, 0.], n, sprite.uv1.x, sprite.uv0.y);
        self.vertex(att * point![0.5, 0.5, 0.], n, sprite.uv1.x, sprite.uv1.y);
        self.vertex(att * point![-0.5, 0.5, 0.], n, sprite.uv0.x, sprite.uv1.y)
    }
}
