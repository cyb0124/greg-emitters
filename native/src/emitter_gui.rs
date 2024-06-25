use crate::{
    emitter_blocks::Emitter,
    global::GlobalMtx,
    jvm::*,
    objs,
    util::{
        cleaner::Cleanable,
        client::ClientExt,
        geometry::write_block_pos,
        gui::{Menu, MenuType},
        tessellator::{Rect, Rounding, Stroke, Tessellator},
        tile::TileExt,
    },
};
use alloc::sync::Arc;
use core::f32::consts::PI;
use nalgebra::{vector, Point3, Rotation2, Vector2, Vector4};

pub struct EmitterMenuType;
pub struct EmitterMenu {
    pub tile: WeakGlobalRef<'static>,
}

impl MenuType for EmitterMenuType {
    fn new_client(&self, lk: &GlobalMtx, level: BorrowedRef<'static, '_>, data: &[u8]) -> Option<Arc<dyn Menu>> {
        let pos: Point3<i32> = postcard::from_bytes(data).ok()?;
        let tile = level.tile_at(write_block_pos(level.jni, pos).raw)?;
        let true = tile.is_instance_of(objs().tile_defs.tile.cls.cls.raw) else { return None };
        let true = objs().tile_defs.tile.read(lk, tile.borrow()).any().is::<Emitter>() else { return None };
        Some(Arc::new(EmitterMenu { tile: tile.new_weak_global_ref().unwrap() }))
    }

    fn raw(&self, lk: &GlobalMtx) -> usize { lk.emitter_blocks.get().unwrap().menu_type.raw }
}

impl Cleanable for EmitterMenu {
    fn free(self: Arc<Self>, jni: &crate::JNI) { Arc::into_inner(self).unwrap().tile.replace_jni(jni); }
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
