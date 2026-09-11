//! Encodings for standard-library types and optional integrations.
//!
//! These implementations are part of the wire format: changing an iteration
//! direction or adding a prefix changes bytes even when values still round-trip.

mod array;
mod collections;
mod cow;
mod cstr;
mod duration;
mod osstr;
mod pathbuf;
mod phantom_data;
mod primitive;
mod result;
mod socket_addr;
mod string;
mod tuple;

#[cfg(feature = "compact_str")]
mod compact_str;
#[cfg(feature = "tinyvec")]
mod tinyvec;
