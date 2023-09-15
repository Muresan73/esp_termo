use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(EspEvent)]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);
    let id_text = ident.to_string();
    let output = quote! {

    impl EspTypedEventSource for #ident {
        fn source() -> *const core::ffi::c_char {
            #id_text.as_bytes().as_ptr() as *const _
        }
    }

    impl EspTypedEventDeserializer<#ident> for #ident {
        fn deserialize<R>(data: &EspEventFetchData, f: &mut impl for<'a> FnMut(&'a #ident) -> R) -> R {
            f(unsafe { data.as_payload() })
        }
    }

    impl EspTypedEventSerializer<#ident> for #ident {
        fn serialize<R>(payload: &#ident, f: impl for<'a> FnOnce(&'a EspEventPostData) -> R) -> R {
            f(&unsafe { EspEventPostData::new(Self::source(), Self::event_id(), payload) })
        }
    }
        };
    output.into()
}
