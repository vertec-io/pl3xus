use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Meta, Expr, ExprLit, Lit};

// =============================================================================
// Invalidates Derive Macro
// =============================================================================

/// Derive macro for automatic query invalidation.
///
/// Use with the `#[invalidates(...)]` attribute to specify which queries
/// should be invalidated when this mutation succeeds.
///
/// # Examples
///
/// ```rust,ignore
/// // Single invalidation
/// #[derive(Invalidates)]
/// #[invalidates("ListPrograms")]
/// pub struct CreateProgram { ... }
///
/// // Multiple invalidations
/// #[derive(Invalidates)]
/// #[invalidates("ListPrograms", "GetProgram")]
/// pub struct DeleteProgram { ... }
/// ```
///
/// This generates an implementation of the `Invalidates` trait:
///
/// ```rust,ignore
/// impl Invalidates for CreateProgram {
///     fn invalidates() -> &'static [&'static str] {
///         &["ListPrograms"]
///     }
/// }
/// ```
#[proc_macro_derive(Invalidates, attributes(invalidates))]
pub fn derive_invalidates(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    // Find the #[invalidates(...)] attribute
    let mut query_types: Vec<String> = Vec::new();

    for attr in &ast.attrs {
        if attr.path().is_ident("invalidates") {
            // Parse the attribute arguments
            if let Meta::List(meta_list) = &attr.meta {
                // Parse as comma-separated string literals
                let result = meta_list.parse_args_with(
                    syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated,
                );

                if let Ok(exprs) = result {
                    for expr in exprs {
                        if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = expr {
                            query_types.push(lit_str.value());
                        }
                    }
                }
            }
        }
    }

    // Generate the trait implementation
    let expanded = quote! {
        impl pl3xus_sync::Invalidates for #name {
            fn invalidates() -> &'static [&'static str] {
                &[#(#query_types),*]
            }
        }
    };

    expanded.into()
}

// =============================================================================
// SubscribeById Derive Macro
// =============================================================================

#[proc_macro_derive(SubscribeById, attributes(subscribe_id))]
pub fn derive_subscribe_by_id(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    // Generate the struct names
    let subscribe_struct_name = quote::format_ident!("SubscribeTo{}", name);
    let unsubscribe_struct_name = quote::format_ident!("UnsubscribeFrom{}", name);

    // Get the subscribe_id field and its type, if any
    let subscribe_id_field = find_subscribe_id_field(&ast.data);

    // Generate the Subscribe and Unsubscribe message structs
    let subscribe_struct = match &subscribe_id_field {
        Some((field_name, _field_type)) => quote! {
            #[derive(Serialize, Deserialize, Debug)]
            pub struct #subscribe_struct_name {
                pub #field_name: String,
            }
        },
        None => quote! {
            #[derive(Serialize, Deserialize, Debug)]
            pub struct #subscribe_struct_name;
        },
    };

    let unsubscribe_struct = match &subscribe_id_field {
        Some((field_name, _field_type)) => quote! {
            #[derive(Serialize, Deserialize, Debug)]
            pub struct #unsubscribe_struct_name {
                pub #field_name: String,
            }
        },
        None => quote! {
            #[derive(Serialize, Deserialize, Debug)]
            pub struct #unsubscribe_struct_name;
        },
    };

    let subscription_impl = match &subscribe_id_field {
        Some((field_name, _field_type)) => quote! {
            impl SubscriptionMessage for #name {
                type SubscribeRequest = #subscribe_struct_name;
                type UnsubscribeRequest = #unsubscribe_struct_name;
                type SubscriptionParams = String;

                fn get_subscription_params(&self) -> Self::SubscriptionParams {
                    self.#field_name.to_string()
                }

                fn create_subscription_request(params: Self::SubscriptionParams) -> Self::SubscribeRequest {
                    #subscribe_struct_name { #field_name: params }
                }

                fn create_unsubscribe_request(params: Self::SubscriptionParams) -> Self::UnsubscribeRequest {
                    #unsubscribe_struct_name { #field_name: params }
                }
            }
        },
        None => quote! {
            impl SubscriptionMessage for #name {
                type SubscribeRequest = #subscribe_struct_name;
                type UnsubscribeRequest = #unsubscribe_struct_name;
                type SubscriptionParams = String;

                fn get_subscription_params(&self) -> Self::SubscriptionParams {
                    // Use type_name() for types without a subscribe_id field
                    // This works for both NetworkMessage (with explicit NAME) and Pl3xusMessage types
                    use std::any::type_name;
                    type_name::<Self>().to_string()
                }

                fn create_subscription_request(_params: Self::SubscriptionParams) -> Self::SubscribeRequest {
                    #subscribe_struct_name
                }

                fn create_unsubscribe_request(_params: Self::SubscriptionParams) -> Self::UnsubscribeRequest {
                    #unsubscribe_struct_name
                }
            }
        },
    };

    quote! {
        #subscribe_struct
        #unsubscribe_struct

        #subscription_impl
    }
    .into()
}

fn find_subscribe_id_field(data: &Data) -> Option<(syn::Ident, syn::Type)> {
    match data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .find(|field| {
                    field
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident("subscribe_id"))
                })
                .map(|field| (field.ident.clone().unwrap(), field.ty.clone())),
            _ => None,
        },
        _ => None,
    }
}

// =============================================================================
// HasSuccess Derive Macro
// =============================================================================

/// Derive macro for response types with a `success: bool` field.
///
/// This automatically implements the `HasSuccess` trait by reading the
/// `success` field from the struct.
///
/// # Requirements
///
/// The struct must have a field named `success` of type `bool`.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(HasSuccess)]
/// pub struct CreateProgramResponse {
///     pub success: bool,
///     pub program_id: Option<i64>,
///     pub error: Option<String>,
/// }
/// ```
///
/// This generates:
///
/// ```rust,ignore
/// impl pl3xus_common::HasSuccess for CreateProgramResponse {
///     fn is_success(&self) -> bool {
///         self.success
///     }
/// }
/// ```
#[proc_macro_derive(HasSuccess)]
pub fn derive_has_success(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    // Verify the struct has a `success` field
    let has_success_field = match &ast.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .any(|field| field.ident.as_ref().map(|i| i == "success").unwrap_or(false)),
            _ => false,
        },
        _ => false,
    };

    if !has_success_field {
        return syn::Error::new_spanned(
            &ast.ident,
            "HasSuccess derive requires a struct with a `success: bool` field",
        )
        .to_compile_error()
        .into();
    }

    // Generate the trait implementation
    let expanded = quote! {
        impl pl3xus_common::HasSuccess for #name {
            fn is_success(&self) -> bool {
                self.success
            }
        }
    };

    expanded.into()
}
