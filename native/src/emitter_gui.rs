use crate::{
    global::GlobalMtx,
    jvm::*,
    util::{
        cleaner::Cleanable,
        client::ClientExt,
        gui::{Menu, MenuType},
        tessellator::{Rect, Rounding, Stroke, Tessellator},
    },
};
use alloc::sync::Arc;
use core::f32::consts::PI;
use nalgebra::{vector, Rotation2, Vector2, Vector4};

pub struct EmitterMenuType;
pub struct EmitterMenu {}

impl MenuType for EmitterMenuType {
    fn new_client(&self, data: &[u8]) -> Arc<dyn Menu> { Arc::new(EmitterMenu {}) }
    fn raw(&self, lk: &GlobalMtx) -> usize { lk.emitter_blocks.get().unwrap().menu_type.raw }
}

impl Cleanable for EmitterMenu {
    fn free(self: Arc<Self>, jni: &crate::JNI) {}
}

const GRID_RADIUS: f32 = 60.;

impl Menu for EmitterMenu {
    fn get_size(&self) -> Vector2<i32> { vector![135, 135 + 12] }
    fn get_offset(&self) -> Vector2<i32> { vector![135, 0] }

    fn render_bg(&self, lk: &GlobalMtx, screen: BorrowedRef, gui: BorrowedRef, rect: Rect) {
        let mut tess = Tessellator::new(gui.jni);
        tess.rect(rect, Rounding::same(4.), 0., vector![1., 1., 1., 0.5], &Stroke::new(1., vector![0., 0., 0., 1.]));
        let mut center = rect.center();
        center.y += 6.;
        let grid_stroke = Stroke::new(1., vector![0.25, 0.25, 0.25, 1.]);
        for i in 1..=3 {
            let radius = (i * 30) as f32 * (GRID_RADIUS / 90.);
            tess.circle(center, radius, Vector4::zeros(), &grid_stroke);
        }
        let mut dir = vector![GRID_RADIUS, 0.];
        let step = Rotation2::new(PI / 6.);
        for _ in 0..12 {
            tess.line([center, center + dir], &grid_stroke);
            dir = step * dir
        }
        gui.gui_draw_mesh(&mut tess.mesh)
    }
}
