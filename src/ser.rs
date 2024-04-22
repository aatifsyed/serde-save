use crate::{Save, Variant};
use core::{convert::Infallible, fmt, marker::PhantomData};

#[derive(Debug)]
pub struct Error {
    msg: String,
    protocol: bool,
}

impl Error {
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

pub struct Serializer<ErrorDiscipline = ShortCircuit> {
    config: Config<ErrorDiscipline>,
}

impl Serializer<ShortCircuit> {
    pub fn new() -> Self {
        Self {
            config: Config {
                is_human_readable: false,
                protocol_errors: true,
                _error_discipline: PhantomData,
            },
        }
    }
}

impl<E> Serializer<E> {
    pub fn human_readable(mut self, is_human_readable: bool) -> Self {
        self.config.is_human_readable = is_human_readable;
        self
    }
    pub fn check_for_protocol_errors(mut self, check: bool) -> Self {
        self.config.protocol_errors = check;
        self
    }
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
    fn default() -> Self {
        Self::new()
    }
}

struct Config<E = ShortCircuit> {
    is_human_readable: bool,
    // TODO(aatifsyed): handle
    // - jagged key/value pairs in a map
    // - incorrect sequence/tuple lengths
    // - duplicate field names
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
    type Ok = Save<E::SaveError>;
    type Error = Error;
    type SerializeSeq = imp::SerializeSeq<E>;
    type SerializeTuple = imp::SerializeTuple<E>;
    type SerializeTupleStruct = imp::SerializeTupleStruct<E>;
    type SerializeTupleVariant = imp::SerializeTupleVariant<E>;
    type SerializeMap = imp::SerializeMap<E>;
    type SerializeStruct = imp::SerializeStruct<E>;
    type SerializeStructVariant = imp::SerializeStructVariant<E>;

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
        Ok(imp::SerializeSeq {
            config: self.config,
            inner: Vec::with_capacity(len.unwrap_or_default()),
            expected_len: len,
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(imp::SerializeTuple {
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
        Ok(imp::SerializeTupleStruct {
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
        Ok(imp::SerializeTupleVariant {
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
        Ok(imp::SerializeMap {
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
        Ok(imp::SerializeStruct {
            expected_len: len,
            config: self.config,
            name,
            fields: Vec::with_capacity(len),
            skipped_fields: Vec::new(),
        })
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(imp::SerializeStructVariant {
            config: self.config,
            variant: Variant {
                name,
                variant_index,
                variant,
            },
            fields: Vec::with_capacity(len),
            skipped_fields: Vec::new(),
            expected_len: len,
        })
    }
}

mod imp {
    use std::cmp;

    use super::*;

    fn check_length<E>(
        what: &str,
        config: &Config<E>,
        expected: usize,
        pushing: &mut Vec<Save<E::SaveError>>,
    ) -> Result<(), Error>
    where
        E: ErrorDiscipline,
    {
        if config.protocol_errors {
            let actual = pushing.len();
            if expected != actual {
                let e = Error {
                    msg: format!(
                        "protcol error: expected a {} of length {}, got {}",
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
        pub(super) config: Config<E>,
        pub(super) expected_len: Option<usize>,
        pub(super) inner: Vec<Save<E::SaveError>>,
    }
    impl<E> serde::ser::SerializeSeq for SerializeSeq<E>
    where
        E: ErrorDiscipline,
    {
        type Ok = Save<E::SaveError>;
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
        pub(super) expected_len: usize,
        pub(super) config: Config<E>,
        pub(super) inner: Vec<Save<E::SaveError>>,
    }
    impl<E> serde::ser::SerializeTuple for SerializeTuple<E>
    where
        E: ErrorDiscipline,
    {
        type Ok = Save<E::SaveError>;
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
        pub(super) expected_len: usize,
        pub(super) config: Config<E>,
        pub(super) name: &'static str,
        pub(super) values: Vec<Save<E::SaveError>>,
    }
    impl<E> serde::ser::SerializeTupleStruct for SerializeTupleStruct<E>
    where
        E: ErrorDiscipline,
    {
        type Ok = Save<E::SaveError>;
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
        pub(super) expected_len: usize,
        pub(super) config: Config<E>,
        pub(super) variant: Variant,
        pub(super) values: Vec<Save<E::SaveError>>,
    }
    impl<E> serde::ser::SerializeTupleVariant for SerializeTupleVariant<E>
    where
        E: ErrorDiscipline,
    {
        type Ok = Save<E::SaveError>;
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
        pub(super) expected_len: Option<usize>,
        pub(super) config: Config<E>,
        pub(super) keys: Vec<Save<E::SaveError>>,
        pub(super) values: Vec<Save<E::SaveError>>,
    }
    impl<E> serde::ser::SerializeMap for SerializeMap<E>
    where
        E: ErrorDiscipline,
    {
        type Ok = Save<E::SaveError>;
        type Error = Error;
        fn serialize_key<T: ?Sized + serde::Serialize>(
            &mut self,
            key: &T,
        ) -> Result<(), Self::Error> {
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
                                        "protcol error: expected a map of length {}, got {}",
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
    pub struct SerializeStruct<E: ErrorDiscipline> {
        // TODO(aatifsyed): handle mismatch and field name duplications
        #[allow(unused)]
        pub(super) expected_len: usize,
        pub(super) config: Config<E>,
        pub(super) name: &'static str,
        pub(super) fields: Vec<(&'static str, Save<E::SaveError>)>,
        pub(super) skipped_fields: Vec<&'static str>,
    }
    impl<E> serde::ser::SerializeStruct for SerializeStruct<E>
    where
        E: ErrorDiscipline,
    {
        type Ok = Save<E::SaveError>;
        type Error = Error;
        fn serialize_field<T: ?Sized + serde::Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            self.fields.push((
                key,
                E::handle(value.serialize(Serializer {
                    config: self.config,
                }))?,
            ));
            Ok(())
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Save::Struct {
                name: self.name,
                fields: self.fields,
                skipped_fields: self.skipped_fields,
            })
        }
        fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
            self.skipped_fields.push(key);
            Ok(())
        }
    }
    pub struct SerializeStructVariant<E: ErrorDiscipline> {
        // TODO(aatifsyed): handle mismatch and field name duplications
        #[allow(unused)]
        pub(super) expected_len: usize,
        pub(super) config: Config<E>,
        pub(super) variant: Variant,
        pub(super) fields: Vec<(&'static str, Save<E::SaveError>)>,
        pub(super) skipped_fields: Vec<&'static str>,
    }
    impl<E> serde::ser::SerializeStructVariant for SerializeStructVariant<E>
    where
        E: ErrorDiscipline,
    {
        type Ok = Save<E::SaveError>;
        type Error = Error;
        fn serialize_field<T: ?Sized + serde::Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            self.fields.push((
                key,
                E::handle(value.serialize(Serializer {
                    config: self.config,
                }))?,
            ));
            Ok(())
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Save::StructVariant {
                variant: self.variant,
                fields: self.fields,
                skipped_fields: self.skipped_fields,
            })
        }
        fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
            self.skipped_fields.push(key);
            Ok(())
        }
    }
}
