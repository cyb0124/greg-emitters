use super::{
    cleaner::Cleanable,
    mapping::{ForgeMV, MV},
    serialize_to_byte_array, ClassBuilder, ClassNamer, ThinWrapper,
};
use crate::{
    asm::*,
    global::{warn, GlobalObjs},
    jvm::*,
    mapping_base::cs,
    objs,
    packets::{handle_c2s, handle_s2c},
    registry::{MOD_ID, PROTOCOL_VERSION},
};
use alloc::{format, sync::Arc};
use bstr::BStr;
use macros::dyn_abi;
use serde::Serialize;

pub struct NetworkDefs {
    channel: GlobalRef<'static>,
    task: ThinWrapper<Task>,
}

struct Task {
    msg: GlobalRef<'static>,
    ctx: GlobalRef<'static>,
}

impl Cleanable for Task {
    fn free(self: Arc<Self>, jni: &JNI) {
        let Task { msg, ctx } = Arc::into_inner(self).unwrap();
        msg.replace_jni(jni);
        ctx.replace_jni(jni);
    }
}

impl NetworkDefs {
    pub fn init(av: &AV<'static>, namer: &ClassNamer, mv: &MV, fmv: &ForgeMV) -> Self {
        let ns = av.ldr.jni.new_utf(&cs(MOD_ID)).unwrap();
        let mut id = av.ldr.jni.new_utf(c"main").unwrap();
        id = mv.resource_loc.new_object(mv.resource_loc_init, &[ns.raw, id.raw]).unwrap();
        let version_supplier = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([
                c"java/util/function/Supplier",
                c"java/util/function/Predicate",
                c"java/util/function/BiConsumer",
                c"java/util/function/Function",
            ])
            .native_1(c"get", c"()Ljava/lang/Object;", get_version_dyn())
            .native_1(c"test", c"(Ljava/lang/Object;)Z", check_version_dyn())
            .native_1(c"accept", c"(Ljava/lang/Object;Ljava/lang/Object;)V", encode_msg_dyn())
            .native_1(c"apply", c"(Ljava/lang/Object;)Ljava/lang/Object;", decode_msg_dyn())
            .define_empty();
        let v_inst = version_supplier.alloc_object().unwrap();
        let args = [id.raw, v_inst.raw, v_inst.raw, v_inst.raw];
        let channel = fmv.network_reg.call_static_object_method(fmv.network_reg_new_simple_channel, &args);
        let channel = channel.unwrap().unwrap().new_global_ref().unwrap();
        let packet_handler = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([c"java/util/function/BiConsumer"])
            .native_1(c"accept", c"(Ljava/lang/Object;Ljava/lang/Object;)V", handle_msg_dyn())
            .define_empty();
        let h_inst = packet_handler.alloc_object().unwrap();
        let msg_type = av.ldr.jni.new_byte_array(0).unwrap().get_object_class();
        channel.call_object_method(fmv.simple_channel_reg_msg, &[0, msg_type.raw, v_inst.raw, v_inst.raw, h_inst.raw]).unwrap();
        let task = ClassBuilder::new_1(av, namer, c"java/lang/Object")
            .interfaces([c"java/lang/Runnable"])
            .native_1(c"run", c"()V", task_run_dyn())
            .define_thin()
            .wrap::<Task>();
        Self { channel, task }
    }

    pub fn send_c2s(&self, jni: &JNI, data: &impl Serialize) {
        self.channel.with_jni(jni).call_void_method(objs().fmv.simple_channel_send_to_server, &[serialize_to_byte_array(jni, data).raw]).unwrap()
    }

    pub fn send_s2c<'a>(&self, player: &impl JRef<'a>, data: &impl Serialize) {
        let GlobalObjs { mv, fmv, .. } = objs();
        let conn = player.get_object_field(mv.server_player_pkt_listener).unwrap().get_object_field(mv.server_pkt_listener_impl_conn).unwrap();
        let ba = serialize_to_byte_array(conn.jni, data);
        self.channel.with_jni(conn.jni).call_void_method(fmv.simple_channel_send_to, &[ba.raw, conn.raw, fmv.network_dir_s2c.raw]).unwrap()
    }
}

#[dyn_abi]
fn get_version(jni: &JNI, _this: usize) -> usize { jni.new_utf(PROTOCOL_VERSION).unwrap().into_raw() }

#[dyn_abi]
fn check_version(jni: &JNI, _this: usize, version: usize) -> bool { jni.new_utf(PROTOCOL_VERSION).unwrap().equals(&objs().av.jv, version).unwrap() }

#[dyn_abi]
fn encode_msg(jni: &JNI, _: usize, msg: usize, buf: usize) {
    BorrowedRef::new(jni, &buf).call_void_method(objs().mv.friendly_byte_buf_write_byte_array, &[msg]).unwrap()
}

#[dyn_abi]
fn decode_msg(jni: &JNI, _: usize, buf: usize) -> usize {
    match BorrowedRef::new(jni, &buf).call_object_method(objs().mv.friendly_byte_buf_read_byte_array, &[]).and_then(|x| x.expect_some()) {
        Ok(x) => x.into_raw(),
        Err(JVMError::Throwable(e)) => {
            e.throw().unwrap();
            0
        }
        Err(e) => panic!("{e}"),
    }
}

#[dyn_abi]
fn task_run(jni: &'static JNI, this: usize) {
    let GlobalObjs { mtx, fmv, mv, net_defs, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let task = net_defs.task.read(&lk, BorrowedRef::new(jni, &this));
    let msg = task.msg.with_jni(jni);
    let ctx = task.ctx.with_jni(jni);
    let msg = msg.byte_elems().unwrap();
    let dir = ctx.call_object_method(fmv.network_ctx_get_dir, &[]).unwrap().unwrap();
    if dir.is_same_object(fmv.network_dir_c2s.raw) {
        let Some(player) = ctx.call_object_method(fmv.network_ctx_get_sender, &[]).unwrap() else { return };
        if let Err(e) = handle_c2s(&lk, &*msg, player.borrow()) {
            let profile = player.get_object_field(mv.player_profile).unwrap();
            let name = profile.call_object_method(mv.game_profile_get_name, &[]).unwrap().unwrap();
            warn(jni, &cs(format!("Failed to handle packet from player {}: {e:?}", BStr::new(&*name.utf_chars().unwrap()))))
        }
    } else if dir.is_same_object(fmv.network_dir_s2c.raw) {
        if let Err(e) = handle_s2c(&lk, &*msg) {
            warn(jni, &cs(format!("Failed to handle packet from server: {e:?}")))
        }
    }
}

#[dyn_abi]
fn handle_msg(jni: &'static JNI, _this: usize, msg: usize, ctx_supplier: usize) {
    let GlobalObjs { av, fmv, net_defs, .. } = objs();
    let ctx = BorrowedRef::new(jni, &ctx_supplier).supplier_get(&av.jv).unwrap().unwrap();
    ctx.call_void_method(fmv.network_ctx_set_handled, &[1]).unwrap();
    let task = Task { msg: BorrowedRef::new(jni, &msg).new_global_ref().unwrap(), ctx: ctx.new_global_ref().unwrap() };
    let task = net_defs.task.new_obj(jni, Arc::new(task));
    ctx.call_object_method(fmv.network_ctx_enqueue_task, &[task.raw]).unwrap();
}
