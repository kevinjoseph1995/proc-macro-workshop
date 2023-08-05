#[derive(Clone, PartialEq)]
enum HasFieldAttr {
    None,
    FieldAttr(String /*"each" field name */),
}

#[derive(Clone)]
struct FieldMetadata {
    is_optional: bool,
    has_builder_attr: HasFieldAttr,
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);

    let struct_data = match &ast.data {
        syn::Data::Struct(data_struct) => data_struct.clone(),
        syn::Data::Enum(_) => panic!("Cannot derive Builder from Enum type"),
        syn::Data::Union(_) => panic!("Cannot derive Builder from union type"),
    };
    let struct_name = ast.ident.clone();
    let builder_struct_ident = quote::format_ident!("{}Builder", struct_name.clone());

    let field_metadata_list: Vec<FieldMetadata> = struct_data
        .fields
        .clone()
        .into_iter()
        .map(|field| -> FieldMetadata {
            let is_optional = match field.ty {
                syn::Type::Path(type_path) => {
                    // This is not an exhaustive list, we could also represent our optional type from the standard library as
                    // std::option::Option
                    type_path.path.segments.first().unwrap().ident == "Option"
                }
                _ => false,
            };
            let has_builder_attr;
            if !field.attrs.is_empty() {
                assert!(field.attrs.len() == 1);
                if let syn::Meta::List(metalist) = &field.attrs[0].meta {
                    let assing_exp: syn::ExprAssign = syn::parse2(metalist.tokens.clone()).unwrap();
                    let each_field_name;
                    match *assing_exp.right {
                        syn::Expr::Lit(expr_lit) => {
                            if let syn::Lit::Str(string_liter) = expr_lit.lit {
                                each_field_name = string_liter.token().to_string();
                            } else {
                                panic!("TODO Better Error message");
                            }
                        }
                        _ => panic!("TODO Better Error message"),
                    };
                    has_builder_attr = HasFieldAttr::FieldAttr(each_field_name);
                } else {
                    panic!("TODO Better Error message");
                }
            } else {
                has_builder_attr = HasFieldAttr::None;
            }

            FieldMetadata {
                is_optional,
                has_builder_attr,
            }
        })
        .collect();

    let extract_inner_type_from_outer_type =
        |type_t: syn::Type, outer_type: &str| -> Option<syn::Type> {
            if let syn::Type::Path(type_path) = type_t {
                assert!(type_path.path.segments.first().unwrap().ident == outer_type);
                if let syn::PathArguments::AngleBracketed(args) =
                    &type_path.path.segments.first().unwrap().arguments
                {
                    if let syn::GenericArgument::Type(inner_type) = args.args.first().unwrap() {
                        return Some(inner_type.clone());
                    }
                }
            }
            None
        };

    let builder_declaraions_token_stream = field_metadata_list
        .clone()
        .into_iter()
        .zip(struct_data.fields.clone().into_iter())
        .map(|(field_metadata, field)| {
            let field_ident = field.ident.clone().unwrap();
            let field_type = {
                if field_metadata.is_optional {
                    match extract_inner_type_from_outer_type(field.ty, "Option") {
                        Some(inner_type) => inner_type,
                        None => panic!("Failed to extract inner type from Option"),
                    }
                } else if field_metadata.has_builder_attr != HasFieldAttr::None {
                    match extract_inner_type_from_outer_type(field.ty, "Vec") {
                        Some(inner_type) => inner_type,
                        None => panic!("Failed to extract inner type from Vec"),
                    }
                } else {
                    field.ty
                }
            };
            if field_metadata.has_builder_attr != HasFieldAttr::None {
                quote::quote! {
                    #field_ident : std::vec::Vec<#field_type>
                }
            } else {
                quote::quote! {
                    #field_ident : std::option::Option<#field_type>
                }
            }
        });

    let builder_methods = field_metadata_list
        .clone()
        .into_iter()
        .zip(struct_data.fields.clone().into_iter())
        .map(|(field_metadata, field)| {
            let field_iden = &field.ident;
            let field_type = {
                if field_metadata.is_optional {
                    match extract_inner_type_from_outer_type(field.ty.clone(), "Option") {
                        Some(inner_type) => inner_type,
                        None => panic!("Failed to extract inner type from Option"),
                    }
                } else {
                    field.ty.clone()
                }
            };
            if let HasFieldAttr::FieldAttr(each_field_name) = field_metadata.has_builder_attr {
                let builder_method =
                    quote::format_ident!("{}", each_field_name[1..each_field_name.len() - 1]);
                let inner_type = match extract_inner_type_from_outer_type(field.ty.clone(), "Vec") {
                    Some(inner_type) => inner_type,
                    None => panic!("Failed to extract inner type from Vec"),
                };
                quote::quote! {
                    pub fn #builder_method(&mut self, #builder_method : #inner_type) -> &mut Self {
                        self.#field_iden.push(#builder_method);
                        self
                    }
                }
            } else {
                quote::quote! {
                    pub fn #field_iden(&mut self, #field_iden : #field_type) -> &mut Self {
                        self.#field_iden = Some(#field_iden);
                        self
                    }
                }
            }
        });

    let field_idents = field_metadata_list
        .clone()
        .into_iter()
        .zip(struct_data.fields.clone().into_iter())
        .map(|(field_metadata, field)| {
            let field_iden = field.clone().ident.unwrap();
            if field_metadata.is_optional {
                quote::quote! {
                     #field_iden : self.#field_iden.clone()
                }
            } else if field_metadata.has_builder_attr != HasFieldAttr::None {
                quote::quote! {
                    #field_iden : self.#field_iden.clone()
                }
            } else {
                quote::quote! {
                    #field_iden : self.#field_iden.clone().ok_or(stringify!(#field_iden not found))?
                }
            }
        });

    let builder_field_initialized = field_metadata_list
        .clone()
        .into_iter()
        .zip(struct_data.fields.clone().into_iter())
        .map(|(field_metadata, field)| {
            let field_iden = &field.ident;
            if field_metadata.has_builder_attr != HasFieldAttr::None {
                quote::quote!(
                    #field_iden : std::vec![]
                )
            } else {
                quote::quote!(
                    #field_iden : None
                )
            }
        });

    let output_tokens = quote::quote! {
        pub struct #builder_struct_ident {
            #(#builder_declaraions_token_stream),*
        }

        impl #struct_name {
            pub fn builder() -> #builder_struct_ident {
                #builder_struct_ident {
                    #(#builder_field_initialized),*
                }
            }
        }

        impl #builder_struct_ident {
            #(#builder_methods)*

            pub fn build (&self) -> Result<#struct_name, Box<dyn std::error::Error>> {
                Ok(#struct_name {
                    #(#field_idents),*
                })
            }
        }


    };

    output_tokens.into()
}
