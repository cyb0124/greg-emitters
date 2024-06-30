use super::{
    cleaner::Cleanable,
    client::SolidRenderer,
    mapping::{ForgeMN, CN, MN},
    nbt::{new_compound, NBTExt, KEY_COMMON, KEY_SERVER},
    ClassBuilder, ClassNamer, FatWrapper,
};
use crate::{
    asm::*,
    global::{GlobalMtx, GlobalObjs},
    jvm::*,
    mapping_base::{cs, CSig, MSig},
    objs,
};
use alloc::{format, sync::Arc, vec::Vec};
use core::any::Any;
use macros::dyn_abi;
use nalgebra::{Affine3, Point2};
use postcard::{de_flavors::Slice, Deserializer, Result};

impl<'a, T: JRef<'a>> TileExt<'a> for T {}
pub trait TileExt<'a>: JRef<'a> {
    fn lazy_opt_invalidate(&self) { self.call_void_method(objs().fmv.lazy_opt_invalidate, &[]).unwrap() }
    fn lazy_opt_of(&self) -> LocalRef<'a> {
        let fmv = &objs().fmv;
        fmv.lazy_opt.with_jni(self.jni()).call_static_object_method(fmv.lazy_opt_of, &[self.raw()]).unwrap().unwrap()
    }

    fn tile_level(&self) -> Option<LocalRef<'a>> { self.get_object_field(objs().mv.tile_level) }
    fn tile_pos(&self) -> LocalRef<'a> { self.get_object_field(objs().mv.tile_pos).unwrap() }
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
    fn save_common(&self) -> Vec<u8>;
    fn save_server(&self) -> Vec<u8>;
    fn load_common<'a>(&self, de: &mut Deserializer<'a, Slice<'a>>) -> Result<()>;
    fn load_server<'a>(&self, de: &mut Deserializer<'a, Slice<'a>>) -> Result<()>;
    fn get_cap(&self, cap: BorrowedRef) -> Option<usize>;
    fn invalidate_caps(&self, jni: &JNI, lk: &GlobalMtx);
    fn render(&self, lk: &GlobalMtx, sr: SolidRenderer, tf: Affine3<f32>);
}

pub struct TileDefs {
    pub tile: FatWrapper<dyn Tile>,
    tile_supplier: FatWrapper<dyn TileSupplier>,
}

impl TileDefs {
    pub fn init(av: &AV<'static>, cn: &CN<Arc<CSig>>, mn: &MN<MSig>, fmn: &ForgeMN, namer: &ClassNamer) -> Self {
        let jni = av.ldr.jni;
        let tile = ClassBuilder::new_1(av, namer, &cn.tile.slash)
            .native_2(&mn.tile_get_update_tag, get_update_tag_dyn())
            .native_2(&mn.tile_save_additional, save_additional_dyn())
            .native_2(&mn.tile_load, on_load_dyn())
            .native_2(&fmn.get_cap, get_cap_dyn())
            .native_2(&fmn.invalidate_caps, invalidate_caps_dyn())
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
fn get_update_tag(jni: &JNI, tile: usize) -> usize {
    let lk = objs().mtx.lock(jni).unwrap();
    let tile = objs().tile_defs.tile.read(&lk, BorrowedRef::new(jni, &tile));
    let tag = new_compound(jni);
    tag.compound_put_byte_array(KEY_COMMON, &tile.save_common());
    tag.into_raw()
}

#[dyn_abi]
fn save_additional(jni: &JNI, tile: usize, tag: usize) {
    let GlobalObjs { mv, tile_defs, mtx, .. } = objs();
    let tag = BorrowedRef::new(jni, &tag);
    let tile = BorrowedRef::new(jni, &tile);
    tile.call_nonvirtual_void_method(mv.tile.raw, mv.tile_save_additional, &[tag.raw]).unwrap();
    let lk = mtx.lock(jni).unwrap();
    let tile = tile_defs.tile.read(&lk, tile);
    tag.compound_put_byte_array(KEY_COMMON, &tile.save_common());
    tag.compound_put_byte_array(KEY_SERVER, &tile.save_server());
}

#[dyn_abi]
fn on_load(jni: &JNI, tile: usize, nbt: usize) {
    let GlobalObjs { av, mv, tile_defs, mtx, .. } = objs();
    let tile = BorrowedRef::new(jni, &tile);
    let tag = BorrowedRef::new(jni, &nbt);
    tile.call_nonvirtual_void_method(mv.tile.raw, mv.tile_load, &[tag.raw]).unwrap();
    let lk = mtx.lock(jni).unwrap();
    let tile = tile_defs.tile.read(&lk, tile);
    let result: Result<()> = try {
        let mut data = tag.compound_get_byte_array(KEY_COMMON);
        let mut buf = data.crit_elems().unwrap();
        if !buf.is_empty() {
            tile.load_common(&mut Deserializer::from_bytes(&*buf))?
        }
        drop(buf);
        data = tag.compound_get_byte_array(KEY_SERVER);
        buf = data.crit_elems().unwrap();
        if !buf.is_empty() {
            tile.load_server(&mut Deserializer::from_bytes(&*buf))?
        }
    };
    let Err(e) = result else { return };
    av.jv.runtime_exception.with_jni(jni).throw_new(&cs(format!("{e}"))).unwrap()
}

#[dyn_abi]
fn get_cap(jni: &JNI, this: usize, cap: usize, side: usize) -> usize {
    let GlobalObjs { fmv, tile_defs, mtx, .. } = objs();
    let this = BorrowedRef::new(jni, &this);
    if let Some(cap) = tile_defs.tile.read(&mtx.lock(jni).unwrap(), this).get_cap(BorrowedRef::new(jni, &cap)) {
        cap
    } else {
        this.call_nonvirtual_object_method(fmv.cap_provider.raw, fmv.get_cap, &[cap, side]).unwrap().unwrap().into_raw()
    }
}

#[dyn_abi]
fn invalidate_caps(jni: &JNI, this: usize) {
    let GlobalObjs { fmv, tile_defs, mtx, .. } = objs();
    let this = BorrowedRef::new(jni, &this);
    this.call_nonvirtual_void_method(fmv.cap_provider.raw, fmv.invalidate_caps, &[]).unwrap();
    let lk = mtx.lock(jni).unwrap();
    tile_defs.tile.read(&lk, this).invalidate_caps(jni, &lk)
}
