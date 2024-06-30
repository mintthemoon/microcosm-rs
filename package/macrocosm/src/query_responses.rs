use syn::{
    parse_quote, Error, Expr, ExprTuple, Generics, ItemEnum, ItemImpl, Result, Type,
    TypeParamBound, Variant,
};

mod context {
    use super::*;
    use std::collections::HashSet;
    use syn::Ident;

    const ATTR_PATH: &str = "query_responses";

    pub struct Context {
        /// If the enum we're trying to derive QueryResponses for collects other QueryMsgs,
        /// setting this flag will derive the implementation appropriately, collecting all
        /// KV pairs from the nested enums rather than expecting `#[return]` annotations.
        pub is_nested: bool,
        /// Disable infering the `JsonSchema` trait bound for chosen type parameters.
        pub no_bounds_for: HashSet<Ident>,
    }

    pub fn get_context(input: &ItemEnum) -> Result<Context> {
        let mut ctx = Context {
            is_nested: false,
            no_bounds_for: HashSet::new(),
        };

        for attr in &input.attrs {
            if !attr.path().is_ident(ATTR_PATH) {
                continue;
            }
            let meta_list = attr.meta.require_list()?;
            meta_list.parse_nested_meta(|param| {
                if param.path.is_ident("no_bounds_for") {
                    param.parse_nested_meta(|item| {
                        ctx.no_bounds_for
                            .insert(item.path.get_ident().unwrap().clone());

                        Ok(())
                    })?;
                } else if param.path.is_ident("nested") {
                    ctx.is_nested = true;
                } else {
                    Error::new_spanned(param.path, "unrecognized QueryResponses param");
                }
                Ok(())
            })?;
        }
        Ok(ctx)
    }

    #[cfg(test)]
    mod test {
        use std::collections::HashSet;

        use quote::format_ident;
        use syn::parse_quote;

        use super::get_context;

        #[test]
        fn parse_context() {
            let input = parse_quote! {
                #[query_responses(crate = "::my_crate::cw_schema")]
                #[query_responses(nested)]
                #[query_responses(no_bounds_for(Item1, Item2))]
                enum Test {}
            };
            let context = get_context(&input).unwrap();

            assert!(context.is_nested);
            assert_eq!(
                context.no_bounds_for,
                HashSet::from([format_ident!("Item1"), format_ident!("Item2")])
            );
        }
    }
}

use context::Context;

pub fn query_responses_derive_impl(input: ItemEnum) -> Result<ItemImpl> {
    let ctx = context::get_context(&input)?;
    let item_impl = if ctx.is_nested {
        let ident = input.ident;
        let subquery_calls = input
            .variants
            .into_iter()
            .map(|variant| parse_subquery(&ctx, variant))
            .collect::<Result<Vec<_>>>()?;
        // Handle generics if the type has any
        let (_, type_generics, where_clause) = input.generics.split_for_impl();
        let impl_generics = impl_generics(
            &ctx,
            &input.generics,
            &[parse_quote! {::microcosm::schema::QueryResponses}],
        );
        let subquery_len = subquery_calls.len();
        parse_quote! {
            #[automatically_derived]
            #[cfg(not(target_arch = "wasm32"))]
            impl #impl_generics ::microcosm::schema::QueryResponses for #ident #type_generics #where_clause {
                fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::microcosm::schemars::schema::RootSchema> {
                    let subqueries = [
                        #( #subquery_calls, )*
                    ];
                    ::microcosm::schema::combine_subqueries::<#subquery_len, #ident #type_generics>(subqueries)
                }
            }
        }
    } else {
        let ident = input.ident;
        let mappings = input
            .variants
            .into_iter()
            .map(|variant| parse_query(&ctx, variant))
            .collect::<syn::Result<Vec<_>>>()?;

        let mut queries: Vec<_> = mappings.clone().into_iter().map(|(q, _)| q).collect();
        queries.sort();
        let mappings = mappings.into_iter().map(parse_tuple);

        // Handle generics if the type has any
        let (_, type_generics, where_clause) = input.generics.split_for_impl();
        let impl_generics = impl_generics(&ctx, &input.generics, &[]);

        parse_quote! {
            #[automatically_derived]
            #[cfg(not(target_arch = "wasm32"))]
            impl #impl_generics ::microcosm::schema::QueryResponses for #ident #type_generics #where_clause {
                fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::microcosm::schemars::schema::RootSchema> {
                    ::std::collections::BTreeMap::from([
                        #( #mappings, )*
                    ])
                }
            }
        }
    };
    Ok(item_impl)
}

/// Takes a list of generics from the type definition and produces a list of generics
/// for the expanded `impl` block, adding trait bounds like `JsonSchema` as appropriate.
fn impl_generics(ctx: &Context, generics: &Generics, bounds: &[TypeParamBound]) -> Generics {
    let mut impl_generics = generics.to_owned();
    for param in impl_generics.type_params_mut() {
        // remove the default type if present, as those are invalid in
        // a trait implementation
        param.default = None;

        if !ctx.no_bounds_for.contains(&param.ident) {
            param
                .bounds
                .push(parse_quote! {::microcosm::schemars::JsonSchema});

            param.bounds.extend(bounds.to_owned());
        }
    }

    impl_generics
}

/// Extract the query -> response mapping out of an enum variant.
fn parse_query(_ctx: &Context, v: Variant) -> Result<(String, Expr)> {
    let query = to_snake_case(&v.ident.to_string());
    let response_ty: Type = v
        .attrs
        .iter()
        .find(|a| a.path().is_ident("returns"))
        .ok_or_else(|| Error::new_spanned(&v, "missing return type for query"))?
        .parse_args()
        .map_err(|e| Error::new(e.span(), "return must be a type"))?;

    Ok((query, parse_quote!(::microcosm::schema::schema_for!(#response_ty))))
}

/// Extract the nested query  -> response mapping out of an enum variant.
fn parse_subquery(_ctx: &Context, v: Variant) -> Result<Expr> {
    let submsg = match v.fields {
        syn::Fields::Named(_) => {
            return Err(Error::new_spanned(
                v,
                "a struct variant is not a valid subquery",
            ))
        }
        syn::Fields::Unnamed(fields) => {
            if fields.unnamed.len() != 1 {
                return Err(Error::new_spanned(
                    fields,
                    "invalid number of subquery parameters",
                ));
            }
            fields.unnamed[0].ty.clone()
        }
        syn::Fields::Unit => {
            return Err(Error::new_spanned(
                v,
                "a unit variant is not a valid subquery",
            ));
        }
    };

    Ok(parse_quote!(<#submsg as ::microcosm::schema::QueryResponses>::response_schemas_impl()))
}

fn parse_tuple((q, r): (String, Expr)) -> ExprTuple {
    parse_quote! {
        (#q.to_string(), #r)
    }
}

fn to_snake_case(input: &str) -> String {
    // this was stolen from serde for consistent behavior
    let mut snake = String::new();
    for (i, ch) in input.char_indices() {
        if i > 0 && ch.is_uppercase() {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }
    snake
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use syn::parse_quote;

    use super::*;

    fn test_context() -> Context {
        Context {
            is_nested: false,
            no_bounds_for: HashSet::new(),
        }
    }

    #[test]
    fn crate_rename() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(rename_all = "snake_case")]
            #[query_responses(crate = "::my_crate::cw_schema")]
            pub enum QueryMsg {
                #[returns(some_crate::AnotherType)]
                Supply {},
                #[returns(SomeType)]
                Balance {},
            }
        };

        assert_eq!(
            query_responses_derive_impl(input).unwrap(),
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl ::my_crate::cw_schema::QueryResponses for QueryMsg {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::my_crate::cw_schema::schemars::schema::RootSchema> {
                        ::std::collections::BTreeMap::from([
                            ("supply".to_string(), ::my_crate::cw_schema::schema_for!(some_crate::AnotherType)),
                            ("balance".to_string(), ::my_crate::cw_schema::schema_for!(SomeType)),
                        ])
                    }
                }
            }
        );
    }

    #[test]
    fn crate_rename_nested() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(untagged)]
            #[query_responses(crate = "::my_crate::cw_schema", nested)]
            pub enum ContractQueryMsg {
                Cw1(QueryMsg1),
                Whitelist(whitelist::QueryMsg),
                Cw1WhitelistContract(QueryMsg),
            }
        };
        let result = query_responses_derive_impl(input).unwrap();
        assert_eq!(
            result,
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl ::my_crate::cw_schema::QueryResponses for ContractQueryMsg {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::my_crate::cw_schema::schemars::schema::RootSchema> {
                        let subqueries = [
                            <QueryMsg1 as ::my_crate::cw_schema::QueryResponses>::response_schemas_impl(),
                            <whitelist::QueryMsg as ::my_crate::cw_schema::QueryResponses>::response_schemas_impl(),
                            <QueryMsg as ::my_crate::cw_schema::QueryResponses>::response_schemas_impl(),
                        ];
                        ::my_crate::cw_schema::combine_subqueries::<3usize, ContractQueryMsg>(subqueries)
                    }
                }
            }
        );
    }

    #[test]
    fn happy_path() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(rename_all = "snake_case")]
            pub enum QueryMsg {
                #[returns(some_crate::AnotherType)]
                Supply {},
                #[returns(SomeType)]
                Balance {},
            }
        };

        assert_eq!(
            query_responses_derive_impl(input).unwrap(),
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl ::cosmwasm_schema::QueryResponses for QueryMsg {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::cosmwasm_schema::schemars::schema::RootSchema> {
                        ::std::collections::BTreeMap::from([
                            ("supply".to_string(), ::cosmwasm_schema::schema_for!(some_crate::AnotherType)),
                            ("balance".to_string(), ::cosmwasm_schema::schema_for!(SomeType)),
                        ])
                    }
                }
            }
        );
    }

    #[test]
    fn empty_query_msg() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(rename_all = "snake_case")]
            pub enum QueryMsg {}
        };

        assert_eq!(
            query_responses_derive_impl(input).unwrap(),
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl ::cosmwasm_schema::QueryResponses for QueryMsg {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::cosmwasm_schema::schemars::schema::RootSchema> {
                        ::std::collections::BTreeMap::from([])
                    }
                }
            }
        );
    }

    #[test]
    fn generics() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(rename_all = "snake_case")]
            pub enum QueryMsg<T> {
                #[returns(bool)]
                Foo,
                #[returns(u32)]
                Bar(T),
            }
        };

        let input2: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(rename_all = "snake_case")]
            pub enum QueryMsg<T: std::fmt::Debug + SomeTrait> {
                #[returns(bool)]
                Foo,
                #[returns(u32)]
                Bar { data: T },
            }
        };

        let input3: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(rename_all = "snake_case")]
            pub enum QueryMsg<T>
                where T: std::fmt::Debug + SomeTrait,
            {
                #[returns(bool)]
                Foo,
                #[returns(u32)]
                Bar { data: T },
            }
        };

        let result = query_responses_derive_impl(input).unwrap();

        assert_eq!(
            result,
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl<T: ::cosmwasm_schema::schemars::JsonSchema> ::cosmwasm_schema::QueryResponses for QueryMsg<T> {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::cosmwasm_schema::schemars::schema::RootSchema> {
                        ::std::collections::BTreeMap::from([
                            ("foo".to_string(), ::cosmwasm_schema::schema_for!(bool)),
                            ("bar".to_string(), ::cosmwasm_schema::schema_for!(u32)),
                        ])
                    }
                }
            }
        );
        assert_eq!(
            query_responses_derive_impl(input2).unwrap(),
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl<T: std::fmt::Debug + SomeTrait + ::cosmwasm_schema::schemars::JsonSchema> ::cosmwasm_schema::QueryResponses for QueryMsg<T> {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::cosmwasm_schema::schemars::schema::RootSchema> {
                        ::std::collections::BTreeMap::from([
                            ("foo".to_string(), ::cosmwasm_schema::schema_for!(bool)),
                            ("bar".to_string(), ::cosmwasm_schema::schema_for!(u32)),
                        ])
                    }
                }
            }
        );
        let a = query_responses_derive_impl(input3).unwrap();
        assert_eq!(
            a,
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl<T: ::cosmwasm_schema::schemars::JsonSchema> ::cosmwasm_schema::QueryResponses for QueryMsg<T>
                    where T: std::fmt::Debug + SomeTrait,
                {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::cosmwasm_schema::schemars::schema::RootSchema> {
                        ::std::collections::BTreeMap::from([
                            ("foo".to_string(), ::cosmwasm_schema::schema_for!(bool)),
                            ("bar".to_string(), ::cosmwasm_schema::schema_for!(u32)),
                        ])
                    }
                }
            }
        );
    }

    #[test]
    #[should_panic(expected = "missing return type for query")]
    fn missing_return() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(rename_all = "snake_case")]
            pub enum QueryMsg {
                Supply {},
                #[returns(SomeType)]
                Balance {},
            }
        };

        query_responses_derive_impl(input).unwrap();
    }

    #[test]
    #[should_panic(expected = "return must be a type")]
    fn invalid_return() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(rename_all = "snake_case")]
            pub enum QueryMsg {
                #[returns(1)]
                Supply {},
                #[returns(SomeType)]
                Balance {},
            }
        };

        query_responses_derive_impl(input).unwrap();
    }

    #[test]
    fn parse_query_works() {
        let variant = parse_quote! {
            #[returns(Foo)]
            GetFoo {}
        };

        assert_eq!(
            parse_tuple(parse_query(&test_context(), variant).unwrap()),
            parse_quote! {
                ("get_foo".to_string(), ::cosmwasm_schema::schema_for!(Foo))
            }
        );

        let variant = parse_quote! {
            #[returns(some_crate::Foo)]
            GetFoo {}
        };

        assert_eq!(
            parse_tuple(parse_query(&test_context(), variant).unwrap()),
            parse_quote! { ("get_foo".to_string(), ::cosmwasm_schema::schema_for!(some_crate::Foo)) }
        );
    }

    #[test]
    fn to_snake_case_works() {
        assert_eq!(to_snake_case("SnakeCase"), "snake_case");
        assert_eq!(to_snake_case("Wasm123AndCo"), "wasm123_and_co");
    }

    #[test]
    fn nested_works() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(untagged)]
            #[query_responses(nested)]
            pub enum ContractQueryMsg {
                Cw1(QueryMsg1),
                Whitelist(whitelist::QueryMsg),
                Cw1WhitelistContract(QueryMsg),
            }
        };
        let result = query_responses_derive_impl(input).unwrap();
        assert_eq!(
            result,
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl ::cosmwasm_schema::QueryResponses for ContractQueryMsg {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::cosmwasm_schema::schemars::schema::RootSchema> {
                        let subqueries = [
                            <QueryMsg1 as ::cosmwasm_schema::QueryResponses>::response_schemas_impl(),
                            <whitelist::QueryMsg as ::cosmwasm_schema::QueryResponses>::response_schemas_impl(),
                            <QueryMsg as ::cosmwasm_schema::QueryResponses>::response_schemas_impl(),
                        ];
                        ::cosmwasm_schema::combine_subqueries::<3usize, ContractQueryMsg>(subqueries)
                    }
                }
            }
        );
    }

    #[test]
    fn nested_empty() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(untagged)]
            #[query_responses(nested)]
            pub enum EmptyMsg {}
        };
        let result = query_responses_derive_impl(input).unwrap();
        assert_eq!(
            result,
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl ::cosmwasm_schema::QueryResponses for EmptyMsg {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::cosmwasm_schema::schemars::schema::RootSchema> {
                        let subqueries = [];
                        ::cosmwasm_schema::combine_subqueries::<0usize, EmptyMsg>(subqueries)
                    }
                }
            }
        );
    }

    #[test]
    #[should_panic(expected = "invalid number of subquery parameters")]
    fn nested_too_many_params() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(untagged)]
            #[query_responses(nested)]
            pub enum ContractQueryMsg {
                Msg1(QueryMsg1, QueryMsg2),
                Whitelist(whitelist::QueryMsg),
            }
        };
        query_responses_derive_impl(input).unwrap();
    }

    #[test]
    #[should_panic(expected = "a struct variant is not a valid subquery")]
    fn nested_mixed() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(untagged)]
            #[query_responses(nested)]
            pub enum ContractQueryMsg {
                Cw1(cw1::QueryMsg),
                Test {
                    mixed: bool,
                }
            }
        };
        query_responses_derive_impl(input).unwrap();
    }

    #[test]
    #[should_panic(expected = "a unit variant is not a valid subquery")]
    fn nested_unit_variant() {
        let input: ItemEnum = parse_quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
            #[serde(untagged)]
            #[query_responses(nested)]
            pub enum ContractQueryMsg {
                Cw1(cw1::QueryMsg),
                Whitelist,
            }
        };
        query_responses_derive_impl(input).unwrap();
    }
}
