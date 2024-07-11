use crate::{
    emitter_blocks::{Emitter, EmitterData},
    global::GlobalMtx,
    jvm::*,
    mapping_base::MBOptExt,
    objs,
    packets::{EmitterAction, C2S},
    util::{
        cleaner::Cleanable,
        client::{play_btn_click_sound, ClientExt},
        geometry::{write_block_pos, Rect, DIR_ADJS, DIR_STEPS},
        gui::{GUIExt, Menu, MenuType},
        tessellator::{Rounding, Stroke, Tessellator},
        tile::TileExt,
    },
};
use alloc::sync::Arc;
use anyhow::{anyhow, ensure, Context, Result};
use core::{
    any::Any,
    cell::Cell,
    f32::consts::{FRAC_PI_2, FRAC_PI_6, PI},
};
use nalgebra::{point, vector, Matrix2, Point2, Point3, Rotation2, Vector2, Vector4};
use num_traits::Float;

#[derive(Clone, Copy)]
enum MouseState {
    Released,
    Dragging,
    Checkbox,
}

pub struct EmitterMenuType;
pub struct EmitterMenu {
    pub tile: WeakGlobalRef<'static>,
    mouse_state: Cell<MouseState>,
    view_tf: Matrix2<f32>,
}

impl MenuType for EmitterMenuType {
    fn raw(&self, lk: &GlobalMtx) -> usize { lk.emitter_blocks.get().unwrap().menu_type.raw }
    fn new_client(&self, lk: &GlobalMtx, level: BorrowedRef<'static, '_>, data: &[u8]) -> Result<Arc<dyn Menu>> {
        let pos: Point3<i32> = postcard::from_bytes(data).map_err(|e| anyhow!("{e}"))?;
        let tile = level.tile_at(&write_block_pos(level.jni, pos)).context("no tile")?;
        ensure!(tile.is_instance_of(objs().tile_defs.tile.cls.cls.raw));
        let dir = lk.try_read_tile::<Emitter>(tile.borrow()).context("wrong tile")?.data.borrow().dir;
        let mvc = objs().mv.client.uref();
        let y_rot = mvc.mc_inst.with_jni(level.jni).get_object_field(mvc.mc_player).unwrap().get_float_field(objs().mv.entity_y_rot) + 90.;
        let mut view_tf;
        if dir < 2 {
            let rot = ((y_rot / 15.).round() * 15.).to_radians();
            view_tf = Matrix2::from(Rotation2::new(if dir == 0 { PI + rot } else { -rot }));
        } else {
            let e = if dir == 2 { -1. } else { 1. };
            view_tf = Matrix2::new(0., e, e, 0.);
            let player_dir = Rotation2::new(y_rot.to_radians()) * vector![1., 0.];
            if DIR_STEPS[dir as usize].xz().cast().dot(&player_dir) < 0. {
                view_tf.row_mut(0).apply(|x| *x = -*x)
            }
        }
        Ok(Arc::new(EmitterMenu { tile: tile.new_weak_global_ref().unwrap(), mouse_state: MouseState::Released.into(), view_tf }))
    }
}

impl Cleanable for EmitterMenu {
    fn free(self: Arc<Self>, jni: &crate::JNI) { Arc::into_inner(self).unwrap().tile.replace_jni(jni); }
}

const GRID_RADIUS: f32 = 56.;
fn grid_center(rect: &Rect) -> Point2<f32> { rect.center() }
fn checkbox_rect(rect: &Rect) -> Rect { Rect::from_center_size(point![rect.min.x + 10., rect.max.y - 10.], vector![8., 8.]) }

impl Menu for EmitterMenu {
    fn any(&self) -> &dyn Any { self }
    fn still_valid(&self, player: BorrowedRef) -> bool {
        let Ok(tile) = self.tile.with_jni(player.jni).new_local_ref() else { return false };
        tile.still_valid(&player)
    }

    fn should_draw_dark_bg(&self) -> bool { false }
    fn get_size(&self) -> Vector2<i32> { vector![150, 150 + 20] }
    fn get_offset(&self) -> Vector2<i32> { vector![150, 0] }
    fn render_bg(&self, lk: &GlobalMtx, screen: BorrowedRef, gui: BorrowedRef, rect: Rect, cursor: Point2<i32>) {
        let Ok(tile) = self.tile.with_jni(gui.jni).new_local_ref() else { return };
        let Some(tile) = lk.try_read_tile::<Emitter>(tile.borrow()) else { return };
        let EmitterData { zenith, azimuth, dir, disable_transfer, .. } = *tile.data.borrow();
        let mut tess = Tessellator::new(gui.jni);
        tess.rect(rect, Rounding::same(4.), 0., vector![1., 1., 1., 0.5], &Stroke::new(1., vector![0., 0., 0., 1.]));
        let center = grid_center(&rect);
        let grid_stroke = Stroke::new(1., vector![0.25, 0.25, 0.25, 1.]);

        // Zenith Grid
        for i in 1..=3 {
            let radius = (i * 30) as f32 * (GRID_RADIUS / 90.);
            tess.circle(center, radius, Vector4::zeros(), &grid_stroke);
        }

        // Azimuth Grid
        let mut div = vector![GRID_RADIUS, 0.];
        let mut step = Rotation2::new(FRAC_PI_6);
        for _ in 0..12 {
            tess.line([center, center + self.view_tf * div], &grid_stroke);
            div = step * div
        }
        let pos = center + self.view_tf * Rotation2::new(-azimuth) * vector![zenith * (GRID_RADIUS / FRAC_PI_2), 0.];
        tess.circle(pos, 4., vector![1., 0., 0., 1.], &Stroke::new(1., vector![0., 0., 0., 1.]));

        // Checkbox
        let cb_rect = checkbox_rect(&rect);
        let cursor = cursor.cast::<f32>();
        let color = match self.mouse_state.get() {
            MouseState::Checkbox => vector![0.5, 0.5, 0.5, 0.5],
            MouseState::Released if rect.contains(cursor) && cursor.y >= cb_rect.min.y => vector![0.8, 0.8, 1., 0.5],
            _ => vector![1., 1., 1., 0.5],
        };
        tess.rect(cb_rect, Rounding::same(1.), 0., color, &Stroke::new(1., vector![0., 0., 0., 1.]));
        if !disable_transfer {
            let center = cb_rect.center();
            let pts = [center + vector![-2.7, 0.5], center + vector![-0.7, 2.7], center + vector![2.7, -2.7]];
            tess.path(&pts, false, Vector4::zeros(), &Stroke::new(1., vector![0., 0., 0., 1.]))
        }
        gui.gui_draw_mesh(&mut tess.mesh);

        // Azimuth Label
        div = vector![GRID_RADIUS + 8., 0.];
        step = Rotation2::new(-FRAC_PI_2);
        let font = screen.screen_font();
        for dir in DIR_ADJS[dir as usize] {
            let text = [c"D", c"U", c"N", c"S", c"W", c"E"][dir as usize];
            let text = gui.jni.new_utf(text).unwrap().literal().to_formatted();
            let width = font.font_width(&text);
            let pos = center + self.view_tf * div - vector![width as f32 * 0.5, 4.5];
            gui.gui_draw_formatted(&font, &text, pos.x as _, pos.y as _, 0, false);
            div = step * div
        }

        // Checkbox Label
        let text = gui.jni.new_utf(c"greg_emitters.transfer_energy").unwrap().translatable().to_formatted();
        gui.gui_draw_formatted(&font, &text, (cb_rect.max.x + 4.) as _, cb_rect.min.y as _, 0, false);
    }

    fn mouse_clicked(&self, _lk: &GlobalMtx, menu: BorrowedRef, rect: Rect, pos: Point2<f32>, button: i32) -> bool {
        let handled = button == 0 && rect.contains(pos);
        if handled {
            if pos.y >= checkbox_rect(&rect).min.y {
                self.mouse_state.set(MouseState::Checkbox)
            } else {
                self.mouse_state.set(MouseState::Dragging);
                self.send_attitude(menu, rect, pos)
            }
            play_btn_click_sound(menu.jni);
        }
        handled
    }

    fn mouse_dragged(&self, _lk: &GlobalMtx, menu: BorrowedRef, rect: Rect, pos: Point2<f32>) -> bool {
        match self.mouse_state.get() {
            MouseState::Released => return false,
            MouseState::Dragging => self.send_attitude(menu, rect, pos),
            MouseState::Checkbox => (),
        }
        true
    }

    fn mouse_released(&self, lk: &GlobalMtx, menu: BorrowedRef, button: i32) -> bool {
        let 0 = button else { return false };
        let MouseState::Checkbox = self.mouse_state.replace(MouseState::Released) else { return false };
        let Ok(tile) = self.tile.with_jni(menu.jni).new_local_ref() else { return false };
        let Some(tile) = lk.try_read_tile::<Emitter>(tile.borrow()) else { return false };
        let menu_id = menu.get_int_field(objs().mv.container_menu_id);
        let value = !tile.data.borrow().disable_transfer;
        objs().net_defs.send_c2s(menu.jni, &C2S { menu_id, action: EmitterAction::SetDisableTransfer(value) });
        false
    }
}

impl EmitterMenu {
    pub fn new_server(tile: WeakGlobalRef<'static>) -> Self { Self { tile, mouse_state: MouseState::Released.into(), view_tf: <_>::default() } }

    fn send_attitude(&self, menu: BorrowedRef, rect: Rect, pos: Point2<f32>) {
        let dir = self.view_tf.transpose() * (pos - grid_center(&rect));
        let zenith = dir.norm() * (FRAC_PI_2 / GRID_RADIUS);
        let azimuth = -libm::atan2f(dir.y, dir.x);
        let menu_id = menu.get_int_field(objs().mv.container_menu_id);
        objs().net_defs.send_c2s(menu.jni, &C2S { menu_id, action: EmitterAction::SetAttitude { zenith, azimuth } });
    }
}
