//! The most complete serialization tree for [`serde`].
//!
//! [`Save`] represents the entire [serde data model](https://serde.rs/data-model.html),
//! including [struct names](Save::Struct::name), [field names](Save::Struct::fields),
//! and [enum variant information](Variant).
//! This means that it can intercept structures when they are serialized, before
//! losslessly forwarding them.
//!
//! [`Save`] can optionally [persist errors](save_errors) _in the serialization tree_,
//! instead of short-circuiting.
//! This is a zero-cost option - see documentation on [`Save::Error`] for more.
//! ```
//! # use std::time::{Duration, SystemTime};
//! # use std::{ffi::OsString, os::unix::ffi::OsStringExt as _, path::PathBuf};
//! # use serde::Serialize;
//! # use serde_save::{Save, save, save_errors};
//! #[derive(Serialize)]
//! struct MyStruct {
//!     system_time: SystemTime,
//!     path_buf: PathBuf,
//!     normal_string: String,
//! }
//!
//! // These will fail to serialize
//! let before_unix_epoch = SystemTime::UNIX_EPOCH - Duration::from_secs(1);
//! let non_utf8_path = PathBuf::from(OsString::from_vec(vec![u8::MAX]));
//!
//! let my_struct = MyStruct {
//!     system_time: before_unix_epoch,
//!     path_buf: non_utf8_path,
//!     normal_string: String::from("this is a string"), // this is fine
//! };
//!
//! // By default errors are short-circuiting
//! assert_eq!(
//!     save(&my_struct).unwrap_err().to_string(),
//!     "SystemTime must be later than UNIX_EPOCH"
//! );
//!
//! // But you can persist and inspect them in-tree if you prefer.
//! assert_eq!(
//!     save_errors(&my_struct), // use this method instead
//!     Save::strukt(
//!         "MyStruct",
//!         [
//!             ("system_time",   Save::error("SystemTime must be later than UNIX_EPOCH")),
//!             ("path_buf",      Save::error("path contains invalid UTF-8 characters")),
//!             ("normal_string", Save::string("this is a string"),
//!         ]
//!     )
//! )
//! ```
//!
//! By default, [`save_errors`] and [`save`] also check for incorrect implementations
//! of the serde protocol.
//! See the documentation on [`Save`]s variants to see which invariants are checked.
//! You can [disable this behaviour](Serializer::check_for_protocol_errors) if you
//! wish.

mod imp;

pub use imp::Serializer;

use core::{convert::Infallible, fmt};
use core::{iter, marker::PhantomData};

use serde::{
    ser::{
        Error as _, SerializeMap as _, SerializeStruct as _, SerializeStructVariant as _,
        SerializeTuple as _, SerializeTupleStruct as _, SerializeTupleVariant as _,
    },
    Deserialize, Serialize,
};

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
    /// struct MyStruct(A);
    /// ```
    NewTypeStruct { name: &'a str, value: Box<Self> },
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
    /// [protocol errors]: Serializer::check_for_protocol_errors
    Seq(Vec<Self>),
    /// A dynamic mapping between values, from a call to [`serde::Serializer::serialize_map`].
    ///
    /// If [protocol errors] are enabled, checks that:
    /// - the number of items matches the length (if any) passed to the call to `serialize_map`.
    /// - there are no orphaned keys or values.
    ///
    /// Note that duplicate map keys are always allowed.
    ///
    /// [protocol errors]: Serializer::check_for_protocol_errors
    Map(Vec<(Self, Self)>),

    /// A fixed sequence of values, from a call to [`serde::Serializer::serialize_tuple`].
    ///
    /// ```
    /// # struct A; struct B; struct C;
    /// (A, B, C);
    /// ```
    ///
    /// If [protocol errors] are enabled, checks that the number of items matches the length passed to the call.
    ///
    /// [protocol errors]: Serializer::check_for_protocol_errors
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
    /// [protocol errors]: Serializer::check_for_protocol_errors
    TupleStruct { name: &'a str, values: Vec<Self> },
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
    /// [protocol errors]: Serializer::check_for_protocol_errors
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
    /// [protocol errors]: Serializer::check_for_protocol_errors
    Struct {
        name: &'a str,
        /// RHS is [`None`] for [skip](`serde::ser::SerializeStruct::skip_field`)ed fields.
        ///
        /// For in-tree errors, the field name is `"!error"`.
        fields: Vec<(&'a str, Option<Self>)>,
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
    /// [protocol errors]: Serializer::check_for_protocol_errors
    StructVariant {
        variant: Variant<'a>,
        /// RHS is [`None`] for [skip](`serde::ser::SerializeStructVariant::skip_field`)ed fields.
        ///
        /// For in-tree errors, the field name is `"!error"`.
        fields: Vec<(&'a str, Option<Self>)>,
    },

    /// An in-tree persisted error.
    ///
    /// Note that this is _uninhabited_ by default, and you can prove it to be
    /// unreachable in your code:
    ///
    /// ```no_run
    /// # use serde_save::Save;
    ///
    /// fn stringify(save: Save) -> String {
    ///     match save {
    ///         // the compiler knows this branch won't be hit, so coerced the
    ///         // empty match to String
    ///         Save::Error(e) => match e {},
    ///         // ...
    ///         # _ => todo!(),
    ///     }
    /// }
    /// ```
    ///
    /// However, if [errors are persisted](save_errors), you can inspect them
    /// ```no_run
    /// # use serde_save::{Save, Error};
    /// let save: Save<Error>;
    /// # let save: Save<Error> = todo!();
    /// match save {
    ///     Save::Error(e) => {
    ///         println!("{}", e);
    ///         if e.is_protocol() { /* .. */ }
    ///     }
    ///     // ...
    ///     # _ => todo!(),
    /// }
    /// ```
    Error(E),
}

impl<'a> Save<'a, Error> {
    /// Convenience method for creating a custom error.
    pub fn error(msg: impl fmt::Display) -> Self {
        Self::Error(Error::custom(msg))
    }
}

impl<'a, E> Save<'a, E> {
    /// Convenience method for creating a [`Save::Struct`] with no skipped fields.
    pub fn strukt<V>(name: &'a str, fields: impl IntoIterator<Item = (&'a str, V)>) -> Self
    where
        V: Into<Save<'a, E>>,
    {
        Self::Struct {
            name,
            fields: fields
                .into_iter()
                .map(|(k, v)| (k, Some(v.into())))
                .collect(),
        }
    }
    /// Convenience method for creating a [`Save::String`]
    pub fn string(it: impl Into<String>) -> Self {
        Self::String(it.into())
    }
    /// Convenience method for creating a [`Save::ByteArray`]
    pub fn bytes(it: impl Into<Vec<u8>>) -> Self {
        Self::ByteArray(it.into())
    }
}

/// Save the serialization tree, returning an [`Err`] if:
/// - Any node's call to [`serde::Serialize::serialize`] fails.
/// - Any node has any [protocol errors].
///
/// [protocol errors]: Serializer::check_for_protocol_errors
pub fn save<T: Serialize>(t: T) -> Result<Save<'static>, Error> {
    t.serialize(Serializer::new())
}

/// Save the serialization tree, annotating it with [`Save::Error`] if:
/// - Any node's call to [`serde::Serialize::serialize`] fails.
/// - Any node has any [protocol errors].
///
/// [protocol errors]: Serializer::check_for_protocol_errors
#[must_use]
pub fn save_errors<T: Serialize>(t: T) -> Save<'static, Error> {
    t.serialize(Serializer::new().save_errors())
        .unwrap_or_else(Save::Error)
}

/// An error returned by an implementation of [`serde::Serialize::serialize`], or
/// [protocol error] checking.
///
/// [protocol error]: Serializer::check_for_protocol_errors
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Error {
    msg: String,
    protocol: bool,
}

impl Error {
    /// Returns `true` if these error was caused by an incorrect implementation
    /// of the [`serde`] methods.
    ///
    /// See documentation on [`Save`]'s variants for the invariants that are checked.
    pub fn is_protocol(&self) -> bool {
        self.protocol
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self {
            msg: msg.to_string(),
            protocol: false,
        }
    }
}

impl std::error::Error for Error {}

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

macro_rules! from {
    ($($variant:ident($ty:ty)),* $(,)?) => {
        $(
            impl<'a, E> From<$ty> for Save<'a, E> {
                fn from(it: $ty) -> Self {
                    Self::$variant(it)
                }
            }
        )*
    };
}

from! {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    F32(f32),
    F64(f64),
    Char(char),
    String(String),
    ByteArray(Vec<u8>),
    UnitVariant(Variant<'a>),
}

impl<'a, E> From<()> for Save<'a, E> {
    fn from(_: ()) -> Self {
        Self::Unit
    }
}
impl<'a, E, T> From<Option<T>> for Save<'a, E>
where
    T: Into<Save<'a, E>>,
{
    fn from(it: Option<T>) -> Self {
        Self::Option(it.map(Into::into).map(Box::new))
    }
}

impl<'a, E, T> FromIterator<T> for Save<'a, E>
where
    T: Into<Save<'a, E>>,
{
    fn from_iter<II: IntoIterator<Item = T>>(iter: II) -> Self {
        Self::Seq(iter.into_iter().map(Into::into).collect())
    }
}

impl<'a, E, K, V> FromIterator<(K, V)> for Save<'a, E>
where
    K: Into<Save<'a, E>>,
    V: Into<Save<'a, E>>,
{
    fn from_iter<II: IntoIterator<Item = (K, V)>>(iter: II) -> Self {
        Self::Map(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

macro_rules! from_tuple {
    ($($ident:ident),* $(,)?) => {
        #[doc(hidden)]
        #[allow(non_snake_case)]
        impl<'a, E, $($ident),*> From<($($ident,)*)> for Save<'a, E>
        where
            $($ident: Into<Save<'a, E>>,)*
        {
            fn from(($($ident,)*): ($($ident,)*)) -> Self {
                Self::Tuple([
                    $($ident.into()),*
                ].into())
            }
        }
    };
}

/// You can construct a [`Save::Tuple`] using [`From`] for tuples of arities
/// between 1 and 24, _except_ 2.
///
/// The other implementations are hidden from rustdoc for brevity.
impl<'a, E, T0, T1, T2> From<(T0, T1, T2)> for Save<'a, E>
where
    T0: Into<Save<'a, E>>,
    T1: Into<Save<'a, E>>,
    T2: Into<Save<'a, E>>,
{
    fn from((t0, t1, t2): (T0, T1, T2)) -> Self {
        Self::Tuple([t0.into(), t1.into(), t2.into()].into())
    }
}

from_tuple!(T0);
// from_tuple!(T0, T1); // conflicting
// from_tuple!(T0, T1, T2); // document it
from_tuple!(T0, T1, T2, T3);
from_tuple!(T0, T1, T2, T3, T4);
from_tuple!(T0, T1, T2, T3, T4, T5);
from_tuple!(T0, T1, T2, T3, T4, T5, T6);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17);
from_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18);
from_tuple!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19
);
from_tuple!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20
);
from_tuple!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21
);
from_tuple!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22
);
from_tuple!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23
);

/// If [`protocol errors`](Serializer::check_for_protocol_errors) are disabled,
/// this will perfectly preserve the underlying structure of the originally
/// saved item.
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

/// This is a best-effort deserialization, provided for completeness.
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
