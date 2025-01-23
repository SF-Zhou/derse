mod bytes_array;
mod deserializer;
mod downward_bytes;
mod error;
mod r#impl;
mod serialization;
mod serializer;
mod varint64;

pub use bytes_array::BytesArray;
pub use deserializer::Deserializer;
pub use downward_bytes::DownwardBytes;
pub use error::{Error, Result};
pub use serializer::Serializer;
pub use varint64::VarInt64;

pub use derse_derive::{Deserialize, Serialize};

/// A trait for serializing data.
pub trait Serialize {
    /// Serializes the data into a new `Serializer` instance.
    ///
    /// # Returns
    ///
    /// A `Result` containing the serialized data or an error.
    fn serialize<S: Serializer + Default>(&self) -> Result<S> {
        let mut serializer = S::default();
        self.serialize_to(&mut serializer)?;
        Ok(serializer)
    }

    /// Serializes the data into the given `Serializer`.
    ///
    /// # Arguments
    ///
    /// * `serializer` - The `Serializer` to serialize the data into.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()>;
}

/// A trait for deserializing data.
pub trait Deserialize<'a> {
    /// Deserializes the data from a `Deserializer`.
    ///
    /// # Arguments
    ///
    /// * `der` - The `Deserializer` to deserialize the data from.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized data or an error.
    fn deserialize<D: Deserializer<'a>>(mut der: D) -> Result<Self>
    where
        Self: Sized,
    {
        Self::deserialize_from(&mut der)
    }

    /// Deserializes the data from the given `Deserializer`.
    ///
    /// # Arguments
    ///
    /// * `buf` - The `Deserializer` to deserialize the data from.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized data or an error.
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized;
}

/// A trait for detailed deserialization.
pub trait DetailedDeserialize<'a> {
    /// Deserializes the length from the given `Deserializer`.
    ///
    /// # Arguments
    ///
    /// * `buf` - The `Deserializer` to deserialize the length from.
    ///
    /// # Returns
    ///
    /// A `Result` containing the length or an error.
    fn deserialize_len<D: Deserializer<'a>>(buf: &mut D) -> Result<usize>;

    /// Deserializes the fields from the given `Deserializer`.
    ///
    /// # Arguments
    ///
    /// * `buf` - The `Deserializer` to deserialize the fields from.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized fields or an error.
    fn deserialize_fields<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized;
}
