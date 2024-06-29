use crate::{
    beams::ClientBeam,
    emitter_blocks::Emitter,
    emitter_gui::EmitterMenu,
    global::GlobalMtx,
    jvm::*,
    objs,
    util::{gui::GUIExt, tile::TileExt},
};
use anyhow::{anyhow, ensure, Context, Result};
use core::f32::consts::{FRAC_PI_2, TAU};
use num_traits::Euclid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum C2S {
    SetEmitterAttitude { menu_id: i32, polar: f32, azimuth: f32 },
}

#[derive(Serialize, Deserialize)]
pub enum S2C {
    SetBeam { id: usize, data: ClientBeam },
    DelBeam { id: usize },
}

pub fn handle_s2c(lk: &GlobalMtx, data: &[u8]) -> Result<()> {
    let data: S2C = postcard::from_bytes(data).map_err(|e| anyhow!("{e}"))?;
    Ok(match data {
        S2C::SetBeam { id, data } => {
            lk.client_state.borrow_mut().beams.insert(id, data);
        }
        S2C::DelBeam { id } => {
            lk.client_state.borrow_mut().beams.remove(&id);
        }
    })
}

pub fn handle_c2s(lk: &GlobalMtx, data: &[u8], player: BorrowedRef) -> Result<()> {
    let gui_defs = &objs().gui_defs;
    let data: C2S = postcard::from_bytes(data).map_err(|e| anyhow!("{e}"))?;
    Ok(match data {
        C2S::SetEmitterAttitude { menu_id, polar, azimuth } => {
            let menu = player.player_container_menu().context("no menu")?;
            ensure!(menu.menu_id() == menu_id);
            ensure!(menu.is_instance_of(gui_defs.menu.cls.cls.raw));
            let menu: &EmitterMenu = gui_defs.menu.read(lk, menu.borrow()).any().downcast_ref().context("wrong menu")?;
            let j_tile = menu.tile.with_jni(player.jni).new_local_ref()?;
            let level = j_tile.tile_level().context("dead tile")?;
            let tile = lk.read_tile::<Emitter>(j_tile.borrow());
            let mut common = tile.common.borrow_mut();
            common.polar = if polar.is_finite() { polar.clamp(0., FRAC_PI_2) } else { 0. };
            common.azimuth = if azimuth.is_finite() { azimuth.rem_euclid(&TAU) } else { 0. };
            drop(common);
            level.level_mark_for_broadcast(&j_tile.tile_pos())
        }
    })
}
