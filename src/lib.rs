//! The most complete serialization tree for [`serde`].
//!
//! [`Save`] represents the entire [serde data model](https://serde.rs/data-model.html),
//! including [struct names](Save::Struct::name), [field names](Save::Struct::fields),
//! and [enum variant information](Variant).
//!
//! [`Save`] can optionally persist errors _in the serialization tree_ instead of short-circuiting.
//!
//!
//!

use core::{convert::Infallible, fmt};
use core::{iter, marker::PhantomData};

use ser::Error;
use serde::{
    ser::{
        Error as _, SerializeMap as _, SerializeStruct as _, SerializeStructVariant as _,
        SerializeTuple as _, SerializeTupleStruct as _, SerializeTupleVariant as _,
    },
    Deserialize, Serialize,
};

pub mod ser;

/// A complete [`serde`] serialization tree.
///
/// Accepts a lifetime to allow users to write dynamic tests.
///
/// See [`crate documentation`](mod@self) for more.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Save<'a, E = Infallible> {
    /// Primitive type, from a call to [`serde::Serializer::serialize_bool`].
    Bool(bool),
    /// Primitive type, from a call to [`serde::Serializer::serialize_i8`].
    I8(i8),
    /// Primitive type, from a call to [`serde::Serializer::serialize_i16`].
    I16(i16),
    /// Primitive type, from a call to [`serde::Serializer::serialize_i32`].
    I32(i32),
    /// Primitive type, from a call to [`serde::Serializer::serialize_i64`].
    I64(i64),
    /// Primitive type, from a call to [`serde::Serializer::serialize_i128`].
    I128(i128),
    /// Primitive type, from a call to [`serde::Serializer::serialize_u8`].
    U8(u8),
    /// Primitive type, from a call to [`serde::Serializer::serialize_u16`].
    U16(u16),
    /// Primitive type, from a call to [`serde::Serializer::serialize_u32`].
    U32(u32),
    /// Primitive type, from a call to [`serde::Serializer::serialize_u64`].
    U64(u64),
    /// Primitive type, from a call to [`serde::Serializer::serialize_u128`].
    U128(u128),
    /// Primitive type, from a call to [`serde::Serializer::serialize_f32`].
    F32(f32),
    /// Primitive type, from a call to [`serde::Serializer::serialize_f64`].
    F64(f64),
    /// Primitive type, from a call to [`serde::Serializer::serialize_char`].
    Char(char),

    /// A call to [`serde::Serializer::serialize_str`].
    String(String),
    /// A call to [`serde::Serializer::serialize_bytes`].
    ByteArray(Vec<u8>),
    /// A call to [`serde::Serializer::serialize_some`] or [`serde::Serializer::serialize_none`].
    Option(Option<Box<Self>>),

    /// The empty tuple, from a call to [`serde::Serializer::serialize_unit`].
    Unit,
    /// A unit struct, from a call to [`serde::Serializer::serialize_unit_struct`].
    /// ```
    /// struct MyUnitStruct;
    /// ```
    UnitStruct(&'a str),
    /// A unit variant of an enum, from a call to [`serde::Serializer::serialize_unit_variant`].
    /// ```
    /// enum MyEnum {
    ///     MyUnitVariant,
    ///     // ...
    /// }
    /// ```
    UnitVariant(Variant<'a>),

    /// A tuple struct with a single unnamed field, from a call to [`serde::Serializer::serialize_newtype_struct`].
    /// ```
    /// # struct A;
    /// struct MyStruct(A)
    /// ```
    NewTypeStruct {
        name: &'a str,
        value: Box<Self>,
    },
    /// A tuple variant of an enum with a single unnamed field, from a call to [`serde::Serializer::serialize_newtype_variant`].
    /// ```
    /// # struct A;
    /// enum MyEnum {
    ///     MyNewTypeVariant(A),
    ///     // ...
    /// }
    /// ```
    NewTypeVariant {
        variant: Variant<'a>,
        value: Box<Self>,
    },

    /// A dynamic sequence of values, from a call to [`serde::Serializer::serialize_seq`].
    ///
    /// If [protocol errors] are enabled, checks that the number of items matches
    /// the length (if any) passed to the call to `serialize_seq`.
    ///
    /// [protocol errors]: ser::Serializer::check_for_protocol_errors
    Seq(Vec<Self>),
    /// A dynamic mapping between values, from a call to [`serde::Serializer::serialize_map`].
    ///
    /// If [protocol errors] are enabled, checks that:
    /// - the number of items matches the length (if any) passed to the call to `serialize_map`.
    /// - there are no orphaned keys or values.
    ///
    /// [protocol errors]: ser::Serializer::check_for_protocol_errors
    Map(Vec<(Self, Self)>),

    /// A fixed sequence of values, from a call to [`serde::Serializer::serialize_tuple`].
    ///
    /// ```
    /// # struct A; struct B; struct C;
    /// (A, B, C)
    /// ```
    ///
    /// If [protocol errors] are enabled, checks that the number of items matches the length passed to the call.
    ///
    /// [protocol errors]: ser::Serializer::check_for_protocol_errors
    Tuple(Vec<Self>),
    /// A fixed sequence of unnamed fields in a struct, from a call to [`serde::Serializer::serialize_tuple_struct`].
    ///
    /// ```
    /// # struct A; struct B; struct C;
    /// struct MyTupleStruct(A, B, C);
    /// ```
    ///
    /// If [protocol errors] are enabled, checks that the number of items matches the length passed to the call.
    ///
    /// [protocol errors]: ser::Serializer::check_for_protocol_errors
    TupleStruct {
        name: &'a str,
        values: Vec<Self>,
    },
    /// A fixed sequence of unnamed fields in an enum variant, from a call to [`serde::Serializer::serialize_tuple_variant`].
    /// ```
    /// # struct A; struct B; struct C;
    /// enum MyEnum {
    ///     MyTupleVariant(A, B, C),
    ///     // ...
    /// }
    /// ```
    /// If [protocol errors] are enabled, checks that the number of items matches the length passed to the call.
    ///
    /// [protocol errors]: ser::Serializer::check_for_protocol_errors
    TupleVariant {
        variant: Variant<'a>,
        values: Vec<Self>,
    },

    /// A fixed mapping from field names to values in a struct, from a call to [`serde::Serializer::serialize_struct`].
    /// ```
    /// struct MyStruct {
    ///     num_yaks: usize,
    ///     shepherd_name: String,
    /// }
    /// ```
    /// If [protocol errors] are enabled, checks that:
    /// - the number of items matches the length passed to the call.
    /// - all fields are unique
    ///
    /// [protocol errors]: ser::Serializer::check_for_protocol_errors
    Struct {
        name: &'a str,
        /// RHS is [`None`] for [skip](`serde::ser::SerializeStruct::skip_field`)ed fields.
        ///
        /// For in-tree errors, the field name is `"!error"`.
        fields: Vec<(&'static str, Option<Self>)>,
    },
    /// A fixed mapping from named fields to values in an enum variant, from a call to [`serde::Serializer::serialize_struct_variant`].
    /// ```
    /// enum MyEnum {
    ///     MyStructVariant {
    ///         num_yaks: usize,
    ///         shepherd_name: String,
    ///     },
    ///     // ...
    /// }
    /// ```
    /// If [protocol errors] are enabled, checks that:
    /// - the number of items matches the length passed to the call.
    /// - all fields are unique
    ///
    /// [protocol errors]: ser::Serializer::check_for_protocol_errors
    StructVariant {
        variant: Variant<'a>,
        /// RHS is [`None`] for [skip](`serde::ser::SerializeStructVariant::skip_field`)ed fields.
        ///
        /// For in-tree errors, the field name is `"!error"`.
        fields: Vec<(&'a str, Option<Self>)>,
    },

    Error(E),
}

impl<'a> Save<'a, Error> {
    pub fn error(msg: impl fmt::Display) -> Self {
        Self::Error(Error::custom(msg))
    }
}

/// Save the serialization tree, returning an [`Err`] if:
/// - Any node's call to [`serde::Serialize::serialize`] fails.
/// - Any node has any [protocol errors].
///
/// [protocol errors]: ser::Serializer::check_for_protocol_errors
pub fn save<T: Serialize>(t: T) -> Result<Save<'static>, ser::Error> {
    t.serialize(ser::Serializer::new())
}

#[must_use]
pub fn save_errors<T: Serialize>(t: T) -> Save<'static, ser::Error> {
    t.serialize(ser::Serializer::new().save_errors())
        .unwrap_or_else(Save::Error)
}

/// Information about a serialized `enum` variant.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variant<'a> {
    /// The name of the outer `enum`.
    pub name: &'a str,
    /// The index of this variant within the outer `enum`.
    pub variant_index: u32,
    /// The name of the inhabited variant within the outer `enum`
    pub variant: &'a str,
}

impl<E> Serialize for Save<'static, E>
where
    E: fmt::Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Save::Bool(it) => serializer.serialize_bool(*it),
            Save::I8(it) => serializer.serialize_i8(*it),
            Save::I16(it) => serializer.serialize_i16(*it),
            Save::I32(it) => serializer.serialize_i32(*it),
            Save::I64(it) => serializer.serialize_i64(*it),
            Save::I128(it) => serializer.serialize_i128(*it),
            Save::U8(it) => serializer.serialize_u8(*it),
            Save::U16(it) => serializer.serialize_u16(*it),
            Save::U32(it) => serializer.serialize_u32(*it),
            Save::U64(it) => serializer.serialize_u64(*it),
            Save::U128(it) => serializer.serialize_u128(*it),
            Save::F32(it) => serializer.serialize_f32(*it),
            Save::F64(it) => serializer.serialize_f64(*it),
            Save::Char(it) => serializer.serialize_char(*it),
            Save::String(it) => serializer.serialize_str(it),
            Save::ByteArray(it) => serializer.serialize_bytes(it),
            Save::Option(None) => serializer.serialize_none(),
            Save::Option(Some(it)) => serializer.serialize_some(it),
            Save::UnitStruct(it) => serializer.serialize_unit_struct(it),
            Save::UnitVariant(Variant {
                name,
                variant_index,
                variant,
            }) => serializer.serialize_unit_variant(name, *variant_index, variant),
            Save::Unit => serializer.serialize_unit(),
            Save::NewTypeStruct { name, value } => serializer.serialize_newtype_struct(name, value),
            Save::NewTypeVariant {
                variant:
                    Variant {
                        name,
                        variant_index,
                        variant,
                    },
                value,
            } => serializer.serialize_newtype_variant(name, *variant_index, variant, value),
            Save::Seq(it) => it.serialize(serializer),
            Save::Map(it) => {
                let mut map = serializer.serialize_map(Some(it.len()))?;
                for (k, v) in it {
                    map.serialize_entry(k, v)?
                }
                map.end()
            }
            Save::Tuple(it) => {
                let mut tup = serializer.serialize_tuple(it.len())?;
                for it in it {
                    tup.serialize_element(it)?
                }
                tup.end()
            }
            Save::TupleStruct { name, values } => {
                let mut tup = serializer.serialize_tuple_struct(name, values.len())?;
                for it in values {
                    tup.serialize_field(it)?
                }
                tup.end()
            }
            Save::TupleVariant {
                variant:
                    Variant {
                        name,
                        variant_index,
                        variant,
                    },
                values,
            } => {
                let mut var = serializer.serialize_tuple_variant(
                    name,
                    *variant_index,
                    variant,
                    values.len(),
                )?;
                for it in values {
                    var.serialize_field(it)?
                }
                var.end()
            }
            Save::Struct { name, fields } => {
                let mut strukt = serializer.serialize_struct(name, fields.len())?;
                for (k, v) in fields {
                    match v {
                        Some(v) => strukt.serialize_field(k, v)?,
                        None => strukt.skip_field(k)?,
                    }
                }
                strukt.end()
            }
            Save::StructVariant {
                variant:
                    Variant {
                        name,
                        variant_index,
                        variant,
                    },
                fields,
            } => {
                let mut var = serializer.serialize_struct_variant(
                    name,
                    *variant_index,
                    variant,
                    fields.len(),
                )?;
                for (k, v) in fields {
                    match v {
                        Some(v) => var.serialize_field(k, v)?,
                        None => var.skip_field(k)?,
                    }
                }
                var.end()
            }
            Save::Error(e) => Err(S::Error::custom(e)),
        }
    }
}

impl<'a, 'de> Deserialize<'de> for Save<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'a>(PhantomData<&'a ()>);

        macro_rules! simple {
            ($($fn:ident($ty:ty) -> $variant:ident);* $(;)?) => {
                $(
                    fn $fn<E: serde::de::Error>(self, v: $ty) -> Result<Self::Value, E> {
                        Ok(Save::$variant(v))
                    }
                )*
            };
        }
        impl<'a, 'de> serde::de::Visitor<'de> for Visitor<'a> {
            type Value = Save<'a>;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a `Save`-able type")
            }

            simple! {
                visit_bool(bool) -> Bool;
                visit_i8(i8) -> I8;
                visit_i16(i16) -> I16;
                visit_i32(i32) -> I32;
                visit_i64(i64) -> I64;
                visit_i128(i128) -> I128;
                visit_u8(u8) -> U8;
                visit_u16(u16) -> U16;
                visit_u32(u32) -> U32;
                visit_u64(u64) -> U64;
                visit_u128(u128) -> U128;
                visit_f32(f32) -> F32;
                visit_f64(f64) -> F64;
                visit_char(char) -> Char;
                visit_string(String) -> String;
                visit_byte_buf(Vec<u8>) -> ByteArray;
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Save::String(v.into()))
            }

            fn visit_borrowed_str<E: serde::de::Error>(
                self,
                v: &'de str,
            ) -> Result<Self::Value, E> {
                Ok(Save::String(v.into()))
            }

            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(Save::ByteArray(v.into()))
            }

            fn visit_borrowed_bytes<E: serde::de::Error>(
                self,
                v: &'de [u8],
            ) -> Result<Self::Value, E> {
                Ok(Save::ByteArray(v.into()))
            }

            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(Save::Option(None))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Ok(Save::Option(Some(Box::new(
                    deserializer.deserialize_any(self)?,
                ))))
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(Save::Unit)
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let _ = deserializer;
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::NewtypeStruct,
                    &self,
                ))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                Ok(Save::Seq(
                    iter::from_fn(|| seq.next_element().transpose())
                        .fuse()
                        .collect::<Result<_, _>>()?,
                ))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                Ok(Save::Map(
                    iter::from_fn(|| map.next_entry().transpose())
                        .fuse()
                        .collect::<Result<_, _>>()?,
                ))
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::EnumAccess<'de>,
            {
                let _ = data;
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Enum,
                    &self,
                ))
            }
        }
        deserializer.deserialize_any(Visitor(PhantomData))
    }
}
