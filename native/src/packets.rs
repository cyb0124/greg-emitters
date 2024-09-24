use crate::{
    beams::{set_beam_dir, ClientBeam},
    emitter_blocks::Emitter,
    emitter_gui::EmitterMenu,
    global::GlobalMtx,
    jvm::*,
    objs,
    util::{gui::GUIExt, strict_deserialize, tile::TileExt},
};
use anyhow::{ensure, Context, Result};
use core::{
    f32::consts::{FRAC_PI_2, TAU},
    num::NonZeroUsize,
};
use num_traits::Euclid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum EmitterAction {
    SetAttitude { zenith: f32, azimuth: f32 },
    SetDisableTransfer(bool),
}

#[derive(Serialize, Deserialize)]
pub struct C2S {
    pub menu_id: i32,
    pub action: EmitterAction,
}

#[derive(Serialize, Deserialize)]
pub enum S2C {
    SetBeam { id: NonZeroUsize, data: ClientBeam },
    DelBeam { id: NonZeroUsize },
}

pub fn handle_s2c(lk: &GlobalMtx, data: &[u8]) -> Result<()> {
    let data: S2C = strict_deserialize(data)?;
    Ok(match data {
        S2C::SetBeam { id, data } => {
            lk.client_state.borrow_mut().beams.insert(id, data);
        }
        S2C::DelBeam { id } => {
            lk.client_state.borrow_mut().beams.remove(&id);
        }
    })
}

pub fn handle_c2s(lk: &GlobalMtx, data: &[u8], player: BorrowedRef<'static, '_>) -> Result<()> {
    let gui_defs = &objs().gui_defs;
    let C2S { menu_id, action } = strict_deserialize(data)?;
    let menu = player.player_container_menu().context("no menu")?;
    ensure!(menu.menu_id() == menu_id);
    ensure!(menu.is_instance_of(gui_defs.menu.cls.cls.raw));
    let menu: &EmitterMenu = gui_defs.menu.read(lk, menu.borrow()).any().downcast_ref().context("wrong menu")?;
    let tile = menu.tile.with_jni(player.jni).new_local_ref()?;
    let level = tile.tile_level().context("dead tile")?;
    let emitter = lk.read_tile::<Emitter>(tile.borrow());
    let mut data = emitter.data.borrow_mut();
    match action {
        EmitterAction::SetAttitude { zenith, azimuth } => {
            data.zenith = if zenith.is_finite() { zenith.clamp(0., FRAC_PI_2) } else { 0. };
            data.azimuth = if azimuth.is_finite() { azimuth.rem_euclid(&TAU) } else { 0. };
            if let Some(beam_id) = emitter.beam_id.get() {
                set_beam_dir(lk, level.jni, beam_id, data.compute_dir())
            }
        }
        EmitterAction::SetDisableTransfer(x) => data.disable_transfer = x,
    }
    tile.tile_mark_for_save();
    Ok(level.level_mark_for_broadcast(&tile.tile_pos()))
}
