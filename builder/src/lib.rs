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

    let is_optional_type_list: Vec<bool> = struct_data
        .fields
        .clone()
        .into_iter()
        .map(|field| {
            match field.ty {
                syn::Type::Path(type_path) => {
                    // This is not an exhaustive list, we could also represent our optional type from the standard library as
                    // std::option::Option
                    return type_path.path.segments.first().unwrap().ident == "Option";
                }
                _ => false,
            }
        })
        .collect();
    let extract_inner_type_from_option = |type_t: syn::Type| {
        if let syn::Type::Path(type_path) = type_t {
            assert!(type_path.path.segments.first().unwrap().ident == "Option");
            if let syn::PathArguments::AngleBracketed(args) =
                &type_path.path.segments.first().unwrap().arguments
            {
                if let syn::GenericArgument::Type(inner_type) = args.args.first().unwrap() {
                    inner_type.clone()
                } else {
                    todo!("Improve Error Message");
                }
            } else {
                todo!("Improve Error Message");
            }
        } else {
            todo!("Improve Error Message");
        }
    };
    let builder_declaraions_token_stream = is_optional_type_list
        .clone()
        .into_iter()
        .zip(struct_data.fields.clone().into_iter())
        .map(|(is_optional, field)| {
            let field_ident = field.ident.clone().unwrap();
            let field_type = {
                if is_optional {
                    extract_inner_type_from_option(field.ty)
                } else {
                    field.ty
                }
            };
            quote::quote! {
                #field_ident : std::option::Option<#field_type>
            }
        });

    let builder_methods = is_optional_type_list
        .clone()
        .into_iter()
        .zip(struct_data.fields.clone().into_iter())
        .map(|(is_optional, field)| {
            println!("{:#?}", field);
            let field_iden = &field.ident;
            let field_type;
            if is_optional {
                field_type = extract_inner_type_from_option(field.ty);
            } else {
                field_type = field.ty;
            }
            quote::quote! {
                pub fn #field_iden(&mut self, #field_iden : #field_type) -> &mut Self {
                    self.#field_iden = Some(#field_iden);
                    self
                }
            }
        });

    let field_idents = is_optional_type_list
        .clone()
        .into_iter()
        .zip(struct_data.fields.clone().into_iter())
        .map(|(is_optional, field)| {
            let field_iden = field.clone().ident.unwrap();
            if is_optional {
                quote::quote! {
                     #field_iden : self.#field_iden.clone()
                }
            } else {
                quote::quote! {
                    #field_iden : self.#field_iden.clone().ok_or(stringify!(#field_iden not found))?
                }
            }
        });

    let builder_field_initialized = struct_data.fields.clone().into_iter().map(|field| {
        let field_iden = &field.ident;
        quote::quote!(
            #field_iden : None
        )
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
