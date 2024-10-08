use crate::{
    global::{GlobalMtx, Tier},
    jvm::*,
    mapping_base::MBOptExt,
    objs,
    packets::S2C,
    ti,
    util::{
        geometry::{block_to_chunk, write_block_pos, write_vec3d, CoveringBlocks, GeomExt},
        tile::TileExt,
    },
};
use alloc::vec::Vec;
use core::{f32::consts::TAU, mem::take, num::NonZeroUsize};
use hashbrown::{hash_map, hash_set, hash_table, HashMap, HashSet, HashTable};
use nalgebra::{Point2, Point3, UnitVector3, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ClientBeam {
    tier: u8,
    active: bool,
    src: Point3<i32>,
    dst: Point3<f64>,
}

#[derive(Default)]
pub struct ClientState {
    pub beams: HashMap<NonZeroUsize, ClientBeam>,
}

pub struct PlayerState {
    player: (GlobalRef<'static>, i32),
    level: (GlobalRef<'static>, i32),
    chunks: HashSet<Point2<i32>>,
    beams: HashMap<NonZeroUsize, usize>,
}

pub struct BeamState {
    level: (GlobalRef<'static>, i32),
    players: HashTable<(GlobalRef<'static>, i32)>,
    chunks: HashSet<Point2<i32>>,
    blocks: Vec<Point3<i32>>,
    tier: u8,
    src: Point3<i32>,
    dir: UnitVector3<f32>,
    dst: Point3<f64>,
    pub hit: Option<(Point3<i32>, u8)>,
    pub active: bool,
    pub dirty: bool,
}

#[derive(Default)]
struct ChunkState {
    players: HashTable<(GlobalRef<'static>, i32)>,
    beams: HashSet<NonZeroUsize>,
}

pub enum TrackedBlock {
    ByOne(NonZeroUsize),
    ByMany(HashSet<NonZeroUsize>),
}

pub struct DimState {
    pub level: (GlobalRef<'static>, i32),
    chunks: HashMap<Point2<i32>, ChunkState>,
    pub blocks: HashMap<Point3<i32>, TrackedBlock>,
}

pub struct ServerState {
    pub dims: HashTable<DimState>,
    pub players: HashTable<PlayerState>,
    pub beams: HashMap<NonZeroUsize, BeamState>,
    next_beam_id: NonZeroUsize,
}

impl Default for ServerState {
    fn default() -> Self {
        Self { dims: HashTable::new(), players: HashTable::new(), beams: HashMap::new(), next_beam_id: NonZeroUsize::new(1).unwrap() }
    }
}

fn find_or_add_dim<'a>(table: &'a mut HashTable<DimState>, level: &impl JRef<'static>) -> &'a mut DimState {
    let hash = ti().id_hash(level.raw()).unwrap();
    (table.entry(hash as _, |x| level.is_same_object(x.level.0.raw), |x| x.level.1 as _))
        .or_insert_with(|| DimState { level: (level.new_global_ref().unwrap(), hash), chunks: HashMap::new(), blocks: HashMap::new() })
        .into_mut()
}

fn del_dim_if_empty(jni: &JNI, entry: hash_table::OccupiedEntry<DimState>) {
    if entry.get().chunks.is_empty() {
        // blocks should also be empty here.
        entry.remove().0.level.0.replace_jni(jni);
    }
}

fn del_chunk_if_empty(entry: hash_map::OccupiedEntry<Point2<i32>, ChunkState>) {
    let chunk = entry.get();
    if chunk.players.is_empty() && chunk.beams.is_empty() {
        entry.remove();
    }
}

impl PlayerState {
    #[must_use]
    fn incr_beam(&mut self, id: NonZeroUsize) -> bool {
        let count = self.beams.entry(id).or_default();
        *count += 1;
        *count == 1
    }

    #[must_use]
    fn decr_beam(&mut self, id: NonZeroUsize) -> bool {
        let hash_map::Entry::Occupied(mut entry) = self.beams.entry(id) else { unreachable!() };
        let count = entry.get_mut();
        *count -= 1;
        let del = *count == 0;
        if del {
            entry.remove();
        }
        del
    }
}

impl DimState {
    fn untrack_block(&mut self, pos: Point3<i32>, id: NonZeroUsize) {
        let hash_map::Entry::Occupied(mut entry) = self.blocks.entry(pos) else { unreachable!() };
        let block = entry.get_mut();
        match block {
            TrackedBlock::ByOne(_) => {
                entry.remove();
            }
            TrackedBlock::ByMany(beams) => {
                beams.remove(&id);
                if beams.len() == 1 {
                    *block = TrackedBlock::ByOne(*beams.iter().next().unwrap())
                }
            }
        }
    }

    fn track_block(&mut self, pos: Point3<i32>, id: NonZeroUsize) {
        match self.blocks.entry(pos) {
            hash_map::Entry::Vacant(entry) => {
                entry.insert(TrackedBlock::ByOne(id));
            }
            hash_map::Entry::Occupied(entry) => {
                let block = entry.into_mut();
                match block {
                    TrackedBlock::ByOne(old_id) => *block = TrackedBlock::ByMany([*old_id, id].into_iter().collect()),
                    TrackedBlock::ByMany(beams) => {
                        beams.insert(id);
                    }
                }
            }
        }
    }
}

impl BeamState {
    fn add_player(players: &mut HashTable<(GlobalRef<'static>, i32)>, player: &impl JRef<'static>, p_hash: i32) {
        players.insert_unique(p_hash as _, (player.new_global_ref().unwrap(), p_hash), |x| x.1 as _);
    }

    fn del_player<'a>(players: &mut HashTable<(GlobalRef<'static>, i32)>, player: &impl JRef<'a>, p_hash: i32) {
        players.find_entry(p_hash as _, |x| player.is_same_object(x.0.raw)).ok().unwrap().remove().0 .0.replace_jni(player.jni());
    }

    fn send_del_beam<'a>(id: NonZeroUsize, player: &impl JRef<'a>) { objs().net_defs.send_s2c(player, &S2C::DelBeam { id }) }
    fn send_set_beam<'a>(&self, id: NonZeroUsize, player: &impl JRef<'a>) {
        let data = ClientBeam { tier: self.tier, active: self.active, src: self.src, dst: self.dst };
        objs().net_defs.send_s2c(player, &S2C::SetBeam { id, data })
    }

    pub fn broadcast_set_beam(&self, jni: &JNI, id: NonZeroUsize) {
        for player in &self.players {
            self.send_set_beam(id, &player.0.with_jni(jni))
        }
    }

    // Will send DelBeam to players that can no longer see the beam, but not SetBeam.
    pub fn recompute(&mut self, jni: &'static JNI, players: &mut HashTable<PlayerState>, dim: &mut DimState, id: NonZeroUsize) {
        let mv = &objs().mv;
        let old_chunks = take(&mut self.chunks);
        for pos in self.blocks.drain(..) {
            dim.untrack_block(pos, id)
        }
        self.chunks.insert(block_to_chunk(self.src));
        let mut covering = CoveringBlocks::new(self.src, Vector3::from_element(0.5), self.dir.cast());
        let j_src = write_vec3d(jni, covering.pos.cast::<f64>().map(|x| x + 0.5));
        let level = self.level.0.with_jni(jni);
        let chunk_source = level.level_get_chunk_source();
        loop {
            covering.step();
            let chunk = block_to_chunk(covering.pos);
            self.chunks.insert(chunk);
            self.blocks.push(covering.pos);
            dim.track_block(covering.pos, id);
            let Some(Some(chunk)) = (!level.is_outside_build_height(covering.pos.y)).then(|| chunk_source.loaded_chunk_at(chunk)) else {
                self.dst = covering.pos.cast::<f64>() + covering.frac;
                self.hit = None;
                break;
            };
            let pos = write_block_pos(jni, covering.pos);
            let state = chunk.block_state_at(&pos);
            let args = [chunk.raw, pos.raw, mv.collision_ctx_empty.raw];
            let shape = state.call_object_method(mv.block_state_get_visual_shape, &args).unwrap().unwrap();
            let j_dst = write_vec3d(jni, (covering.pos.cast::<f64>() + covering.frac + *covering.dir * 2.).cast());
            if let Some(hit) = shape.call_object_method(mv.voxel_shape_clip, &[j_src.raw, j_dst.raw, pos.raw]).unwrap() {
                if !hit.get_bool_field(mv.block_hit_result_miss) {
                    self.dst = hit.get_object_field(mv.block_hit_result_pos).unwrap().read_vec3d().cast();
                    self.hit = Some((covering.pos, hit.get_object_field(mv.block_hit_result_dir).unwrap().read_dir()));
                    break;
                }
            }
        }
        for &pos in self.chunks.difference(&old_chunks) {
            let c_state = dim.chunks.entry(pos).or_default();
            c_state.beams.insert(id);
            for &(ref player, p_hash) in &c_state.players {
                let player = player.with_jni(jni);
                if players.find_mut(p_hash as _, |x| player.is_same_object(x.player.0.raw)).unwrap().incr_beam(id) {
                    Self::add_player(&mut self.players, &player, p_hash);
                }
            }
        }
        for &pos in old_chunks.difference(&self.chunks) {
            let hash_map::Entry::Occupied(mut c_entry) = dim.chunks.entry(pos) else { unreachable!() };
            let c_state = c_entry.get_mut();
            for &(ref player, p_hash) in &c_state.players {
                let player = player.with_jni(jni);
                if players.find_mut(p_hash as _, |x| player.is_same_object(x.player.0.raw)).unwrap().decr_beam(id) {
                    Self::del_player(&mut self.players, &player, p_hash);
                    Self::send_del_beam(id, &player)
                }
            }
            c_state.beams.remove(&id);
            del_chunk_if_empty(c_entry)
        }
        self.dirty = false
    }
}

pub fn on_chunk_watch(player: &impl JRef<'static>, level: &impl JRef<'static>, pos: Point2<i32>) {
    let lk = objs().mtx.lock(level.jni()).unwrap();
    let mut srv = lk.server_state.borrow_mut();
    let srv = &mut *srv;
    let p_hash = ti().id_hash(player.raw()).unwrap();
    let dim = find_or_add_dim(&mut srv.dims, level);
    let chunk = dim.chunks.entry(pos).or_default();
    let hash_table::Entry::Vacant(p_entry) = chunk.players.entry(p_hash as _, |x| player.is_same_object(x.0.raw), |x| x.1 as _) else { return };
    p_entry.insert((player.new_global_ref().unwrap(), p_hash));
    let l_hash = dim.level.1;
    let p_entry = (srv.players.entry(p_hash as _, |x| player.is_same_object(x.player.0.raw), |x| x.player.1 as _)).or_insert_with(|| PlayerState {
        player: (player.new_global_ref().unwrap(), p_hash),
        level: (level.new_global_ref().unwrap(), l_hash),
        chunks: HashSet::new(),
        beams: HashMap::new(),
    });
    let p_state = p_entry.into_mut();
    if p_state.chunks.insert(pos) {
        for &id in &chunk.beams {
            if p_state.incr_beam(id) {
                let beam = srv.beams.get_mut(&id).unwrap();
                BeamState::add_player(&mut beam.players, player, p_hash);
                beam.send_set_beam(id, player)
            }
        }
    }
}

pub fn on_chunk_unwatch<'a>(player: &impl JRef<'a>, pos: Point2<i32>) {
    let lk = objs().mtx.lock(player.jni()).unwrap();
    let mut srv = lk.server_state.borrow_mut();
    let srv = &mut *srv;
    let p_hash = ti().id_hash(player.raw()).unwrap();
    let Ok(mut p_entry) = srv.players.find_entry(p_hash as _, |x| player.is_same_object(x.player.0.raw)) else { return };
    let p_state = p_entry.get_mut();
    let hash_set::Entry::Occupied(c_entry) = p_state.chunks.entry(pos) else { return };
    c_entry.remove();
    let level = p_state.level.0.with_jni(player.jni()).new_local_ref().unwrap();
    let l_hash = p_state.level.1;
    let mut d_entry = srv.dims.find_entry(l_hash as _, |x| level.is_same_object(x.level.0.raw)).ok().unwrap();
    let hash_map::Entry::Occupied(mut c_entry) = d_entry.get_mut().chunks.entry(pos) else { unreachable!() };
    let c_state = c_entry.get_mut();
    for &id in &c_state.beams {
        if p_state.decr_beam(id) {
            BeamState::del_player(&mut srv.beams.get_mut(&id).unwrap().players, player, p_hash);
            BeamState::send_del_beam(id, player)
        }
    }
    c_state.players.find_entry(p_hash as _, |x| player.is_same_object(x.0.raw)).ok().unwrap().remove().0 .0.replace_jni(level.jni);
    del_chunk_if_empty(c_entry);
    del_dim_if_empty(level.jni, d_entry);
    if p_state.chunks.is_empty() {
        // beams should also be empty here.
        let (state, _) = p_entry.remove();
        state.player.0.replace_jni(level.jni);
        state.level.0.replace_jni(level.jni);
    }
}

pub fn on_chunk_load_or_unload(level: &impl JRef<'static>, pos: Point2<i32>) {
    let lk = objs().mtx.lock(level.jni()).unwrap();
    let mut srv = lk.server_state.borrow_mut();
    let srv = &mut *srv;
    let Some(dim) = srv.dims.find(ti().id_hash(level.raw()).unwrap() as _, |x| level.is_same_object(x.level.0.raw)) else { return };
    let Some(chunk) = dim.chunks.get(&pos) else { return };
    for &id in &chunk.beams {
        srv.beams.get_mut(&id).unwrap().dirty = true
    }
}

pub fn del_beam(jni: &JNI, lk: &GlobalMtx, id: NonZeroUsize) {
    let mut srv = lk.server_state.borrow_mut();
    let srv = &mut *srv;
    let beam = srv.beams.remove(&id).unwrap();
    for (player, p_hash) in beam.players {
        let player = player.replace_jni(jni);
        srv.players.find_mut(p_hash as _, |x| player.is_same_object(x.player.0.raw)).unwrap().beams.remove(&id);
        BeamState::send_del_beam(id, &player)
    }
    let level = beam.level.0.replace_jni(jni);
    let Ok(mut d_entry) = srv.dims.find_entry(beam.level.1 as _, |x| level.is_same_object(x.level.0.raw)) else { unreachable!() };
    for pos in beam.chunks {
        let hash_map::Entry::Occupied(mut c_entry) = d_entry.get_mut().chunks.entry(pos) else { unreachable!() };
        c_entry.get_mut().beams.remove(&id);
        del_chunk_if_empty(c_entry)
    }
    for pos in beam.blocks {
        d_entry.get_mut().untrack_block(pos, id)
    }
    del_dim_if_empty(jni, d_entry)
}

pub fn add_beam(lk: &GlobalMtx, level: &impl JRef<'static>, tier: u8, src: Point3<i32>, dir: UnitVector3<f32>) -> NonZeroUsize {
    let mut srv = lk.server_state.borrow_mut();
    let srv = &mut *srv;
    let dim = find_or_add_dim(&mut srv.dims, level);
    let id = srv.next_beam_id;
    srv.next_beam_id = srv.next_beam_id.checked_add(1).unwrap();
    let hash_map::Entry::Vacant(entry) = srv.beams.entry(id) else { unreachable!() };
    let beam = entry.insert(BeamState {
        level: (level.new_global_ref().unwrap(), dim.level.1),
        players: HashTable::new(),
        chunks: HashSet::new(),
        blocks: Vec::new(),
        tier,
        src,
        dir,
        dst: <_>::default(),
        hit: None,
        active: false,
        dirty: true,
    });
    beam.recompute(level.jni(), &mut srv.players, dim, id);
    beam.broadcast_set_beam(level.jni(), id);
    id
}

pub fn set_beam_dir(lk: &GlobalMtx, jni: &'static JNI, id: NonZeroUsize, dir: UnitVector3<f32>) {
    let mut srv = lk.server_state.borrow_mut();
    let srv = &mut *srv;
    let beam = srv.beams.get_mut(&id).unwrap();
    let level = beam.level.0.with_jni(jni);
    let dim = srv.dims.find_mut(beam.level.1 as _, |x| level.is_same_object(x.level.0.raw)).unwrap();
    beam.dir = dir;
    beam.recompute(jni, &mut srv.players, dim, id);
    beam.broadcast_set_beam(jni, id)
}

impl ClientBeam {
    pub fn render<'a>(&self, tiers: &[Tier], vb: &impl JRef<'a>, pose: &impl JRef<'a>, camera_pos: Point3<f64>, tick: i32, sub_tick: f32) {
        // TODO: frustum culling
        let mvc = objs().mv.client.uref();
        let src = (self.src.cast::<f64>().map(|x| x + 0.5) - camera_pos).cast::<f32>();
        let dst = (self.dst - camera_pos).cast::<f32>();
        let dir = (dst - src).normalize();
        let mut b = Vector3::zeros();
        b[dir.abs().argmin().0] = 1.;
        let radius = if self.active { 0.2 } else { 0.1 };
        let mut n = b.cross(&dir) * radius;
        let mut b = n.cross(&dir);
        if self.active {
            const PERIOD: i32 = 10;
            let (s, c) = libm::sincosf(((tick % PERIOD) as f32 + sub_tick) * (TAU / PERIOD as f32));
            (n, b) = (c * n - s * b, s * n + c * b)
        }
        let pts = [src + n, dst + n, src + b, dst + b, src - n, dst - n, src - b, dst - b];
        let color = tiers[self.tier as usize].color;
        for i in [0, 1, 3, 2, 2, 3, 5, 4, 4, 5, 7, 6, 6, 7, 1, 0] {
            let p = pts[i];
            vb.call_object_method(mvc.vertex_consumer_pos, &[pose.raw(), f_raw(p.x), f_raw(p.y), f_raw(p.z)]).unwrap();
            vb.call_object_method(mvc.vertex_consumer_color, &[f_raw(color.x), f_raw(color.y), f_raw(color.z), f_raw(1.)]).unwrap();
        }
    }
}
