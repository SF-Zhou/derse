use std::{borrow::Cow, marker::PhantomData};

use derse::{BytesArray, Deserialize, DownwardBytes, Error, Serialize};

#[test]
fn recursive_aliases_and_mutually_recursive_types_remain_supported() {
    type Children = Vec<Node>;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Node {
        value: u8,
        children: Children,
        branches: Vec<Branch>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Branch(Vec<Node>);

    let leaf = |value| Node {
        value,
        children: vec![],
        branches: vec![],
    };
    let value = Node {
        value: 1,
        children: vec![leaf(2)],
        branches: vec![Branch(vec![leaf(3)])],
    };
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(Node::deserialize(&bytes[..]).unwrap(), value);
}

#[test]
fn derives_call_traits_despite_inherent_methods() {
    #[derive(Default)]
    struct Field(u8);

    impl Field {
        #[allow(dead_code)]
        fn serialize_to<S: derse::Serializer>(&self, output: &mut S) -> derse::Result<()> {
            99u8.serialize_to(output)
        }

        #[allow(dead_code)]
        fn deserialize_from<'de, D: derse::Deserializer<'de>>(_: &mut D) -> derse::Result<Self> {
            panic!("derive must call the Deserialize trait")
        }
    }

    impl Serialize for Field {
        fn serialize_to<S: derse::Serializer>(&self, output: &mut S) -> derse::Result<()> {
            self.0.serialize_to(output)
        }
    }

    impl<'de> Deserialize<'de> for Field {
        fn deserialize_from<D: derse::Deserializer<'de>>(input: &mut D) -> derse::Result<Self> {
            u8::deserialize_from(input).map(Self)
        }
    }

    #[derive(Serialize, Deserialize)]
    struct Message(Field);

    impl Message {
        #[allow(dead_code)]
        fn deserialize_len<'de, D: derse::Deserializer<'de>>(_: &mut D) -> derse::Result<usize> {
            panic!("derive must call the DetailedDeserialize trait")
        }

        #[allow(dead_code)]
        fn deserialize_fields<'de, D: derse::Deserializer<'de>>(_: &mut D) -> derse::Result<Self> {
            panic!("derive must call the DetailedDeserialize trait")
        }
    }

    let bytes = Message(Field(7)).serialize::<DownwardBytes>().unwrap();
    assert_eq!(bytes.as_ref(), &[1, 7]);
    assert_eq!(Message(Field(7)).serialize::<usize>().unwrap(), bytes.len());
    let Message(Field(value)) = Message::deserialize(&bytes[..]).unwrap();
    assert_eq!(value, 7);

    #[derive(Serialize, Deserialize)]
    enum Choice {
        V(Field),
    }

    let bytes = Choice::V(Field(7)).serialize::<DownwardBytes>().unwrap();
    assert_eq!(bytes.as_ref(), &[3, 1, b'V', 7]);
    let Choice::V(Field(value)) = Choice::deserialize(&bytes[..]).unwrap();
    assert_eq!(value, 7);
}

#[test]
fn generated_names_do_not_shadow_fields_or_generics() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Message<Serializer, Deserializer, __DerseSerializer, __DerseDeserializer> {
        Named {
            serializer: Serializer,
            buf: Deserializer,
            __derse_serializer: __DerseSerializer,
            __derse_field_0: __DerseDeserializer,
        },
        Tuple(
            Serializer,
            Deserializer,
            __DerseSerializer,
            __DerseDeserializer,
        ),
    }

    for value in [
        Message::Named {
            serializer: 7u8,
            buf: 8u16,
            __derse_serializer: 9u32,
            __derse_field_0: 10u64,
        },
        Message::Tuple(11, 12, 13, 14),
    ] {
        let bytes = value.serialize::<DownwardBytes>().unwrap();
        assert_eq!(Message::deserialize(&bytes[..]).unwrap(), value);
    }
}

#[test]
fn generated_lifetimes_respect_higher_ranked_bounds() {
    trait Bound<'a> {}
    impl<'a> Bound<'a> for u8 {}

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Message<T>(T)
    where
        T: for<'__derse_de> Bound<'__derse_de> + for<'__derse_borrow> Bound<'__derse_borrow>;

    let value = Message(7u8);
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(Message::<u8>::deserialize(&bytes[..]).unwrap(), value);
}

#[test]
fn generic_bounds_follow_field_types() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Wrapper<T>(T);

    let value = Wrapper(42u8);
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(Wrapper::<u8>::deserialize(&bytes[..]).unwrap(), value);

    trait HasValue {
        type Value;
    }

    struct NoSerializationTraits;

    impl HasValue for NoSerializationTraits {
        type Value = u8;
    }

    #[derive(Serialize, Deserialize)]
    struct Associated<T: HasValue> {
        value: T::Value,
        marker: PhantomData<T>,
    }

    let value = Associated::<NoSerializationTraits> {
        value: 7,
        marker: PhantomData,
    };
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(
        Associated::<NoSerializationTraits>::deserialize(&bytes[..])
            .unwrap()
            .value,
        7
    );

    #[derive(Serialize, Deserialize)]
    struct Marker<'a, T>(PhantomData<&'a T>);

    let bytes = Marker::<NoSerializationTraits>(PhantomData)
        .serialize::<DownwardBytes>()
        .unwrap();
    // An output-only marker lifetime must not require the input to be 'static.
    let _: Marker<'static, NoSerializationTraits> = Marker::deserialize(&bytes[..]).unwrap();
}

#[test]
fn nonrecursive_type_paths_keep_automatic_field_bounds() {
    {
        mod other {
            #[derive(Default, derse::Serialize, derse::Deserialize)]
            pub struct Node<T>(pub T);
        }

        #[derive(Serialize, Deserialize)]
        struct Node<T>(other::Node<T>);

        let bytes = Node(other::Node(7u8)).serialize::<DownwardBytes>().unwrap();
        let Node(other::Node(value)) = Node::<u8>::deserialize(&bytes[..]).unwrap();
        assert_eq!(value, 7);
    }

    {
        trait HasNode {
            type Node;
        }

        struct Provider;
        impl HasNode for Provider {
            type Node = u8;
        }

        #[derive(Serialize, Deserialize)]
        struct Node<T: HasNode> {
            shorthand: T::Node,
            qualified: <T as HasNode>::Node,
        }

        let value = Node::<Provider> {
            shorthand: 7,
            qualified: 8,
        };
        let bytes = value.serialize::<DownwardBytes>().unwrap();
        let decoded = Node::<Provider>::deserialize(&bytes[..]).unwrap();
        assert_eq!(decoded.shorthand, value.shorthand);
        assert_eq!(decoded.qualified, value.qualified);
    }
}

#[test]
fn deriving_recursive_types_does_not_create_cyclic_bounds() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Node {
        value: u8,
        children: Vec<Node>,
    }

    let value = Node {
        value: 7,
        children: vec![Node {
            value: 8,
            children: Vec::new(),
        }],
    };
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(Node::deserialize(&bytes[..]).unwrap(), value);

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Tree<T> {
        value: T,
        children: Vec<Tree<T>>,
    }

    let value = Tree {
        value: 7u8,
        children: vec![Tree {
            value: 8,
            children: Vec::new(),
        }],
    };
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(Tree::<u8>::deserialize(&bytes[..]).unwrap(), value);
}

#[test]
fn recursive_fields_infer_bounds_for_nonrecursive_components() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Tree<T> {
        edges: Vec<(T, Tree<T>)>,
    }

    let value = Tree {
        edges: vec![(7u8, Tree { edges: vec![] })],
    };
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(bytes.as_ref(), &[4, 1, 7, 1, 0]);
    assert_eq!(Tree::<u8>::deserialize(&bytes[..]).unwrap(), value);
}

#[test]
fn const_generic_fields_respect_array_length_limit() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Samples<const N: usize> {
        #[derse(required)]
        values: [u16; N],
    }

    let empty = Samples { values: [] };
    let bytes = empty.serialize::<DownwardBytes>().unwrap();
    assert_eq!(bytes.as_ref(), &[0]);
    assert_eq!(Samples::<0>::deserialize(&bytes[..]).unwrap(), empty);

    let value = Samples { values: [1; 32] };
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    let expected: Vec<u8> = std::iter::once(64)
        .chain([1, 0].into_iter().cycle().take(64))
        .collect();
    assert_eq!(bytes.as_ref(), expected);
    let fragments: Vec<_> = bytes.chunks(3).collect();
    assert_eq!(
        Samples::<32>::deserialize(BytesArray::new(&fragments)).unwrap(),
        value
    );
    assert!(Samples::<32>::deserialize(&[0][..]).is_err());
}

#[test]
fn recursive_aliases_preserve_explicit_generic_bounds() {
    mod owned {
        use derse::{Deserialize, Serialize};

        type Children<T> = Vec<Node<T>>;

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        pub struct Node<T>
        where
            T: Serialize + for<'a> Deserialize<'a> + Default,
        {
            pub value: T,
            pub children: Children<T>,
        }
    }

    mod borrowed {
        use std::marker::PhantomData;

        use derse::{Deserialize, Serialize};

        type Children<'a, T> = Vec<Node<'a, T>>;

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        pub struct Node<'a, T: Serialize + Deserialize<'a> + Default> {
            pub value: T,
            pub children: Children<'a, T>,
            pub marker: PhantomData<&'a ()>,
        }
    }

    let value = owned::Node {
        value: 7u8,
        children: vec![owned::Node {
            value: 8,
            children: Vec::new(),
        }],
    };
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(owned::Node::<u8>::deserialize(&bytes[..]).unwrap(), value);

    let value = borrowed::Node {
        value: "root",
        children: vec![borrowed::Node {
            value: "leaf",
            children: Vec::new(),
            marker: PhantomData,
        }],
        marker: PhantomData,
    };
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(
        borrowed::Node::<&str>::deserialize(&bytes[..]).unwrap(),
        value
    );
}

#[test]
fn borrowed_fields_support_independent_lifetimes_and_outlives_bounds() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Borrowed<'a: 'b, 'b, '__derse_de> {
        first: &'a str,
        second: Cow<'b, str>,
        third: &'__derse_de str,
    }

    fn decode<'input, 'first, 'second, 'third>(
        input: &'input [u8],
    ) -> Borrowed<'first, 'second, 'third>
    where
        'input: 'first + 'second + 'third,
        'first: 'second,
    {
        Borrowed::deserialize(input).unwrap()
    }

    let value = Borrowed {
        first: "first",
        second: Cow::Borrowed("second"),
        third: "third",
    };
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    let decoded = decode(&bytes[..]);
    assert_eq!(decoded, value);
    assert!(matches!(decoded.second, Cow::Borrowed(_)));
}

#[test]
fn similar_field_types_can_have_distinct_lifetimes() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Strings<'a, 'b>(&'a str, &'b str);

    fn decode<'input: 'a + 'b, 'a, 'b>(input: &'input [u8]) -> Strings<'a, 'b> {
        Strings::deserialize(input).unwrap()
    }

    let value = Strings("first", "second");
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(decode(&bytes[..]), value);

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Cows<'a, 'b, T: Clone>(Cow<'a, T>, Cow<'b, T>);

    let decoded: Cows<'static, 'static, u8> = {
        let value = Cows::<u8>(Cow::Owned(7), Cow::Owned(8));
        let bytes = value.serialize::<DownwardBytes>().unwrap();
        Cows::deserialize(&bytes[..]).unwrap()
    };
    assert_eq!(decoded, Cows(Cow::Owned(7), Cow::Owned(8)));
    assert!(matches!(decoded.0, Cow::Owned(_)));
    assert!(matches!(decoded.1, Cow::Owned(_)));
}

#[test]
fn enum_names_can_span_input_fragments() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Message {
        LongVariant(u8),
    }

    let value = Message::LongVariant(42);
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    // One-byte fragments force every multi-byte tag through owned Cow storage.
    let fragments: Vec<_> = bytes.chunks(1).collect();
    assert_eq!(
        Message::deserialize(BytesArray::new(&fragments)).unwrap(),
        value
    );

    #[derive(Serialize)]
    enum FutureMessage {
        UnknownVariant,
    }

    let bytes = FutureMessage::UnknownVariant
        .serialize::<DownwardBytes>()
        .unwrap();
    let fragments: Vec<_> = bytes.chunks(1).collect();
    assert_eq!(
        Message::deserialize(BytesArray::new(&fragments)).unwrap_err(),
        Error::InvalidType("Message::UnknownVariant".to_owned())
    );
}

#[test]
fn missing_fields_use_the_declared_policy() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct NoDefault(u8);

    fn fallback() -> NoDefault {
        NoDefault(42)
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Required {
        #[derse(required)]
        value: NoDefault,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct WithDefault(
        #[derse(default = "fallback")] NoDefault,
        #[derse(default)] String,
    );

    let value = Required {
        value: NoDefault(7),
    };
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(Required::deserialize(&bytes[..]).unwrap(), value);
    assert_eq!(
        Required::deserialize(&[0][..]).unwrap_err(),
        Error::DataIsShort {
            expect: 1,
            actual: 0
        }
    );
    assert_eq!(
        WithDefault::deserialize(&[0][..]).unwrap(),
        WithDefault(NoDefault(42), String::new())
    );

    let value = WithDefault(NoDefault(8), "present".to_owned());
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(WithDefault::deserialize(&bytes[..]).unwrap(), value);

    // Missing trailing fields may use defaults, but partial fields must still fail.
    assert!(matches!(
        WithDefault::deserialize(&[1, 1][..]),
        Err(Error::DataIsShort { .. })
    ));

    #[derive(Serialize)]
    enum Old {
        Required,
        Default,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    enum New {
        Required(#[derse(required)] NoDefault),
        Default {
            #[derse(default = "fallback")]
            value: NoDefault,
        },
    }

    let bytes = Old::Required.serialize::<DownwardBytes>().unwrap();
    assert_eq!(
        New::deserialize(&bytes[..]).unwrap_err(),
        Error::DataIsShort {
            expect: 1,
            actual: 0
        }
    );
    let bytes = Old::Default.serialize::<DownwardBytes>().unwrap();
    assert_eq!(
        New::deserialize(&bytes[..]).unwrap(),
        New::Default {
            value: NoDefault(42)
        }
    );

    #[derive(Debug, Deserialize, PartialEq)]
    struct RequiredZeroBytes(#[derse(required)] ());

    assert_eq!(
        RequiredZeroBytes::deserialize(&[0][..]).unwrap(),
        RequiredZeroBytes(())
    );
}

#[test]
fn empty_enums_derive_and_reject_all_tags() {
    #[derive(Debug, Serialize, Deserialize)]
    enum Empty {}

    #[derive(Serialize)]
    enum Source {
        Value,
    }

    let bytes = Source::Value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(
        Empty::deserialize(&bytes[..]).unwrap_err(),
        Error::InvalidType("Empty::Value".to_owned())
    );
}
