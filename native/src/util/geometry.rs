use crate::{jvm::*, objs};
use core::f32::consts::FRAC_1_SQRT_2;
use nalgebra::{point, vector, Point2, Point3, Quaternion, Unit, UnitQuaternion, UnitVector3, Vector2, Vector3};
use num_traits::Signed;

pub const DIR_STEPS: [Vector3<i32>; 6] =
    [vector![0, -1, 0], vector![0, 1, 0], vector![0, 0, -1], vector![0, 0, 1], vector![-1, 0, 0], vector![1, 0, 0]];
pub const DIR_ADJS: [[u8; 4]; 6] = [[2, 5, 3, 4], [3, 5, 2, 4], [1, 5, 0, 4], [0, 5, 1, 4], [0, 3, 1, 2], [0, 2, 1, 3]];
pub const DIR_ATTS: [UnitQuaternion<f32>; 6] = [
    Unit::new_unchecked(Quaternion::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0., 0.)),
    Unit::new_unchecked(Quaternion::new(FRAC_1_SQRT_2, -FRAC_1_SQRT_2, 0., 0.)),
    Unit::new_unchecked(Quaternion::new(0., 1., 0., 0.)),
    Unit::new_unchecked(Quaternion::new(1., 0., 0., 0.)),
    Unit::new_unchecked(Quaternion::new(FRAC_1_SQRT_2, 0., -FRAC_1_SQRT_2, 0.)),
    Unit::new_unchecked(Quaternion::new(FRAC_1_SQRT_2, 0., FRAC_1_SQRT_2, 0.)),
];

impl<'a, T: JRef<'a>> GeomExt<'a> for T {}
pub trait GeomExt<'a>: JRef<'a> {
    fn read_dir(&self) -> u8 { self.get_int_field(objs().mv.dir_3d_data) as _ }
    fn read_vec3i(&self) -> Point3<i32> {
        let mv = &objs().mv;
        point![self.get_int_field(mv.vec3i_x), self.get_int_field(mv.vec3i_y), self.get_int_field(mv.vec3i_z)]
    }

    fn read_vec3d(&self) -> Point3<f64> {
        let mv = &objs().mv;
        point![self.get_double_field(mv.vec3d_x), self.get_double_field(mv.vec3d_y), self.get_double_field(mv.vec3d_z)]
    }

    fn read_chunk_pos(&self) -> Point2<i32> {
        let mv = &objs().mv;
        point![self.get_int_field(mv.chunk_pos_x), self.get_int_field(mv.chunk_pos_z)]
    }
}

pub fn mul_i(v: Vector2<f32>) -> Vector2<f32> { vector![-v.y, v.x] }
pub fn mul_ni(v: Vector2<f32>) -> Vector2<f32> { vector![v.y, -v.x] }
pub fn lerp(a: f32, b: f32, t: f32) -> f32 { a * (1. - t) + b * t }
pub fn write_dir<'a>(jni: &'a JNI, dir: u8) -> LocalRef<'a> { objs().mv.dir_by_3d_data.with_jni(jni).get_object_elem(dir as _).unwrap().unwrap() }
pub fn block_to_chunk(pos: Point3<i32>) -> Point2<i32> { pos.xz().map(|x| x.div_euclid(16)) }

pub fn write_block_pos(jni: &JNI, v: Point3<i32>) -> LocalRef {
    let mv = &objs().mv;
    mv.block_pos.with_jni(jni).new_object(mv.block_pos_init, &[v.x as _, v.y as _, v.z as _]).unwrap()
}

pub fn write_vec3d(jni: &JNI, v: Point3<f64>) -> LocalRef {
    let mv = &objs().mv;
    mv.vec3d.with_jni(jni).new_object(mv.vec3d_init, &[d_raw(v.x), d_raw(v.y), d_raw(v.z)]).unwrap()
}

pub fn new_voxel_shape<'a>(jni: &'a JNI, min: Point3<f32>, max: Point3<f32>) -> GlobalRef<'a> {
    let mv = &objs().mv;
    let args = [min.x, min.y, min.z, max.x, max.y, max.z].map(|x| d_raw(x as _));
    mv.shapes.with_jni(jni).call_static_object_method(mv.shapes_create, &args).unwrap().unwrap().new_global_ref().unwrap()
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub min: Point2<f32>,
    pub max: Point2<f32>,
}

impl Rect {
    pub fn width(&self) -> f32 { self.max.x - self.min.x }
    pub fn height(&self) -> f32 { self.max.y - self.min.y }
    pub fn size(&self) -> Vector2<f32> { self.max - self.min }
    pub fn center(&self) -> Point2<f32> { self.min.lerp(&self.max, 0.5) }
    pub fn center_top(&self) -> Point2<f32> { point!(self.center().x, self.min.y) }
    pub fn center_bottom(&self) -> Point2<f32> { point!(self.center().x, self.max.y) }
    pub fn left_center(&self) -> Point2<f32> { point!(self.min.x, self.center().y) }
    pub fn right_center(&self) -> Point2<f32> { point!(self.max.x, self.center().y) }
    pub fn contains(&self, p: Point2<f32>) -> bool { self.min.x <= p.x && p.x <= self.max.x && self.min.y <= p.y && p.y <= self.max.y }
    pub fn from_center_size(center: Point2<f32>, size: Vector2<f32>) -> Self { Self { min: center - size * 0.5, max: center + size * 0.5 } }
}

pub struct CoveringBlocks {
    dir: UnitVector3<f32>,
    inv_dir: Vector3<f32>,
    pub pos: Point3<i32>,
    pub frac: Vector3<f32>,
}

impl CoveringBlocks {
    pub fn new(pos: Point3<i32>, frac: Vector3<f32>, dir: UnitVector3<f32>) -> Self { Self { dir, inv_dir: dir.map(|x| x.abs().recip()), pos, frac } }
    pub fn step(&mut self) {
        let cost = self.frac.zip_map(&self.dir, |x, d| if d < 0. { x } else { 1. - x }).component_mul(&self.inv_dir);
        let (axis, cost) = cost.argmin();
        self.frac += *self.dir * cost;
        let (new_frac, step) = if self.dir[axis] < 0. { (1., -1) } else { (0., 1) };
        self.frac[axis] = new_frac;
        self.pos[axis] += step
    }
}
