use crate::mapping_base::*;
use alloc::sync::Arc;
use mapping_macros::Functor;

#[derive(Functor)]
pub struct GregCN<T> {
    pub reg: T,
    pub item_builder: T,
    pub non_null_fn: T,
}

impl GregCN<Arc<CSig>> {
    pub fn new() -> Self {
        let names = GregCN::<&[u8]> {
            reg: b"com.gregtechceu.gtceu.api.registry.registrate.GTRegistrate",
            item_builder: b"com.tterrag.registrate.builders.ItemBuilder",
            non_null_fn: b"com.tterrag.registrate.util.nullness.NonNullFunction",
        };
        names.fmap(|x| Arc::new(CSig::new(x)))
    }
}

pub struct GregMN {
    pub reg_item: MSig,
}

impl GregMN {
    pub fn new(gcn: &GregCN<Arc<CSig>>) -> Self {
        GregMN {
            reg_item: MSig {
                owner: gcn.reg.clone(),
                name: cs("item"),
                sig: msig([b"Ljava/lang/String;", gcn.non_null_fn.sig.to_bytes()], gcn.item_builder.sig.to_bytes()),
            },
        }
    }
}
