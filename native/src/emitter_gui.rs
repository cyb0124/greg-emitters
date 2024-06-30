use crate::{
    emitter_blocks::{CommonData, Emitter},
    global::GlobalMtx,
    jvm::*,
    objs,
    packets::C2S,
    util::{
        cleaner::Cleanable,
        client::{play_btn_click_sound, ClientExt},
        geometry::{write_block_pos, Rect, DIR_ADJS},
        gui::{GUIExt, Menu, MenuType},
        tessellator::{Rounding, Stroke, Tessellator},
        tile::TileExt,
    },
};
use alloc::sync::Arc;
use core::{
    any::Any,
    cell::Cell,
    f32::consts::{FRAC_PI_2, FRAC_PI_6},
};
use nalgebra::{vector, Point2, Point3, Rotation2, Vector2, Vector4};

pub struct EmitterMenuType;
pub struct EmitterMenu {
    pub tile: WeakGlobalRef<'static>,
    pub dragged: Cell<bool>,
}

impl MenuType for EmitterMenuType {
    fn raw(&self, lk: &GlobalMtx) -> usize { lk.emitter_blocks.get().unwrap().menu_type.raw }
    fn new_client(&self, _lk: &GlobalMtx, level: BorrowedRef<'static, '_>, data: &[u8]) -> Option<Arc<dyn Menu>> {
        let pos: Point3<i32> = postcard::from_bytes(data).ok()?;
        let tile = level.tile_at(&write_block_pos(level.jni, pos))?;
        let true = tile.is_instance_of(objs().tile_defs.tile.cls.cls.raw) else { return None };
        Some(Arc::new(EmitterMenu { tile: tile.new_weak_global_ref().unwrap(), dragged: false.into() }))
    }
}

impl Cleanable for EmitterMenu {
    fn free(self: Arc<Self>, jni: &crate::JNI) { Arc::into_inner(self).unwrap().tile.replace_jni(jni); }
}

const GRID_RADIUS: f32 = 56.;
fn grid_center(rect: &Rect) -> Point2<f32> { rect.center() + vector![0., 6.] }

impl Menu for EmitterMenu {
    fn any(&self) -> &dyn Any { self }
    fn still_valid(&self, player: BorrowedRef) -> bool {
        let Ok(tile) = self.tile.with_jni(player.jni).new_local_ref() else { return false };
        tile.still_valid(&player)
    }

    fn should_draw_dark_bg(&self) -> bool { false }
    fn get_size(&self) -> Vector2<i32> { vector![140, 140 + 12] }
    fn get_offset(&self) -> Vector2<i32> { vector![140, 0] }
    fn render_bg(&self, lk: &GlobalMtx, screen: BorrowedRef, gui: BorrowedRef, rect: Rect) {
        let Ok(tile) = self.tile.with_jni(gui.jni).new_local_ref() else { return };
        let Some(tile) = lk.try_read_tile::<Emitter>(tile.borrow()) else { return };
        let CommonData { polar, azimuth, dir } = *tile.common.borrow();
        let Some(dir) = dir else { return };
        let mut tess = Tessellator::new(gui.jni);
        tess.rect(rect, Rounding::same(4.), 0., vector![1., 1., 1., 0.5], &Stroke::new(1., vector![0., 0., 0., 1.]));
        let center = grid_center(&rect);
        let grid_stroke = Stroke::new(1., vector![0.25, 0.25, 0.25, 1.]);

        // Polar Grid
        for i in 1..=3 {
            let radius = (i * 30) as f32 * (GRID_RADIUS / 90.);
            tess.circle(center, radius, Vector4::zeros(), &grid_stroke);
        }

        // Azimuth Grid
        let mut div = vector![GRID_RADIUS, 0.];
        let mut step = Rotation2::new(FRAC_PI_6);
        for _ in 0..12 {
            tess.line([center, center + div], &grid_stroke);
            div = step * div
        }
        let pos = center + Rotation2::new(-azimuth) * vector![polar * (GRID_RADIUS / FRAC_PI_2), 0.];
        tess.circle(pos, 4., vector![1., 0., 0., 1.], &Stroke::new(1., vector![0., 0., 0., 1.]));
        gui.gui_draw_mesh(&mut tess.mesh);

        // Azimuth Label
        div = vector![GRID_RADIUS + 8., 0.];
        step = Rotation2::new(-FRAC_PI_2);
        let font = screen.screen_font();
        for dir in DIR_ADJS[dir as usize] {
            let text = [c"D", c"U", c"N", c"S", c"W", c"E"][dir as usize];
            let text = gui.jni.new_utf(text).unwrap().literal().to_formatted();
            let width = font.font_width(&text);
            let pos = center + div - vector![width as f32 * 0.5, 4.5];
            gui.gui_draw_formatted(&font, &text, pos.x as _, pos.y as _, 0, false);
            div = step * div
        }
    }

    fn mouse_clicked(&self, _lk: &GlobalMtx, menu: BorrowedRef, rect: Rect, pos: Point2<f32>, button: i32) -> bool {
        let handled = button == 0 && rect.contains(pos);
        if handled {
            self.dragged.set(true);
            self.send_attitude(menu, rect, pos);
            play_btn_click_sound(menu.jni);
        }
        handled
    }

    fn mouse_dragged(&self, _lk: &GlobalMtx, menu: BorrowedRef, rect: Rect, pos: Point2<f32>) -> bool {
        let handled = self.dragged.get();
        if handled {
            self.send_attitude(menu, rect, pos)
        }
        handled
    }

    fn mouse_released(&self, _lk: &GlobalMtx, button: i32) -> bool {
        if button == 0 {
            self.dragged.set(false)
        }
        false
    }
}

impl EmitterMenu {
    fn send_attitude(&self, menu: BorrowedRef, rect: Rect, pos: Point2<f32>) {
        let dir = pos - grid_center(&rect);
        let polar = dir.norm() * (FRAC_PI_2 / GRID_RADIUS);
        let azimuth = -libm::atan2f(dir.y, dir.x);
        let menu_id = menu.get_int_field(objs().mv.container_menu_id);
        objs().net_defs.send_c2s(menu.jni, &C2S::SetEmitterAttitude { menu_id, polar, azimuth });
    }
}
