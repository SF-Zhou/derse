use super::*;
use quote::ToTokens;

fn compact(tokens: impl ToTokens) -> String {
    tokens.to_token_stream().to_string().replace(' ', "")
}

fn implementations(expansion: TokenStream2) -> Vec<syn::ItemImpl> {
    let file: syn::File = syn::parse2(expansion).expect("expansion must be valid Rust syntax");
    let [syn::Item::Const(scope)] = file.items.as_slice() else {
        panic!("generated items must be isolated in an anonymous constant");
    };
    assert_eq!(scope.ident, "_");
    let syn::Expr::Block(block) = &*scope.expr else {
        panic!("expected an isolation scope");
    };
    block
        .block
        .stmts
        .iter()
        .filter_map(|statement| match statement {
            syn::Stmt::Item(syn::Item::Impl(item)) => Some(item.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn invalid_attributes_report_actionable_errors_for_both_derives() {
    for (source, expected) in [
        (
            "#[derse(required)] struct Message(u8);",
            "derse attributes are only supported on fields",
        ),
        (
            "enum Message { #[derse(default)] Value }",
            "derse attributes are only supported on fields",
        ),
        (
            "struct Message(#[derse(skip)] u8);",
            "unsupported derse attribute",
        ),
        (
            "struct Message(#[derse(required, default)] u8);",
            "duplicate or conflicting field default policy",
        ),
        (
            "struct Message(#[derse(default)] #[derse(default)] u8);",
            "duplicate or conflicting field default policy",
        ),
        (
            "struct Message(#[derse(default = 7)] u8);",
            "expected string literal",
        ),
        (
            "struct Message(#[derse(default = \"not a path\")] u8);",
            "unexpected token",
        ),
        (
            "struct Message(#[derse(required = true)] u8);",
            "expected `,`",
        ),
        (
            "union Message { value: u8 }",
            "only structs and enums are supported",
        ),
    ] {
        let input = syn::parse_str::<DeriveInput>(source).unwrap();
        for expand in [expand_serialize, expand_deserialize] {
            let error = expand(&input).unwrap_err().to_string();
            assert!(error.contains(expected), "{source}: {error}");
        }
    }
}

#[test]
fn field_policies_distinguish_required_trait_and_function_defaults() {
    let input: DeriveInput = parse_quote! {
        struct Message {
            #[allow(dead_code)]
            implicit: u8,
            #[derse(default)]
            explicit: u8,
            #[derse(required)]
            required: u8,
            #[derse(default = "crate::defaults::value")]
            custom: u8,
        }
    };
    let fields = validate(&input).unwrap().fields;
    assert!(matches!(
        default_policy(fields[0]).unwrap(),
        DefaultPolicy::Trait
    ));
    assert!(matches!(
        default_policy(fields[1]).unwrap(),
        DefaultPolicy::Trait
    ));
    assert!(matches!(
        default_policy(fields[2]).unwrap(),
        DefaultPolicy::Required
    ));
    let DefaultPolicy::Function(path) = default_policy(fields[3]).unwrap() else {
        panic!("expected a custom function");
    };
    assert_eq!(compact(path), "crate::defaults::value");
}

#[test]
fn generated_names_avoid_raw_identifiers_higher_ranked_lifetimes_and_default_paths() {
    let input: DeriveInput = parse_quote! {
        struct Message<T>
        where T: for<'__derse_de> Trait<'__derse_de>
        {
            r#__derse_buf: T,
            __derse_buf_: u8,
            #[derse(default = "defaults::__derse_serializer")]
            value: u8,
        }
    };
    let names = Names::new(&input, &validate(&input).unwrap().policies);
    assert_eq!(names.ident("__derse_buf"), "__derse_buf__");
    assert_eq!(names.ident("__derse_serializer"), "__derse_serializer_");
    assert_eq!(names.lifetime("__derse_de").to_string(), "'__derse_de_");
    assert_eq!(names.ident("unused"), "unused");
}

#[test]
fn generic_usage_includes_type_const_and_lifetime_parameters() {
    let input: DeriveInput = parse_quote! { struct Message<'a, r#Type, const N: usize>; };
    for source in ["Type", "Vec<Type>", "[u8; N]", "&'a str"] {
        assert!(
            uses_generics(&syn::parse_str(source).unwrap(), &input.generics),
            "{source}"
        );
    }
    for source in ["u8", "Vec<Concrete>", "[u8; 3]", "&'static str"] {
        assert!(
            !uses_generics(&syn::parse_str(source).unwrap(), &input.generics),
            "{source}"
        );
    }
}

#[test]
fn explicit_bounds_include_where_clauses_without_bounding_associated_types() {
    let input: DeriveInput = parse_quote! {
        struct Message<'a, 'b: 'a, T: 'a + Serialize, U, const N: usize>
        where
            T: Deserialize<'a> + Default,
            U: Serialize,
            T::Value: Default,
            'a: 'b;
    };
    assert_eq!(bounds_for(&input.generics, &parse_quote!(T)).len(), 4);
    assert_eq!(bounds_for(&input.generics, &parse_quote!(U)).len(), 1);
    assert!(bounds_for(&input.generics, &parse_quote!(Missing)).is_empty());

    for (source, trait_name, lifetime, expected) in [
        ("T", "Serialize", None, true),
        ("(T, U)", "Serialize", None, true),
        ("(T, U)", "Default", None, false),
        ("T", "Deserialize", Some("'a"), true),
        ("T", "Deserialize", Some("'b"), false),
        ("&'b T", "Deserialize", Some("'a"), false),
        ("&'a T", "Deserialize", Some("'a"), true),
        ("T::Value", "Serialize", None, false),
        ("<T as Provider>::Value", "Serialize", None, false),
        ("Vec<T>", "Serialize", None, true),
        ("[T; N]", "Serialize", None, false),
        ("u8", "Serialize", None, false),
    ] {
        let lifetime = lifetime.map(|value| syn::parse_str::<Lifetime>(value).unwrap());
        assert_eq!(
            explicitly_bounded(
                &syn::parse_str(source).unwrap(),
                &input.generics,
                trait_name,
                lifetime.as_ref()
            ),
            expected,
            "{source}: {trait_name}",
        );
    }
}

#[test]
fn input_lifetime_comes_only_from_non_higher_ranked_deserialize_bounds() {
    for source in [
        "struct Message<'a, T: Deserialize<'a>>(T);",
        "struct Message<'a, T>(T) where T: derse::Deserialize<'a>;",
    ] {
        let input: DeriveInput = syn::parse_str(source).unwrap();
        assert_eq!(
            existing_input_lifetime(&input.generics)
                .unwrap()
                .to_string(),
            "'a"
        );
    }
    for source in [
        "struct Message<'a, T: for<'b> Deserialize<'b>>(T);",
        "struct Message<T: Deserialize<'static>>(T);",
        "struct Message<'a, T: 'a + Serialize>(T);",
        "struct Message<T>(T);",
    ] {
        let input: DeriveInput = syn::parse_str(source).unwrap();
        assert!(
            existing_input_lifetime(&input.generics).is_none(),
            "{source}"
        );
    }
    for source in [
        "'static",
        "Serialize",
        "Deserialize",
        "Deserialize<u8>",
        "for<'a> Deserialize<'a>",
    ] {
        let bound: syn::TypeParamBound = syn::parse_str(source).unwrap();
        assert!(deserialize_lifetime(&bound).is_none(), "{source}");
    }
    let input: DeriveInput = parse_quote! { struct Message<T: for<'a> Deserialize<'a>>(T); };
    assert!(explicitly_bounded(
        &parse_quote!(T),
        &input.generics,
        "Deserialize",
        Some(&parse_quote!('__input))
    ));
}

#[test]
fn recursion_recognizes_self_paths_without_confusing_projections_and_other_modules() {
    let name = parse_quote!(Node);
    for source in [
        "Self",
        "Node<T>",
        "crate::tree::Node<T>",
        "self::Node<T>",
        "super::Node<T>",
        "Vec<Node<T>>",
    ] {
        assert!(
            contains_self(&syn::parse_str(source).unwrap(), &name),
            "{source}"
        );
    }
    for source in [
        "T::Node",
        "other::Node<T>",
        "<T as Provider>::Node",
        "Vec<T>",
        "u8",
    ] {
        assert!(
            !contains_self(&syn::parse_str(source).unwrap(), &name),
            "{source}"
        );
    }
    let ty: Type = parse_quote! { (T, Vec<Node<T>>, [u8; 3], Option<(Node<T>, Box<U>)>) };
    let parts: Vec<_> = nonrecursive_types(&ty, &name)
        .into_iter()
        .map(compact)
        .collect();
    assert_eq!(parts, ["T", "[u8;3]", "Box<U>"]);
    assert!(nonrecursive_types(&parse_quote!(Self), &name).is_empty());
}

#[test]
fn expansions_preserve_struct_and_enum_shapes_and_use_resolved_trait_paths() {
    for source in [
        "struct Unit;",
        "struct Tuple(u8, u16);",
        "struct Named { value: u8 }",
        "enum Message { Unit, Tuple(u8, u16), Named { value: u8 } }",
        "enum Empty {}",
    ] {
        let input: DeriveInput = syn::parse_str(source).unwrap();
        let serialization = implementations(expand_serialize(&input).unwrap());
        assert_eq!(serialization.len(), 1);
        assert_eq!(
            compact(&serialization[0].trait_.as_ref().unwrap().1),
            "::derse::Serialize"
        );
        let deserialization = implementations(expand_deserialize(&input).unwrap());
        assert_eq!(deserialization.len(), 2);
        assert_eq!(
            compact(&deserialization[0].trait_.as_ref().unwrap().1),
            "::derse::DetailedDeserialize<'__derse_de>"
        );
        assert_eq!(
            compact(&deserialization[1].trait_.as_ref().unwrap().1),
            "::derse::Deserialize<'__derse_de>"
        );
        assert_eq!(compact(&serialization[0].self_ty), input.ident.to_string());
        assert_eq!(
            compact(&deserialization[0].self_ty),
            input.ident.to_string()
        );
        for implementation in serialization.iter().chain(&deserialization) {
            assert!(implementation
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("automatically_derived")));
        }
    }
}

#[test]
fn recursive_expansions_bound_nonrecursive_parts_and_preserve_existing_bounds() {
    let input: DeriveInput = parse_quote! {
        struct Node<T, U: Serialize + for<'a> Deserialize<'a> + Default> {
            #[derse(required)]
            nested: Option<(Node<T, U>, T)>,
            existing: U,
        }
    };
    let serialization = implementations(expand_serialize(&input).unwrap());
    assert_eq!(
        compact(serialization[0].generics.where_clause.as_ref().unwrap()),
        "whereT:::derse::Serialize"
    );
    let deserialization = implementations(expand_deserialize(&input).unwrap());
    assert_eq!(
        compact(deserialization[0].generics.where_clause.as_ref().unwrap()),
        "whereT:::derse::Deserialize<'__derse_de>"
    );
}

#[test]
fn inferred_bounds_are_field_specific_and_default_requirements_follow_policy() {
    let input: DeriveInput = parse_quote! {
        struct Message<T> {
            #[derse(required)]
            required: T,
            #[derse(default = "fallback")]
            custom: T,
            defaulted: T,
        }
    };
    let serialization = implementations(expand_serialize(&input).unwrap());
    let serialized = compact(serialization[0].generics.where_clause.as_ref().unwrap());
    assert_eq!(serialized.matches("::derse::Serialize").count(), 3);
    for index in 0..3 {
        assert!(serialized.contains(&format!("PhantomData<[();{index}usize]>")));
    }
    let deserialization = implementations(expand_deserialize(&input).unwrap());
    let deserialized = compact(deserialization[0].generics.where_clause.as_ref().unwrap());
    assert_eq!(deserialized.matches("::derse::Deserialize").count(), 3);
    assert_eq!(deserialized.matches("::core::default::Default").count(), 1);
    assert!(deserialized
        .contains("(T,::core::marker::PhantomData<[();2usize]>):::core::default::Default"));
    let body = compact(&deserialization[0]);
    assert!(body.contains("fallback()"));
    assert_eq!(body.matches("::derse::Deserializer::is_empty").count(), 2);
}

#[test]
fn expansion_reuses_an_explicit_input_lifetime() {
    let input: DeriveInput = parse_quote! {
        struct Message<'input, T: Deserialize<'input> + Default>(T, &'input str);
    };
    for implementation in implementations(expand_deserialize(&input).unwrap()) {
        assert_eq!(implementation.generics.lifetimes().count(), 1);
        assert_eq!(
            implementation
                .generics
                .lifetimes()
                .next()
                .unwrap()
                .lifetime
                .to_string(),
            "'input"
        );
        assert!(compact(&implementation.trait_.unwrap().1).contains("<'input>"));
    }
}

#[test]
fn missing_runtime_dependency_reports_the_manifest_and_crate() {
    const CHILD_ENV: &str = "DERSE_TEST_MISSING_RUNTIME";
    if std::env::var_os(CHILD_ENV).is_some() {
        let error = get_crate_name().unwrap_err().to_string();
        assert!(error.contains("Could not find `derse`"), "{error}");
        let manifest = std::path::Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("Cargo.toml");
        assert!(error.contains(manifest.to_str().unwrap()), "{error}");
        return;
    }

    // Crate resolution reads Cargo's environment. A child process prevents this
    // failure case from racing the expansion tests that resolve a real dependency.
    let directory = std::env::temp_dir().join(format!(
        "derse-missing-runtime-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(
        directory.join("Cargo.toml"),
        "[package]\nname = 'without-runtime'\nversion = '0.0.0'\n[lib]\npath = 'lib.rs'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(directory.join("lib.rs"), "").unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "tests::missing_runtime_dependency_reports_the_manifest_and_crate",
            "--nocapture",
        ])
        .env(CHILD_ENV, "1")
        .env("CARGO_MANIFEST_DIR", &directory)
        .output();
    std::fs::remove_dir_all(&directory).unwrap();
    let output = result.unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
