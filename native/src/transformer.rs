use crate::{
    asm::{ASMRefExt, ACC_NATIVE, ACC_PUBLIC, ACC_STATIC, OP_ALOAD, OP_ASTORE, OP_DUP, OP_IFNULL, OP_INVOKESTATIC, OP_POP},
    global::GlobalObjs,
    jvm::*,
    mapping::Mapping,
    mapping_base::*,
    objs,
    registry::{self, MOD_ID},
};
use alloc::{sync::Arc, vec::Vec};
use macros::dyn_abi;
use mapping_macros::Functor;

#[derive(Functor)]
struct TfCN<T> {
    srv: T,
    tf: T,
    env: T,
    target: T,
    vote: T,
    vote_ctx: T,
}

impl TfCN<Arc<CSig>> {
    fn new() -> Self {
        let names = TfCN::<&[u8]> {
            srv: b"cpw.mods.modlauncher.api.ITransformationService",
            tf: b"cpw.mods.modlauncher.api.ITransformer",
            env: b"cpw.mods.modlauncher.api.IEnvironment",
            target: b"cpw.mods.modlauncher.api.ITransformer$Target",
            vote: b"cpw.mods.modlauncher.api.TransformerVoteResult",
            vote_ctx: b"cpw.mods.modlauncher.api.ITransformerVotingContext",
        };
        names.fmap(|x| Arc::new(CSig::new(x)))
    }
}

struct TfMN {
    srv_name: MSig,
    srv_on_load: MSig,
    srv_initialize: MSig,
    srv_tfs: MSig,
    tf_targets: MSig,
    tf_cast_vote: MSig,
    tf_transform: MSig,
    target_of_cls: MSig,
    vote_yes: MSig,
}

impl TfMN {
    fn new(tcn: &TfCN<Arc<CSig>>) -> Self {
        TfMN {
            srv_name: MSig { owner: tcn.srv.clone(), name: cs("name"), sig: cs("()Ljava/lang/String;") },
            srv_on_load: MSig { owner: tcn.srv.clone(), name: cs("onLoad"), sig: msig([tcn.env.sig.to_bytes(), b"Ljava/util/Set;"], b"V") },
            srv_initialize: MSig { owner: tcn.srv.clone(), name: cs("initialize"), sig: msig([tcn.env.sig.to_bytes()], b"V") },
            srv_tfs: MSig { owner: tcn.srv.clone(), name: cs("transformers"), sig: cs("()Ljava/util/List;") },
            tf_targets: MSig { owner: tcn.tf.clone(), name: cs("targets"), sig: cs("()Ljava/util/Set;") },
            tf_cast_vote: MSig { owner: tcn.tf.clone(), name: cs("castVote"), sig: msig([tcn.vote_ctx.sig.to_bytes()], tcn.vote.sig.to_bytes()) },
            tf_transform: MSig {
                owner: tcn.tf.clone(),
                name: cs("transform"),
                sig: msig([b"Ljava/lang/Object;", tcn.vote_ctx.sig.to_bytes()], b"Ljava/lang/Object;"),
            },
            target_of_cls: MSig {
                owner: tcn.target.clone(),
                name: cs("targetClass"),
                sig: msig([b"Ljava/lang/String;".as_slice()], tcn.target.sig.to_bytes()),
            },
            vote_yes: MSig { owner: tcn.vote.clone(), name: cs("YES"), sig: tcn.vote.sig.clone() },
        }
    }
}

pub struct TfDefs {
    tfs: GlobalRef<'static>,
    targets: GlobalRef<'static>,
    vote_yes: GlobalRef<'static>,
    reg_item_stub: MSig,
}

impl TfDefs {
    pub fn free(self, jni: &JNI) {
        let TfDefs { tfs, targets, vote_yes, reg_item_stub: _ } = self;
        tfs.replace_jni(jni);
        targets.replace_jni(jni);
        vote_yes.replace_jni(jni);
    }
}

pub fn init(srv_cls: &impl JRef<'static>) {
    // Names
    let tcn = TfCN::new();
    let tmn = TfMN::new(&tcn);

    // Targets
    let jni = srv_cls.jni();
    let GlobalObjs { av, namer, mtx, gcn, .. } = objs();
    let target = av.ldr.with_jni(jni).load_class(&av.jv, &tcn.target.dot).unwrap();
    let of_cls = tmn.target_of_cls.get_static_method_id(&target).unwrap();
    let gt_reg = target.call_static_object_method(of_cls, &[jni.new_utf(&gcn.reg.slash).unwrap().raw]).unwrap().unwrap();
    let targets = gt_reg.singleton(&av.jv).unwrap().new_global_ref().unwrap();

    // Transformers & Stubs
    let name = namer.next();
    let mut cls = av.new_class_node(jni, &name.slash, c"java/lang/Object").unwrap();
    cls.add_interfaces(av, [&*tcn.tf.slash]).unwrap();
    let gsig = [b"Ljava/lang/Object;L", tcn.tf.slash.to_bytes(), b"<Lorg/objectweb/asm/tree/ClassNode;>;"];
    cls.class_set_gsig(av, &cs(Vec::from_iter(gsig.into_iter().flatten().copied()))).unwrap();
    let reg_item_stub = MSig { owner: name.clone(), name: cs("0"), sig: msig([b"Ljava/lang/String;".as_slice()], gcn.non_null_fn.sig.to_bytes()) };
    let methods = [
        tmn.tf_targets.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        tmn.tf_cast_vote.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        tmn.tf_transform.new_method_node(av, jni, ACC_PUBLIC | ACC_NATIVE).unwrap(),
        reg_item_stub.new_method_node(av, jni, ACC_PUBLIC | ACC_STATIC | ACC_NATIVE).unwrap(),
    ];
    let natives = [
        tmn.tf_targets.native(tf_targets_dyn()),
        tmn.tf_cast_vote.native(tf_cast_vote_dyn()),
        tmn.tf_transform.native(tf_transform_dyn()),
        reg_item_stub.native(reg_item_stub_dyn()),
    ];
    cls.class_methods(av).unwrap().collection_extend(&av.jv, methods).unwrap();
    cls = av.ldr.with_jni(jni).define_class(&name.slash, &*cls.write_class_simple(av).unwrap().byte_elems().unwrap()).unwrap();
    cls.register_natives(&natives).unwrap();
    let tfs = cls.new_object_array(1, cls.alloc_object().unwrap().raw).unwrap().array_as_list(&av.jv).unwrap().new_global_ref().unwrap();

    // Service
    let natives = [
        tmn.srv_name.native(srv_name_dyn()),
        tmn.srv_on_load.native(srv_on_load_dyn()),
        tmn.srv_initialize.native(srv_initialize_dyn()),
        tmn.srv_tfs.native(srv_tfs_dyn()),
        NativeMethod { name: c"modInit".as_ptr(), sig: c"()V".as_ptr(), func: mod_init_dyn() },
    ];
    srv_cls.register_natives(&natives).unwrap();

    // Misc
    let vote = av.ldr.with_jni(jni).load_class(&av.jv, &tcn.vote.dot).unwrap();
    let vote_yes = vote.get_static_object_field(tmn.vote_yes.get_static_field_id(&vote).unwrap()).unwrap().new_global_ref().unwrap();
    mtx.lock(jni).unwrap().tf = Some(TfDefs { tfs, targets, vote_yes, reg_item_stub });
}

#[dyn_abi]
fn srv_name(jni: &JNI, _this: usize) -> usize { jni.new_utf(&cs(MOD_ID)).unwrap().into_raw() }

#[dyn_abi]
fn srv_on_load(_: &JNI, _this: usize, _env: usize, _other_services: usize) {}

#[dyn_abi]
fn srv_initialize(_: &JNI, _this: usize, _env: usize) {}

#[dyn_abi]
fn srv_tfs(jni: &JNI, _this: usize) -> usize { objs().mtx.lock(jni).unwrap().tf.uref().tfs.raw }

#[dyn_abi]
fn tf_targets(jni: &JNI, _this: usize) -> usize { objs().mtx.lock(jni).unwrap().tf.uref().targets.raw }

#[dyn_abi]
fn tf_cast_vote(jni: &JNI, _this: usize, _vote_ctx: usize) -> usize { objs().mtx.lock(jni).unwrap().tf.uref().vote_yes.raw }

#[dyn_abi]
fn tf_transform(jni: &JNI, _this: usize, cls: usize, _vote_ctx: usize) -> usize {
    let cls = BorrowedRef::new(jni, &cls);
    let GlobalObjs { av, mtx, gcn, gmn, .. } = objs();
    let lk = mtx.lock(jni).unwrap();
    let defs = lk.tf.uref();
    let name = cls.get_object_field(av.class_node_name).unwrap();
    let name = name.utf_chars().unwrap();
    if &*name == gcn.reg.slash.to_bytes() {
        let mut found = false;
        for method in cls.class_methods_iter(av).unwrap() {
            let method = method.unwrap().unwrap();
            if gmn.reg_item.matches_node(av, &method).unwrap() {
                let skip = av.new_label(jni).unwrap();
                let stub = [
                    av.new_var_insn(jni, OP_ALOAD, 1).unwrap(),
                    defs.reg_item_stub.new_method_insn(av, jni, OP_INVOKESTATIC).unwrap(),
                    av.new_insn(jni, OP_DUP).unwrap(),
                    av.new_jump_insn(OP_IFNULL, &skip).unwrap(),
                    av.new_insn(jni, OP_DUP).unwrap(),
                    av.new_var_insn(jni, OP_ASTORE, 2).unwrap(),
                    skip,
                    av.new_insn(jni, OP_POP).unwrap(),
                ];
                method.method_insns(av).unwrap().prepend_insns(av, stub).unwrap();
                found = true;
                break;
            }
        }
        assert!(found)
    }
    cls.raw
}

#[dyn_abi]
fn mod_init(jni: &'static JNI, _: usize) {
    let mut lk = objs().mtx.lock(jni).unwrap();
    lk.tf.take().unwrap().free(jni);
    lk.m = Some(Mapping::new(jni));
    registry::init(&*lk)
}

#[dyn_abi]
fn reg_item_stub(jni: &'static JNI, _: usize, name: usize) -> usize {
    let name = BorrowedRef::new(jni, &name);
    let name = name.utf_chars().unwrap();
    let true = name.ends_with(b"_emitter") else { return 0 };

    let GlobalObjs { mtx, .. } = objs();
    let mut lk = mtx.lock(jni).unwrap();

    0
}
