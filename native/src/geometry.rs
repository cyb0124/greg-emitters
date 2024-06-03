use crate::{jvm::*, objs};
use core::f32::consts::FRAC_1_SQRT_2;
use nalgebra::{Point3, Quaternion, Unit, UnitQuaternion};

pub const DIR_ATTS: [UnitQuaternion<f32>; 6] = [
    Unit::new_unchecked(Quaternion::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0., 0.)),
    Unit::new_unchecked(Quaternion::new(FRAC_1_SQRT_2, -FRAC_1_SQRT_2, 0., 0.)),
    Unit::new_unchecked(Quaternion::new(0., 1., 0., 0.)),
    Unit::new_unchecked(Quaternion::new(1., 0., 0., 0.)),
    Unit::new_unchecked(Quaternion::new(FRAC_1_SQRT_2, 0., -FRAC_1_SQRT_2, 0.)),
    Unit::new_unchecked(Quaternion::new(FRAC_1_SQRT_2, 0., FRAC_1_SQRT_2, 0.)),
];

pub fn new_voxel_shape<'a>(jni: &'a JNI, min: Point3<f32>, max: Point3<f32>) -> GlobalRef<'a> {
    let mv = &objs().mv;
    let args = [min.x, min.y, min.z, max.x, max.y, max.z].map(|x| d_raw(x as _));
    mv.shapes.with_jni(jni).call_static_object_method(mv.shapes_create, &args).unwrap().unwrap().new_global_ref().unwrap()
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 { a * (1. - t) + b * t }
