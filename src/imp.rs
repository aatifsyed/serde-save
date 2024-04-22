use crate::{Error, Save, Variant};
use core::{cmp, convert::Infallible, fmt, marker::PhantomData};
use std::collections::BTreeSet;

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::ShortCircuit {}
    impl Sealed for super::Persist {}
}

pub trait ErrorDiscipline: sealed::Sealed {
    type SaveError;
    fn handle(res: Result<Save<Self::SaveError>, Error>) -> Result<Save<Self::SaveError>, Error>;
}

pub enum ShortCircuit {}
pub enum Persist {}

impl ErrorDiscipline for ShortCircuit {
    type SaveError = Infallible;
    fn handle(res: Result<Save<Self::SaveError>, Error>) -> Result<Save<Self::SaveError>, Error> {
        res
    }
}

impl ErrorDiscipline for Persist {
    type SaveError = Error;
    fn handle(res: Result<Save<Self::SaveError>, Error>) -> Result<Save<Self::SaveError>, Error> {
        Ok(res.unwrap_or_else(Save::Error))
    }
}

/// Serializer which produces [`Save`]s.
///
/// See [crate documentation](mod@super) for more.
pub struct Serializer<ErrorDiscipline = ShortCircuit> {
    config: Config<ErrorDiscipline>,
}

impl Serializer<ShortCircuit> {
    /// Create a serializer which is:
    /// - [human readable](`serde::Serializer::is_human_readable`) (this is the default for serde formats).
    /// - sensitive to [protocol errors](Self::check_for_protocol_errors).
    pub fn new() -> Self {
        Self {
            config: Config {
                is_human_readable: true,
                protocol_errors: true,
                _error_discipline: PhantomData,
            },
        }
    }
}

impl<E> Serializer<E> {
    /// See [`serde::Serializer::is_human_readable`].
    pub fn human_readable(mut self, is_human_readable: bool) -> Self {
        self.config.is_human_readable = is_human_readable;
        self
    }
    /// Whether to check for incorrect implementations of e.g [`serde::ser::SerializeSeq`].
    /// See documentation on variants of [`Save`] for the invariants which are checked.
    pub fn check_for_protocol_errors(mut self, check: bool) -> Self {
        self.config.protocol_errors = check;
        self
    }
    /// Persist the errors in-tree.
    ///
    /// If any node's implementation of [`serde::Serialize::serialize`] fails, it
    /// will be recorded as a [`Save::Error`].
    ///
    /// If there are any [protocol errors](Self::check_for_protocol_errors), they
    /// will be recorded as the final element(s) of the corresponding collection.
    pub fn save_errors(self) -> Serializer<Persist> {
        let Self {
            config:
                Config {
                    is_human_readable,
                    protocol_errors,
                    _error_discipline,
                },
        } = self;
        Serializer {
            config: Config {
                is_human_readable,
                protocol_errors,
                _error_discipline: PhantomData,
            },
        }
    }
}

impl Default for Serializer {
    /// See [`Self::new`].
    fn default() -> Self {
        Self::new()
    }
}

struct Config<E = ShortCircuit> {
    is_human_readable: bool,
    protocol_errors: bool,
    _error_discipline: PhantomData<fn() -> E>,
}

impl<E> Clone for Config<E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E> Copy for Config<E> {}

macro_rules! simple {
    ($($method:ident($ty:ty) -> $variant:ident);* $(;)?) => {
        $(
            fn $method(self, v: $ty) -> Result<Self::Ok, Self::Error> {
                Ok(Save::$variant(v))
            }
        )*
    };
}

impl<E> serde::Serializer for Serializer<E>
where
    E: ErrorDiscipline,
{
    type Ok = Save<'static, E::SaveError>;
    type Error = Error;
    type SerializeSeq = SerializeSeq<E>;
    type SerializeTuple = SerializeTuple<E>;
    type SerializeTupleStruct = SerializeTupleStruct<E>;
    type SerializeTupleVariant = SerializeTupleVariant<E>;
    type SerializeMap = SerializeMap<E>;
    type SerializeStruct = SerializeStruct<E>;
    type SerializeStructVariant = SerializeStructVariant<E>;

    fn is_human_readable(&self) -> bool {
        self.config.is_human_readable
    }

    simple! {
        serialize_bool(bool) -> Bool;
        serialize_i8(i8) -> I8;
        serialize_i16(i16) -> I16;
        serialize_i32(i32) -> I32;
        serialize_i64(i64) -> I64;
        serialize_u8(u8) -> U8;
        serialize_u16(u16) -> U16;
        serialize_u32(u32) -> U32;
        serialize_u64(u64) -> U64;
        serialize_f32(f32) -> F32;
        serialize_f64(f64) -> F64;
        serialize_char(char) -> Char;
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Save::String(v.into()))
    }
    fn collect_str<T: ?Sized + fmt::Display>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        Ok(Save::String(value.to_string()))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Save::ByteArray(v.into()))
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Save::Option(None))
    }
    fn serialize_some<T: ?Sized + serde::Serialize>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Save::Option(Some(Box::new(E::handle(
            value.serialize(self),
        )?))))
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Save::Unit)
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(Save::UnitStruct(name))
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Save::UnitVariant(Variant {
            name,
            variant_index,
            variant,
        }))
    }
    fn serialize_newtype_struct<T: ?Sized + serde::Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Save::NewTypeStruct {
            name,
            value: Box::new(E::handle(value.serialize(self))?),
        })
    }
    fn serialize_newtype_variant<T: ?Sized + serde::Serialize>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Save::NewTypeVariant {
            variant: Variant {
                name,
                variant_index,
                variant,
            },
            value: Box::new(E::handle(value.serialize(self))?),
        })
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeSeq {
            config: self.config,
            inner: Vec::with_capacity(len.unwrap_or_default()),
            expected_len: len,
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SerializeTuple {
            config: self.config,
            inner: Vec::with_capacity(len),
            expected_len: len,
        })
    }
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SerializeTupleStruct {
            expected_len: len,
            config: self.config,
            name,
            values: Vec::with_capacity(len),
        })
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SerializeTupleVariant {
            expected_len: len,
            config: self.config,
            variant: Variant {
                name,
                variant_index,
                variant,
            },
            values: Vec::with_capacity(len),
        })
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let capacity = len.unwrap_or_default();
        Ok(SerializeMap {
            config: self.config,
            expected_len: len,
            keys: Vec::with_capacity(capacity),
            values: Vec::with_capacity(capacity),
        })
    }
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeStruct {
            expected_len: len,
            config: self.config,
            name,
            fields: Vec::with_capacity(len),
        })
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(SerializeStructVariant {
            config: self.config,
            variant: Variant {
                name,
                variant_index,
                variant,
            },
            fields: Vec::with_capacity(len),
            expected_len: len,
        })
    }
}

fn check_length<E>(
    what: &str,
    config: &Config<E>,
    expected: usize,
    pushing: &mut Vec<Save<'static, E::SaveError>>,
) -> Result<(), Error>
where
    E: ErrorDiscipline,
{
    if config.protocol_errors {
        let actual = pushing.len();
        if expected != actual {
            let e = Error {
                msg: format!(
                    "protocol error: expected a {} of length {}, got {}",
                    what, expected, actual
                ),
                protocol: true,
            };
            pushing.push(E::handle(Err(e))?)
        }
    }
    Ok(())
}

pub struct SerializeSeq<E: ErrorDiscipline> {
    config: Config<E>,
    expected_len: Option<usize>,
    inner: Vec<Save<'static, E::SaveError>>,
}
impl<E> serde::ser::SerializeSeq for SerializeSeq<E>
where
    E: ErrorDiscipline,
{
    type Ok = Save<'static, E::SaveError>;
    type Error = Error;
    fn serialize_element<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.inner.push(E::handle(value.serialize(Serializer {
            config: self.config,
        }))?);
        Ok(())
    }
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        if let Some(expected_len) = self.expected_len {
            check_length("sequence", &self.config, expected_len, &mut self.inner)?;
        }
        Ok(Save::Seq(self.inner))
    }
}
pub struct SerializeTuple<E: ErrorDiscipline> {
    expected_len: usize,
    config: Config<E>,
    inner: Vec<Save<'static, E::SaveError>>,
}
impl<E> serde::ser::SerializeTuple for SerializeTuple<E>
where
    E: ErrorDiscipline,
{
    type Ok = Save<'static, E::SaveError>;
    type Error = Error;
    fn serialize_element<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.inner.push(E::handle(value.serialize(Serializer {
            config: self.config,
        }))?);
        Ok(())
    }
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        check_length("tuple", &self.config, self.expected_len, &mut self.inner)?;
        Ok(Save::Tuple(self.inner))
    }
}
pub struct SerializeTupleStruct<E: ErrorDiscipline> {
    expected_len: usize,
    config: Config<E>,
    name: &'static str,
    values: Vec<Save<'static, E::SaveError>>,
}
impl<E> serde::ser::SerializeTupleStruct for SerializeTupleStruct<E>
where
    E: ErrorDiscipline,
{
    type Ok = Save<'static, E::SaveError>;
    type Error = Error;
    fn serialize_field<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.values.push(E::handle(value.serialize(Serializer {
            config: self.config,
        }))?);
        Ok(())
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        check_length(
            "tuple struct",
            &self.config,
            self.expected_len,
            &mut self.values,
        )?;
        Ok(Save::TupleStruct {
            name: self.name,
            values: self.values,
        })
    }
}
pub struct SerializeTupleVariant<E: ErrorDiscipline> {
    expected_len: usize,
    config: Config<E>,
    variant: Variant<'static>,
    values: Vec<Save<'static, E::SaveError>>,
}
impl<E> serde::ser::SerializeTupleVariant for SerializeTupleVariant<E>
where
    E: ErrorDiscipline,
{
    type Ok = Save<'static, E::SaveError>;
    type Error = Error;
    fn serialize_field<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.values.push(E::handle(value.serialize(Serializer {
            config: self.config,
        }))?);
        Ok(())
    }
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        check_length(
            "tuple variant",
            &self.config,
            self.expected_len,
            &mut self.values,
        )?;

        Ok(Save::TupleVariant {
            variant: self.variant,
            values: self.values,
        })
    }
}
pub struct SerializeMap<E: ErrorDiscipline> {
    expected_len: Option<usize>,
    config: Config<E>,
    keys: Vec<Save<'static, E::SaveError>>,
    values: Vec<Save<'static, E::SaveError>>,
}
impl<E> serde::ser::SerializeMap for SerializeMap<E>
where
    E: ErrorDiscipline,
{
    type Ok = Save<'static, E::SaveError>;
    type Error = Error;
    fn serialize_key<T: ?Sized + serde::Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.keys.push(E::handle(key.serialize(Serializer {
            config: self.config,
        }))?);
        Ok(())
    }
    fn serialize_value<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.values.push(E::handle(value.serialize(Serializer {
            config: self.config,
        }))?);
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        let n_keys = self.keys.len();
        let n_values = self.values.len();
        let mut map = Vec::with_capacity(cmp::max(n_keys, n_values));
        let mut keys = self.keys.into_iter();
        let mut values = self.values.into_iter();
        loop {
            let e = || Error {
                msg: format!(
                    "protocol error: map has {} keys and {} values",
                    n_keys, n_values
                ),
                protocol: true,
            };
            match (keys.next(), values.next()) {
                (None, None) => {
                    if let Some(expected) = self.expected_len {
                        if self.config.protocol_errors && expected != map.len() {
                            let e = || Error {
                                msg: format!(
                                    "protocol error: expected a map of length {}, got {}",
                                    expected,
                                    map.len()
                                ),
                                protocol: true,
                            };
                            map.push((E::handle(Err(e()))?, E::handle(Err(e()))?))
                        }
                    }
                    return Ok(Save::Map(map));
                }
                (Some(key), Some(value)) => map.push((key, value)),
                (None, Some(value)) => map.push((E::handle(Err(e()))?, value)),
                (Some(key), None) => map.push((key, E::handle(Err(e()))?)),
            }
        }
    }
}

fn check<E>(
    what: &str,
    config: &Config<E>,
    expected_len: usize,
    fields: &mut Vec<(&'static str, Option<Save<'static, E::SaveError>>)>,
) -> Result<(), Error>
where
    E: ErrorDiscipline,
{
    if config.protocol_errors {
        let mut seen = BTreeSet::new();
        let mut dups = Vec::new();
        for name in fields.iter().map(|(it, _)| it) {
            let new = seen.insert(*name);
            if !new {
                dups.push(*name)
            }
        }
        if !dups.is_empty() {
            let e = Error {
                msg: format!(
                    "protocol error: {} has duplicate field names: {}",
                    what,
                    dups.join(", ")
                ),
                protocol: true,
            };
            fields.push(("!error", Some(E::handle(Err(e))?)))
        }

        let actual = fields.len();
        if expected_len != actual {
            let e = Error {
                msg: format!(
                    "protocol error: expected a {} of length {}, got {}",
                    what, expected_len, actual
                ),
                protocol: true,
            };
            fields.push(("!error", Some(E::handle(Err(e))?)))
        }
    }
    Ok(())
}

pub struct SerializeStruct<E: ErrorDiscipline> {
    expected_len: usize,
    config: Config<E>,
    name: &'static str,
    fields: Vec<(&'static str, Option<Save<'static, E::SaveError>>)>,
}
impl<E> serde::ser::SerializeStruct for SerializeStruct<E>
where
    E: ErrorDiscipline,
{
    type Ok = Save<'static, E::SaveError>;
    type Error = Error;
    fn serialize_field<T: ?Sized + serde::Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.fields.push((
            key,
            Some(E::handle(value.serialize(Serializer {
                config: self.config,
            }))?),
        ));
        Ok(())
    }
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        check("struct", &self.config, self.expected_len, &mut self.fields)?;
        Ok(Save::Struct {
            name: self.name,
            fields: self.fields,
        })
    }
    fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
        self.fields.push((key, None));
        Ok(())
    }
}
pub struct SerializeStructVariant<E: ErrorDiscipline> {
    expected_len: usize,
    config: Config<E>,
    variant: Variant<'static>,
    fields: Vec<(&'static str, Option<Save<'static, E::SaveError>>)>,
}
impl<E> serde::ser::SerializeStructVariant for SerializeStructVariant<E>
where
    E: ErrorDiscipline,
{
    type Ok = Save<'static, E::SaveError>;
    type Error = Error;
    fn serialize_field<T: ?Sized + serde::Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.fields.push((
            key,
            Some(E::handle(value.serialize(Serializer {
                config: self.config,
            }))?),
        ));
        Ok(())
    }
    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        check("struct", &self.config, self.expected_len, &mut self.fields)?;

        Ok(Save::StructVariant {
            variant: self.variant,
            fields: self.fields,
        })
    }
    fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
        self.fields.push((key, None));
        Ok(())
    }
}
