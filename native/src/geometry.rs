use crate::{jvm::*, objs};
use nalgebra::Point3;

pub fn new_voxel_shape<'a>(jni: &'a JNI, min: Point3<f32>, max: Point3<f32>) -> LocalRef<'a> {
    let mv = &objs().mv;
    let args = [min.x, min.y, min.z, max.x, max.y, max.z].map(|x| d_raw(x as _));
    mv.shapes.with_jni(jni).call_static_object_method(mv.shapes_create, &args).unwrap().unwrap()
}
