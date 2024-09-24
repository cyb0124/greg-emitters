use super::{
    cleaner::Cleanable,
    mapping::{ForgeCN, ForgeMN, CN, MN, MV},
    ClassBuilder, ClassNamer, ThinWrapper,
};
use crate::{
    asm::*,
    global::{warn, GlobalObjs},
    jvm::*,
    mapping_base::{cs, CSig, MSig},
    objs,
    packets::{handle_c2s, handle_s2c},
    registry::MOD_ID,
};
use alloc::{format, sync::Arc, vec::Vec};
use bstr::BStr;
use macros::dyn_abi;
use serde::Serialize;

pub struct NetworkDefs {
    pub payload_type: GlobalRef<'static>,
    payload: ThinWrapper<Payload>,
    empty_payload_array: GlobalRef<'static>,
    pub stream_codec: GlobalRef<'static>,
    pub payload_handler: GlobalRef<'static>,
}

struct Payload(Vec<u8>);
impl Cleanable for Payload {
    fn free(self: Arc<Self>, _: &JNI) {}
}

impl NetworkDefs {
    pub fn init(av: &AV<'static>, namer: &ClassNamer, cn: &CN<Arc<CSig>>, mn: &MN<MSig>, mv: &MV, fcn: &ForgeCN<Arc<CSig>>, fmn: &ForgeMN) -> Self {
        let ns = av.ldr.jni.new_utf(&cs(MOD_ID)).unwrap();
        let mut id = av.ldr.jni.new_utf(c"main").unwrap();
        id = mv.resource_loc.new_object(mv.resource_loc_init, &[ns.raw, id.raw]).unwrap();
        let payload_type = mv.custom_payload_type.new_object(mv.custom_payload_type_init, &[id.raw]).unwrap().new_global_ref().unwrap();

        let payload = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([&*cn.custom_payload.slash])
            .native_2(&mn.custom_payload_type, payload_type_dyn())
            .define_thin()
            .wrap::<Payload>();

        let stream_codec = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([&*cn.stream_codec.slash])
            .native_2(&mn.stream_codec_encode, encode_dyn())
            .native_2(&mn.stream_codec_decode, decode_dyn())
            .define_empty();

        let payload_handler = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([&*fcn.payload_handler.slash])
            .native_2(&fmn.handle_payload, handle_payload_dyn())
            .define_empty();

        Self {
            payload_type,
            empty_payload_array: payload.cls.cls.new_object_array(0, 0).unwrap().new_global_ref().unwrap(),
            payload,
            stream_codec: stream_codec.alloc_object().unwrap().new_global_ref().unwrap(),
            payload_handler: payload_handler.alloc_object().unwrap().new_global_ref().unwrap(),
        }
    }

    pub fn send_c2s(&self, jni: &JNI, data: &impl Serialize) {
        let fmv = &objs().fmv;
        let data = self.payload.new_obj(jni, Payload(postcard::to_allocvec(data).unwrap()).into());
        fmv.pkt_distributor.with_jni(jni).call_static_void_method(fmv.send_c2s, &[data.raw, self.empty_payload_array.raw]).unwrap()
    }

    pub fn send_s2c<'a>(&self, player: &impl JRef<'a>, data: &impl Serialize) {
        let fmv = &objs().fmv;
        let jni = player.jni();
        let data = self.payload.new_obj(jni, Payload(postcard::to_allocvec(data).unwrap()).into());
        fmv.pkt_distributor.with_jni(jni).call_static_void_method(fmv.send_s2c, &[player.raw(), data.raw, self.empty_payload_array.raw]).unwrap()
    }
}

#[dyn_abi]
fn payload_type(_: &JNI, _this: usize) -> usize { objs().net_defs.payload_type.raw }

#[dyn_abi]
fn encode(jni: &JNI, _: usize, buf: usize, msg: usize) {
    let lk = objs().mtx.lock(jni).unwrap();
    let data = &*objs().net_defs.payload.read(&*lk, BorrowedRef::new(jni, &msg)).0;
    let ba = jni.new_byte_array(data.len() as _).unwrap();
    ba.write_byte_array(data, 0).unwrap();
    drop(lk);
    BorrowedRef::new(jni, &buf).call_void_method(objs().mv.friendly_byte_buf_write_byte_array, &[ba.raw]).unwrap()
}

#[dyn_abi]
fn decode(jni: &JNI, _: usize, buf: usize) -> usize {
    match BorrowedRef::new(jni, &buf).call_object_method(objs().mv.friendly_byte_buf_read_byte_array, &[]).and_then(|x| x.expect_some()) {
        Ok(x) => objs().net_defs.payload.new_obj(jni, Payload(x.byte_elems().unwrap().to_vec()).into()).into_raw(),
        Err(JVMError::Throwable(e)) => {
            e.throw().unwrap();
            0
        }
        Err(e) => panic!("{e}"),
    }
}

#[dyn_abi]
fn handle_payload(jni: &'static JNI, _this: usize, payload: usize, ctx: usize) {
    let GlobalObjs { mtx, fmv, mv, net_defs, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let data = &*net_defs.payload.read(&*lk, BorrowedRef::new(jni, &payload)).0;
    let ctx = BorrowedRef::new(jni, &ctx);
    if ctx.call_object_method(fmv.payload_ctx_flow, &[]).unwrap().unwrap().call_bool_method(fmv.pkt_flow_is_s2c, &[]).unwrap() {
        if let Err(e) = handle_s2c(&*lk, data) {
            warn(jni, &cs(format!("Failed to handle packet from server: {e:?}")))
        }
    } else {
        let player = ctx.call_object_method(fmv.payload_ctx_player, &[]).unwrap().unwrap();
        if let Err(e) = handle_c2s(&lk, data, player.borrow()) {
            let profile = player.get_object_field(mv.player_profile).unwrap();
            let name = profile.call_object_method(mv.game_profile_get_name, &[]).unwrap().unwrap();
            warn(jni, &cs(format!("Failed to handle packet from player {}: {e:?}", BStr::new(&*name.utf_chars().unwrap()))))
        }
    }
}
