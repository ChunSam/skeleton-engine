use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Type};

/// Returns `true` if the field has `#[reflect(skip)]`.
fn has_reflect_skip(field: &syn::Field) -> syn::Result<bool> {
    for attr in &field.attrs {
        if !attr.path().is_ident("reflect") {
            continue;
        }
        let mut found_skip = false;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                found_skip = true;
                Ok(())
            } else {
                Err(meta.error("unknown reflect attribute; expected `skip`"))
            }
        })?;
        if found_skip {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Attempt to map a field type to a `ReflectValue` variant.
/// Returns `None` when the type is unrecognised (caller should emit a compile error).
fn map_type(ty: &Type) -> Option<FieldKind> {
    match ty {
        Type::Path(tp) => {
            let last = tp.path.segments.last()?;
            let ident = last.ident.to_string();
            match ident.as_str() {
                "f32" => Some(FieldKind::F32),
                "i32" => Some(FieldKind::I32),
                "Vec2" => Some(FieldKind::Vec2),
                "bool" => Some(FieldKind::Bool),
                "String" => Some(FieldKind::String),
                "Color" => Some(FieldKind::Color),
                _ => None,
            }
        }
        // [f32; 4]
        Type::Array(arr) => {
            if let Type::Path(tp) = arr.elem.as_ref() {
                if let Some(seg) = tp.path.segments.last() {
                    if seg.ident == "f32" {
                        // check length == 4 via the Expr
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Int(ref n),
                            ..
                        }) = arr.len
                        {
                            if n.base10_parse::<usize>().ok() == Some(4) {
                                return Some(FieldKind::Array4);
                            }
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum FieldKind {
    F32,
    I32,
    Vec2,
    Bool,
    String,
    Color,
    Array4,
}

impl FieldKind {
    fn fields_expr(self, field_name: &syn::Ident) -> TokenStream {
        match self {
            FieldKind::F32 => quote!(engine::reflect::ReflectValue::F32(self.#field_name)),
            FieldKind::I32 => quote!(engine::reflect::ReflectValue::I32(self.#field_name)),
            FieldKind::Vec2 => quote!(engine::reflect::ReflectValue::Vec2(self.#field_name)),
            FieldKind::Bool => quote!(engine::reflect::ReflectValue::Bool(self.#field_name)),
            FieldKind::String => {
                quote!(engine::reflect::ReflectValue::String(self.#field_name.clone()))
            }
            FieldKind::Color => {
                quote!(engine::reflect::ReflectValue::Color(self.#field_name.to_array()))
            }
            FieldKind::Array4 => quote!(engine::reflect::ReflectValue::Color(self.#field_name)),
        }
    }

    fn set_arm(self, field_name: &syn::Ident, name_str: &str) -> TokenStream {
        match self {
            FieldKind::F32 => quote! {
                #name_str => {
                    if let engine::reflect::ReflectValue::F32(v) = val {
                        self.#field_name = v;
                        return true;
                    }
                }
            },
            FieldKind::I32 => quote! {
                #name_str => {
                    if let engine::reflect::ReflectValue::I32(v) = val {
                        self.#field_name = v;
                        return true;
                    }
                }
            },
            FieldKind::Vec2 => quote! {
                #name_str => {
                    if let engine::reflect::ReflectValue::Vec2(v) = val {
                        self.#field_name = v;
                        return true;
                    }
                }
            },
            FieldKind::Bool => quote! {
                #name_str => {
                    if let engine::reflect::ReflectValue::Bool(v) = val {
                        self.#field_name = v;
                        return true;
                    }
                }
            },
            FieldKind::String => quote! {
                #name_str => {
                    if let engine::reflect::ReflectValue::String(v) = val {
                        self.#field_name = v;
                        return true;
                    }
                }
            },
            FieldKind::Color => quote! {
                #name_str => {
                    if let engine::reflect::ReflectValue::Color(v) = val {
                        self.#field_name = engine::color::Color::from(v);
                        return true;
                    }
                }
            },
            FieldKind::Array4 => quote! {
                #name_str => {
                    if let engine::reflect::ReflectValue::Color(v) = val {
                        self.#field_name = v;
                        return true;
                    }
                }
            },
        }
    }
}

/// Core expansion: called from the proc-macro entry point with the parsed `DeriveInput`.
pub fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let name_str = name.to_string();

    // Only named-field structs are supported.
    let fields =
        match &input.data {
            Data::Struct(ds) => match &ds.fields {
                Fields::Named(f) => &f.named,
                Fields::Unnamed(_) => return Err(Error::new_spanned(
                    name,
                    "#[derive(Reflect)] does not support tuple structs; use a named-field struct",
                )),
                Fields::Unit => return Err(Error::new_spanned(
                    name,
                    "#[derive(Reflect)] does not support unit structs; use a named-field struct",
                )),
            },
            Data::Enum(_) => {
                return Err(Error::new_spanned(
                    name,
                    "#[derive(Reflect)] does not support enums; use a named-field struct",
                ))
            }
            Data::Union(_) => {
                return Err(Error::new_spanned(
                    name,
                    "#[derive(Reflect)] does not support unions; use a named-field struct",
                ))
            }
        };

    let mut fields_arms: Vec<TokenStream> = Vec::new();
    let mut set_arms: Vec<TokenStream> = Vec::new();

    for field in fields {
        let skip = has_reflect_skip(field)?;
        if skip {
            continue;
        }

        let field_ident = field
            .ident
            .as_ref()
            .expect("named field has ident; checked above");
        let field_name_str = field_ident.to_string();

        match map_type(&field.ty) {
            Some(kind) => {
                let val_expr = kind.fields_expr(field_ident);
                fields_arms.push(quote!((#field_name_str, #val_expr)));
                set_arms.push(kind.set_arm(field_ident, &field_name_str));
            }
            None => {
                // Unsupported type — emit a compile error pointing at the field.
                let msg = format!(
                    "field `{}`: type not supported by #[derive(Reflect)]; \
                     add #[reflect(skip)] to omit it",
                    field_name_str
                );
                return Err(Error::new_spanned(&field.ty, msg));
            }
        }
    }

    let expanded = quote! {
        impl engine::reflect::Reflect for #name {
            fn fields(&self) -> ::std::vec::Vec<(&'static str, engine::reflect::ReflectValue)> {
                vec![#(#fields_arms),*]
            }

            fn set_field(&mut self, name: &str, val: engine::reflect::ReflectValue) -> bool {
                match name {
                    #(#set_arms)*
                    _ => {}
                }
                false
            }

            fn type_name(&self) -> &'static str {
                #name_str
            }
        }
    };

    Ok(expanded)
}
