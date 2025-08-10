use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(ShaderInput)]
pub fn shader_input_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let fields = if let Data::Struct(data_struct) = input.data {
        if let Fields::Named(fields_named) = data_struct.fields {
            fields_named.named.into_iter().map(|f| {
                let field_name = f.ident.unwrap().to_string();
                let ty = f.ty;
                quote! {
                    (#field_name, std::any::type_name::<#ty>())
                }
            }).collect::<Vec<_>>()
        } else {
            panic!("ShaderInput can only be derived for structs with named fields");
        }
    } else {
        panic!("ShaderInput can only be derived for structs");
    };

    let generated = quote! {
        impl ShaderInput for #name {
            fn reflect() -> Vec<(&'static str, &'static str)> {
                vec![#(#fields),*]
            }
        }
    };

    generated.into()
}

#[proc_macro_derive(AsAny)]
pub fn as_any_derive(input: TokenStream) -> TokenStream {
    let input  = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let generated = quote! {
        impl AsAny for #name {
            fn as_any(self: &Self) -> &dyn Any { self }
            fn as_any_mut(self: &mut Self) -> &mut dyn Any { self }
        }
    };

    generated.into()
}