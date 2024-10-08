use super::{
    cleaner::Cleanable,
    client::SolidRenderer,
    geometry::GeomExt,
    mapping::{CN, MN},
    nbt::{new_compound, NBTExt, KEY_SAVE, KEY_SYNC},
    ClassBuilder, ClassNamer, FatWrapper,
};
use crate::{
    asm::*,
    global::{warn, GlobalMtx, GlobalObjs},
    jvm::*,
    mapping_base::*,
    objs,
};
use alloc::{format, sync::Arc, vec::Vec};
use anyhow::Result;
use core::any::Any;
use macros::dyn_abi;
use nalgebra::{Affine3, Point2};

impl<'a, T: JRef<'a>> TileExt<'a> for T {}
pub trait TileExt<'a>: JRef<'a> {
    fn tile_level(&self) -> Option<LocalRef<'a>> { self.get_object_field(objs().mv.tile_level) }
    fn tile_pos(&self) -> LocalRef<'a> { self.get_object_field(objs().mv.tile_pos).unwrap() }
    fn tile_mark_for_save(&self) { self.call_void_method(objs().mv.tile_set_changed, &[]).unwrap() }
    fn block_state_get_block(&self) -> LocalRef<'a> { self.call_object_method(objs().mv.block_state_get_block, &[]).unwrap().unwrap() }
    fn block_state_at(&self, pos: &impl JRef<'a>) -> LocalRef<'a> {
        self.call_object_method(objs().mv.block_getter_get_block_state, &[pos.raw()]).unwrap().unwrap()
    }

    fn loaded_chunk_at(&self, pos: Point2<i32>) -> Option<LocalRef<'a>> {
        self.call_object_method(objs().mv.chunk_source_get_chunk_now, &[pos.x as _, pos.y as _]).unwrap()
    }

    fn tile_at(&self, pos: &impl JRef<'a>) -> Option<LocalRef<'a>> { self.call_object_method(objs().mv.block_getter_get_tile, &[pos.raw()]).unwrap() }
    fn is_outside_build_height(&self, y: i32) -> bool { self.call_bool_method(objs().mv.level_is_outside_build_height, &[y as _]).unwrap() }
    fn level_is_client(&self) -> bool { self.get_bool_field(objs().mv.level_is_client) }
    fn level_get_chunk_source(&self) -> LocalRef<'a> { self.call_object_method(objs().mv.level_get_chunk_source, &[]).unwrap().unwrap() }
    fn level_mark_for_broadcast(&self, pos: &impl JRef<'a>) {
        self.level_get_chunk_source().call_void_method(objs().mv.server_chunk_cache_block_changed, &[pos.raw()]).unwrap()
    }
}

pub trait TileSupplier: Send {
    fn new_tile(&self, lk: &GlobalMtx, pos: BorrowedRef<'static, '_>, state: BorrowedRef<'static, '_>) -> Option<LocalRef<'static>>;
}

pub trait Tile: Cleanable {
    fn any(&self) -> &dyn Any;
    fn encode_save(&self) -> Vec<u8>;
    fn encode_sync(&self) -> Vec<u8>;
    fn decode_save(&self, bytes: &[u8]) -> Result<()>;
    fn decode_sync(&self, bytes: &[u8]) -> Result<()>;
    fn set_removed(&self, jni: &JNI, lk: &GlobalMtx);
    fn render(&self, lk: &GlobalMtx, sr: SolidRenderer, tf: Affine3<f32>);
}

pub struct TileDefs {
    pub tile: FatWrapper<dyn Tile>,
    tile_supplier: FatWrapper<dyn TileSupplier>,
}

impl TileDefs {
    pub fn init(av: &AV<'static>, cn: &CN<Arc<CSig>>, mn: &MN<MSig>, namer: &ClassNamer) -> Self {
        let jni = av.ldr.jni;
        let tile = ClassBuilder::new_1(av, namer, &cn.tile.slash)
            .native_2(&mn.tile_get_update_tag, get_update_tag_dyn())
            .native_2(&mn.tile_save_additional, save_additional_dyn())
            .native_2(&mn.tile_load_additional, load_additional_dyn())
            .native_2(&mn.tile_set_removed, set_removed_dyn())
            .insns(
                &mn.tile_get_update_pkt,
                [
                    av.new_var_insn(jni, OP_ALOAD, 0).unwrap(),
                    mn.s2c_tile_data_create.new_method_insn(av, jni, OP_INVOKESTATIC).unwrap(),
                    av.new_insn(jni, OP_ARETURN).unwrap(),
                ],
            )
            .define_fat()
            .wrap::<dyn Tile>();
        let tile_supplier = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([&*cn.tile_supplier.slash])
            .native_2(&mn.tile_supplier_create, tile_supplier_create_dyn())
            .define_fat()
            .wrap::<dyn TileSupplier>();
        Self { tile, tile_supplier }
    }

    pub fn new_tile_type<'a>(&self, jni: &'a JNI, supplier: &'static dyn TileSupplier, blocks: &impl JRef<'a>) -> GlobalRef<'a> {
        let supplier = self.tile_supplier.new_static(jni, supplier);
        let mv = &objs().mv;
        mv.tile_type.with_jni(jni).new_object(mv.tile_type_init, &[supplier.raw, blocks.raw(), 0]).unwrap().new_global_ref().unwrap()
    }

    pub fn new_tile<'a>(&self, jni: &'a JNI, tile_type: usize, pos: usize, state: usize, data: Arc<dyn Tile>) -> LocalRef<'a> {
        let tile = self.tile.new_obj(jni, data);
        tile.call_void_method(objs().mv.tile_init, &[tile_type, pos, state]).unwrap();
        tile
    }
}

impl GlobalMtx {
    pub fn read_tile<'a, T: Tile + 'static>(&'a self, obj: BorrowedRef<'_, 'a>) -> &'a T { self.try_read_tile(obj).unwrap() }
    pub fn try_read_tile<'a, T: Tile + 'static>(&'a self, obj: BorrowedRef<'_, 'a>) -> Option<&'a T> {
        objs().tile_defs.tile.read(self, obj).any().downcast_ref()
    }
}

#[dyn_abi]
fn tile_supplier_create(jni: &'static JNI, this: usize, pos: usize, state: usize) -> usize {
    let lk = objs().mtx.lock(jni).unwrap();
    let this = objs().tile_defs.tile_supplier.read(&lk, BorrowedRef::new(jni, &this));
    this.new_tile(&lk, BorrowedRef::new(jni, &pos), BorrowedRef::new(jni, &state)).map_or(0, |x| x.into_raw())
}

#[dyn_abi]
fn get_update_tag(jni: &JNI, tile: usize, _regs: usize) -> usize {
    let lk = objs().mtx.lock(jni).unwrap();
    let tile = objs().tile_defs.tile.read(&lk, BorrowedRef::new(jni, &tile));
    let tag = new_compound(jni);
    tag.compound_put_byte_array(KEY_SYNC, &tile.encode_sync());
    tag.into_raw()
}

#[dyn_abi]
fn save_additional(jni: &JNI, tile: usize, tag: usize, regs: usize) {
    let GlobalObjs { mv, tile_defs, mtx, .. } = objs();
    let tag = BorrowedRef::new(jni, &tag);
    let tile = BorrowedRef::new(jni, &tile);
    tile.call_nonvirtual_void_method(mv.tile.raw, mv.tile_save_additional, &[tag.raw, regs]).unwrap();
    tag.compound_put_byte_array(KEY_SAVE, &tile_defs.tile.read(&mtx.lock(jni).unwrap(), tile).encode_save());
}

#[dyn_abi]
fn load_additional(jni: &JNI, tile: usize, nbt: usize, regs: usize) {
    let GlobalObjs { mv, tile_defs, mtx, .. } = objs();
    let j_tile = BorrowedRef::new(jni, &tile);
    let tag = BorrowedRef::new(jni, &nbt);
    j_tile.call_nonvirtual_void_method(mv.tile.raw, mv.tile_load_additional, &[tag.raw, regs]).unwrap();
    let lk = mtx.lock(jni).unwrap();
    let tile = tile_defs.tile.read(&lk, j_tile);
    let mut data = tag.compound_get_byte_array(KEY_SYNC);
    let mut buf = data.crit_elems().unwrap();
    let result = (!buf.is_empty()).then(|| tile.decode_sync(&*buf));
    drop(buf);
    if let Some(Err(e)) = result {
        warn(jni, &cs(format!("Failed to load sync data for tile at {}: {e:?}", j_tile.tile_pos().read_vec3i())))
    }
    data = tag.compound_get_byte_array(KEY_SAVE);
    buf = data.crit_elems().unwrap();
    let result = (!buf.is_empty()).then(|| tile.decode_save(&*buf));
    drop(buf);
    if let Some(Err(e)) = result {
        warn(jni, &cs(format!("Failed to load save data for tile at {}: {e:?}", j_tile.tile_pos().read_vec3i())))
    }
}

#[dyn_abi]
fn set_removed(jni: &JNI, this: usize) {
    let GlobalObjs { mv, tile_defs, mtx, .. } = objs();
    let this = BorrowedRef::new(jni, &this);
    let lk = mtx.lock(jni).unwrap();
    tile_defs.tile.read(&lk, this).set_removed(jni, &lk);
    drop(lk);
    this.call_nonvirtual_void_method(mv.tile.raw, mv.tile_set_removed, &[]).unwrap();
}
