use syn::{
    parse_quote, Expr, ExprTuple, Generics, ItemEnum, ItemImpl, Type, TypeParamBound, Variant
};
use context::Context;

pub mod context {
    use std::collections::HashSet;
    use syn::{ItemEnum, Meta, NestedMeta, Ident};

    const ATTR_PATH: &str = "query_responses";

    pub struct Context {
        /// If the enum we're trying to derive QueryResponses for collects other QueryMsgs,
        /// setting this flag will derive the implementation appropriately, collecting all
        /// KV pairs from the nested enums rather than expecting `#[return]` annotations.
        pub is_nested: bool,
        /// Disable infering the `JsonSchema` trait bound for chosen type parameters.
        pub no_bounds_for: HashSet<Ident>,
    }
    
    pub fn get_context(input: &ItemEnum) -> Context {
        let params = input
            .attrs
            .iter()
            .filter(|attr| matches!(attr.path.get_ident(), Some(id) if *id == ATTR_PATH))
            .flat_map(|attr| {
                if let Meta::List(l) = attr.parse_meta().unwrap() {
                    l.nested
                } else {
                    panic!("{ATTR_PATH} attribute must contain a meta list");
                }
            })
            .map(|nested_meta| {
                if let NestedMeta::Meta(m) = nested_meta {
                    m
                } else {
                    panic!("no literals allowed in QueryResponses params")
                }
            });
    
        let mut ctx = Context {
            is_nested: false,
            no_bounds_for: HashSet::new(),
        };
    
        for param in params {
            match param.path().get_ident().unwrap().to_string().as_str() {
                "no_bounds_for" => {
                    if let Meta::List(l) = param {
                        for item in l.nested {
                            match item {
                                NestedMeta::Meta(Meta::Path(p)) => {
                                    ctx.no_bounds_for.insert(p.get_ident().unwrap().clone());
                                }
                                _ => panic!("`no_bounds_for` only accepts a list of type params"),
                            }
                        }
                    } else {
                        panic!("expected a list for `no_bounds_for`")
                    }
                }
                "nested" => ctx.is_nested = true,
                path => panic!("unrecognized QueryResponses param: {path}"),
            }
        }
    
        ctx
    }    
}

pub fn query_responses_derive_impl(input: ItemEnum) -> ItemImpl {
    let ctx = context::get_context(&input);

    if ctx.is_nested {
        let ident = input.ident;
        let subquery_calls = input.variants.into_iter().map(parse_subquery);

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
        let mappings = input.variants.into_iter().map(parse_query);
        let mut queries: Vec<_> = mappings.clone().map(|(q, _)| q).collect();
        queries.sort();
        let mappings = mappings.map(parse_tuple);

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
    }
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
fn parse_query(v: Variant) -> (String, Expr) {
    let query = to_snake_case(&v.ident.to_string());
    let response_ty: Type = v
        .attrs
        .iter()
        .find(|a| a.path.get_ident().unwrap() == "returns")
        .unwrap_or_else(|| panic!("missing return type for query: {}", v.ident))
        .parse_args()
        .unwrap_or_else(|_| panic!("return for {} must be a type", v.ident));

    (
        query,
        parse_quote!(::microcosm::schema::schema_for!(#response_ty)),
    )
}

/// Extract the nested query  -> response mapping out of an enum variant.
fn parse_subquery(v: Variant) -> Expr {
    let submsg = match v.fields {
        syn::Fields::Named(_) => panic!("a struct variant is not a valid subquery"),
        syn::Fields::Unnamed(fields) => {
            if fields.unnamed.len() != 1 {
                panic!("invalid number of subquery parameters");
            }

            fields.unnamed[0].ty.clone()
        }
        syn::Fields::Unit => panic!("a unit variant is not a valid subquery"),
    };
    parse_quote!(<#submsg as ::microcosm::schema::QueryResponses>::response_schemas_impl())
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
    use syn::parse_quote;

    use super::*;

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
            query_responses_derive_impl(input),
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl ::microcosm::schema::QueryResponses for QueryMsg {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::microcosm::schemars::schema::RootSchema> {
                        ::std::collections::BTreeMap::from([
                            ("supply".to_string(), ::microcosm::schema::schema_for!(some_crate::AnotherType)),
                            ("balance".to_string(), ::microcosm::schema::schema_for!(SomeType)),
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
            query_responses_derive_impl(input),
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl ::microcosm::schema::QueryResponses for QueryMsg {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::microcosm::schemars::schema::RootSchema> {
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

        let result = query_responses_derive_impl(input);

        assert_eq!(
            result,
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl<T: ::microcosm::schemars::JsonSchema> ::microcosm::schema::QueryResponses for QueryMsg<T> {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::microcosm::schemars::schema::RootSchema> {
                        ::std::collections::BTreeMap::from([
                            ("foo".to_string(), ::microcosm::schema::schema_for!(bool)),
                            ("bar".to_string(), ::microcosm::schema::schema_for!(u32)),
                        ])
                    }
                }
            }
        );
        assert_eq!(
            query_responses_derive_impl(input2),
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl<T: std::fmt::Debug + SomeTrait + ::microcosm::schemars::JsonSchema> ::microcosm::schema::QueryResponses for QueryMsg<T> {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::microcosm::schemars::schema::RootSchema> {
                        ::std::collections::BTreeMap::from([
                            ("foo".to_string(), ::microcosm::schema::schema_for!(bool)),
                            ("bar".to_string(), ::microcosm::schema::schema_for!(u32)),
                        ])
                    }
                }
            }
        );
        let a = query_responses_derive_impl(input3);
        assert_eq!(
            a,
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl<T: ::microcosm::schemars::JsonSchema> ::microcosm::schema::QueryResponses for QueryMsg<T>
                    where T: std::fmt::Debug + SomeTrait,
                {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::microcosm::schemars::schema::RootSchema> {
                        ::std::collections::BTreeMap::from([
                            ("foo".to_string(), ::microcosm::schema::schema_for!(bool)),
                            ("bar".to_string(), ::microcosm::schema::schema_for!(u32)),
                        ])
                    }
                }
            }
        );
    }

    #[test]
    #[should_panic(expected = "missing return type for query: Supply")]
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

        query_responses_derive_impl(input);
    }

    #[test]
    #[should_panic(expected = "return for Supply must be a type")]
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

        query_responses_derive_impl(input);
    }

    #[test]
    fn parse_query_works() {
        let variant = parse_quote! {
            #[returns(Foo)]
            GetFoo {}
        };

        assert_eq!(
            parse_tuple(parse_query(variant)),
            parse_quote! {
                ("get_foo".to_string(), ::microcosm::schema::schema_for!(Foo))
            }
        );

        let variant = parse_quote! {
            #[returns(some_crate::Foo)]
            GetFoo {}
        };

        assert_eq!(
            parse_tuple(parse_query(variant)),
            parse_quote! { ("get_foo".to_string(), ::microcosm::schema::schema_for!(some_crate::Foo)) }
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
        let result = query_responses_derive_impl(input);
        assert_eq!(
            result,
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl ::microcosm::schema::QueryResponses for ContractQueryMsg {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::microcosm::schemars::schema::RootSchema> {
                        let subqueries = [
                            <QueryMsg1 as ::microcosm::schema::QueryResponses>::response_schemas_impl(),
                            <whitelist::QueryMsg as ::microcosm::schema::QueryResponses>::response_schemas_impl(),
                            <QueryMsg as ::microcosm::schema::QueryResponses>::response_schemas_impl(),
                        ];
                        ::microcosm::schema::combine_subqueries::<3usize, ContractQueryMsg>(subqueries)
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
        let result = query_responses_derive_impl(input);
        assert_eq!(
            result,
            parse_quote! {
                #[automatically_derived]
                #[cfg(not(target_arch = "wasm32"))]
                impl ::microcosm::schema::QueryResponses for EmptyMsg {
                    fn response_schemas_impl() -> ::std::collections::BTreeMap<String, ::microcosm::schemars::schema::RootSchema> {
                        let subqueries = [];
                        ::microcosm::schema::combine_subqueries::<0usize, EmptyMsg>(subqueries)
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
        query_responses_derive_impl(input);
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
        query_responses_derive_impl(input);
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
        query_responses_derive_impl(input);
    }
}