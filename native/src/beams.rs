use crate::{
    jvm::*,
    objs, ti,
    util::{
        geometry::{block_to_chunk, write_block_pos, CoveringBlocks},
        tile::TileExt,
    },
};
use alloc::sync::Arc;
use core::{cell::RefCell, mem::take};
use hashbrown::{hash_map::Entry, HashMap, HashSet, HashTable};
use nalgebra::{vector, Point2, Point3, Unit, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ClientBeam {
    tier: u8,
    src: Point3<i32>,
    dst: Point3<f32>,
}

#[derive(Default)]
pub struct ClientState {
    pub beams: HashMap<usize, ClientBeam>,
}

struct PlayerState {
    player: (GlobalRef<'static>, i32),
    level: (GlobalRef<'static>, i32),
    chunks: HashSet<Point2<i32>>,
}

/*
struct BeamState {
    tile: GlobalRef<'static>,
    chunks: HashSet<Point2<i32>>,
    src: Point3<i32>,
    dir: Unit<Vector3<f32>>,
    dst: Point3<f32>,
    dst_block: Point3<i32>,
}
*/

#[derive(Default)]
struct ChunkState {
    players: HashTable<(GlobalRef<'static>, i32)>,
}

struct DimState {
    level: (GlobalRef<'static>, i32),
    chunks: HashMap<Point2<i32>, ChunkState>,
}

#[derive(Default)]
pub struct ServerState {
    dims: HashTable<DimState>,
    players: HashTable<PlayerState>,
}

fn find_or_add_dim<'a>(table: &'a mut HashTable<DimState>, level: &impl JRef<'static>) -> &'a mut DimState {
    let hash = ti().id_hash(level.raw()).unwrap();
    (table.entry(hash as _, |x| level.is_same_object(x.level.0.raw), |x| x.level.1 as _))
        .or_insert_with(|| DimState { level: (level.new_global_ref().unwrap(), hash), chunks: HashMap::new() })
        .into_mut()
}

pub fn on_chunk_watch(player: &impl JRef<'static>, level: &impl JRef<'static>, pos: Point2<i32>) {
    let lk = objs().mtx.lock(level.jni()).unwrap();
    let mut srv = lk.server_state.borrow_mut();
    let p_hash = ti().id_hash(player.raw()).unwrap();
    let dim = find_or_add_dim(&mut srv.dims, level);
    (dim.chunks.entry(pos).or_default().players.entry(p_hash as _, |x| player.is_same_object(x.0.raw), |x| x.1 as _))
        .or_insert_with(|| (player.new_global_ref().unwrap(), p_hash));
    let l_hash = dim.level.1;
    let player = (srv.players.entry(p_hash as _, |x| player.is_same_object(x.player.0.raw), |x| x.player.1 as _)).or_insert_with(|| PlayerState {
        player: (player.new_global_ref().unwrap(), p_hash),
        level: (level.new_global_ref().unwrap(), l_hash),
        chunks: HashSet::new(),
    });
    player.into_mut().chunks.insert(pos);
}

pub fn on_chunk_unwatch<'a>(player: &impl JRef<'a>, pos: Point2<i32>) {
    let lk = objs().mtx.lock(player.jni()).unwrap();
    let mut srv = lk.server_state.borrow_mut();
    let p_hash = ti().id_hash(player.raw()).unwrap();
    let Ok(mut entry) = srv.players.find_entry(p_hash as _, |x| player.is_same_object(x.player.0.raw)) else { return };
    let state = entry.get_mut();
    let level = state.level.0.with_jni(player.jni()).new_local_ref().unwrap();
    let l_hash = state.level.1;
    state.chunks.remove(&pos);
    if state.chunks.is_empty() {
        let (state, _) = entry.remove();
        state.player.0.replace_jni(level.jni);
        state.level.0.replace_jni(level.jni);
    }
    let Ok(mut d_entry) = srv.dims.find_entry(l_hash as _, |x| level.is_same_object(x.level.0.raw)) else { return };
    let d_state = d_entry.get_mut();
    let Entry::Occupied(mut c_entry) = d_state.chunks.entry(pos) else { return };
    let c_state = c_entry.get_mut();
    let Ok(p_entry) = c_state.players.find_entry(p_hash as _, |x| player.is_same_object(x.0.raw)) else { return };
    p_entry.remove().0 .0.replace_jni(level.jni);
    if c_state.players.is_empty() {
        c_entry.remove();
    }
    if d_state.chunks.is_empty() {
        let (state, _) = d_entry.remove();
        state.level.0.replace_jni(level.jni);
    }
}

/*
fn update_beam(jni: &JNI, beam: &mut ServerBeam) {
    let level = beam.tile.with_jni(jni).tile_level().unwrap();
    let old_chunks = take(&mut beam.chunks);
    beam.chunks.insert(block_to_chunk(beam.src));
    let mut covering = CoveringBlocks::new(beam.src, vector![0.5, 0.5, 0.5], beam.dir);
    loop {
        covering.step();
        let pos = write_block_pos(jni, covering.pos);
        let true = level.level_is_loaded(&pos) else { break };
    }
    // TODO:
}
*/
