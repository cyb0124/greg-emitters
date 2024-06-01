extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemFn};

#[proc_macro_attribute]
pub fn dyn_abi(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let name = &input.sig.ident;
    let ret = &input.sig.output;
    let arg_types = Vec::from_iter(input.sig.inputs.iter().map(|x| {
        let FnArg::Typed(x) = x else { panic!() };
        x.ty.clone()
    }));
    let arg_names = Vec::from_iter((0..arg_types.len()).map(|i| format_ident!("v{i}")));
    let arg_list = Vec::from_iter(arg_names.iter().zip(&arg_types).map(|(x, y)| quote!(#x : #y)));
    let name_a = format_ident!("{name}_a");
    let name_b = format_ident!("{name}_b");
    let name_dyn = format_ident!("{name}_dyn");
    quote! {
        #input
        #[cfg(target_arch = "x86_64")]
        fn #name_dyn() -> usize {
            extern "sysv64" fn #name_a(#(#arg_list,)*) #ret { #name(#(#arg_names,)*) }
            extern "win64" fn #name_b(#(#arg_list,)*) #ret { #name(#(#arg_names,)*) }
            if unsafe { crate::ENV.is_win } { #name_b as _ } else { #name_a as _ }
        }
        #[cfg(target_arch = "aarch64")]
        fn #name_dyn() -> usize {
            extern "C" fn #name_a(#(#arg_list,)*) #ret { #name(#(#arg_names,)*) }
            #name_a as _
        }
    }
    .into()
}
