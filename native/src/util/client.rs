use super::{
    geometry::lerp,
    mapping::{CN, MN},
    ClassBuilder, ClassNamer, AV, OP_ALOAD, OP_ARETURN,
};
use crate::{global::GlobalMtx, jvm::*, mapping_base::*, objs, registry::make_resource_loc};
use alloc::sync::Arc;
use core::{ffi::CStr, mem::MaybeUninit};
use macros::dyn_abi;
use nalgebra::{point, vector, Affine3, ArrayStorage, Matrix4, Point2, Point3, Vector3};

impl<'a, T: JRef<'a>> ClientExt<'a> for T {}
pub trait ClientExt<'a>: JRef<'a> {
    // Called on PoseStack
    fn last_pose(&self) -> Affine3<f32> {
        let mvc = objs().mv.client.uref();
        let pose = self.call_object_method(mvc.pose_stack_last, &[]).unwrap().unwrap().get_object_field(mvc.pose_pose).unwrap();
        let mut pose_data = MaybeUninit::<ArrayStorage<f32, 4, 4>>::uninit();
        pose.call_object_method(mvc.matrix4fc_read, &[pose_data.as_mut_ptr() as _]).unwrap();
        Affine3::from_matrix_unchecked(Matrix4::from_data(unsafe { pose_data.assume_init() }))
    }
}

pub struct ClientDefs {
    pub tile_renderer: GlobalRef<'static>,
    pub screen_constructor: GlobalRef<'static>,
    container_screen: GlobalRef<'static>,
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
            .define_empty();
        Self {
            tile_renderer: tile_renderer.alloc_object().unwrap().new_global_ref().unwrap(),
            screen_constructor: screen_constructor.alloc_object().unwrap().new_global_ref().unwrap(),
            container_screen,
        }
    }
}

#[dyn_abi]
fn container_screen_render_bg(jni: &JNI, this: usize, gui_graphics: usize, partial_tick: f32, mx: i32, my: i32) {}

#[dyn_abi]
fn container_screen_minit(jni: &JNI, this: usize) {
    let mvc = objs().mv.client.uref();
    let lk = objs().mtx.lock(jni).unwrap();
    let this = BorrowedRef::new(jni, &this);
    let menu = this.get_object_field(mvc.container_screen_menu).unwrap();
    let menu = objs().gui_defs.menu.read(&lk, menu.borrow());
    let max_size = vector![this.get_int_field(mvc.container_screen_width), this.get_int_field(mvc.container_screen_height)];
    let pos = (max_size - menu.get_size()) / 2 + menu.get_offset();
    this.set_int_field(mvc.container_screen_left, pos.x);
    this.set_int_field(mvc.container_screen_top, pos.y)
}

#[dyn_abi]
fn container_screen_render_labels(jni: &JNI, this: usize, gui_graphics: usize, mx: i32, my: i32) {
    let mvc = objs().mv.client.uref();
    let this = BorrowedRef::new(jni, &this);
    BorrowedRef::new(jni, &gui_graphics)
        .call_int_method(
            mvc.gui_graphics_draw_string,
            &[
                this.get_object_field(mvc.container_screen_font).unwrap().raw,
                this.get_object_field(mvc.container_screen_title).unwrap().raw,
                this.get_int_field(mvc.container_screen_title_x) as _,
                this.get_int_field(mvc.container_screen_title_y) as _,
                0x404040,
                0,
            ],
        )
        .unwrap();
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
