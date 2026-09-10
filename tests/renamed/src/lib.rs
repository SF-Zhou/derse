//! Integration coverage for using derse under a different dependency name.

#[cfg(test)]
#[allow(dead_code, non_snake_case, non_upper_case_globals, unused_macros)]
mod tests {
    use ::ds::{Deserialize, DownwardBytes, Serialize};

    // These names must not affect any path emitted by the derive macros.
    mod derse {}
    mod ds {}
    struct Default;
    fn Ok() {}
    fn Err() {}
    const __derse_serializer: usize = 0;
    const __derse_buf: usize = 0;
    const __derse_start: usize = 0;
    const __derse_len: usize = 0;
    const __derse_tag: usize = 0;
    const __derse_field_0: usize = 0;
    macro_rules! format {
        ($($tokens:tt)*) => {
            compile_error!("derive used the caller's format macro")
        };
    }

    #[derive(Debug, PartialEq, ::ds::Serialize, ::ds::Deserialize)]
    struct Message<T> {
        value: T,
        text: String,
    }

    #[derive(Debug, PartialEq, ::ds::Serialize, ::ds::Deserialize)]
    enum Event {
        Unit,
        Named { serializer: u8 },
        Tuple(String),
    }

    #[test]
    fn renamed_dependency_and_shadowed_names() {
        let value = Message {
            value: 7u8,
            text: "hello".to_owned(),
        };
        let bytes = value.serialize::<DownwardBytes>().unwrap();
        assert_eq!(value.serialize::<usize>().unwrap(), bytes.len());
        assert_eq!(Message::deserialize(&bytes[..]).unwrap(), value);
        let fragments: Vec<_> = bytes.chunks(1).collect();
        assert_eq!(
            Message::deserialize(::ds::BytesArray::new(&fragments)).unwrap(),
            value
        );
        let value: Message<u8> = Message::deserialize(&[0][..]).unwrap();
        assert_eq!(value.value, 0);
        assert!(value.text.is_empty());

        for value in [
            Event::Unit,
            Event::Named { serializer: 42 },
            Event::Tuple("hello".to_owned()),
        ] {
            let bytes = value.serialize::<DownwardBytes>().unwrap();
            assert_eq!(Event::deserialize(&bytes[..]).unwrap(), value);
            let fragments: Vec<_> = bytes.chunks(1).collect();
            assert_eq!(
                Event::deserialize(::ds::BytesArray::new(&fragments)).unwrap(),
                value
            );
        }

        #[derive(::ds::Serialize)]
        enum FutureEvent {
            Unknown,
        }
        let bytes = FutureEvent::Unknown.serialize::<DownwardBytes>().unwrap();
        assert_eq!(
            Event::deserialize(&bytes[..]).unwrap_err(),
            ::ds::Error::InvalidType("Event::Unknown".to_owned()),
        );
    }

    #[test]
    fn custom_default_paths_are_not_shadowed_by_internal_bindings() {
        fn __derse_buf() -> u8 {
            42
        }

        #[derive(::ds::Deserialize)]
        struct Message(#[derse(default = "__derse_buf")] u8);

        assert_eq!(Message::deserialize(&[0][..]).unwrap().0, 42);
    }
}
