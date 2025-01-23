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
