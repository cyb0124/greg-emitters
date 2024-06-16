use super::geometry::lerp;
use crate::{global::GlobalMtx, jvm::*, mapping_base::*, objs, registry::make_resource_loc};
use core::{ffi::CStr, mem::MaybeUninit};
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
