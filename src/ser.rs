use crate::{Save, Variant};
use core::{fmt, iter};
use std::{convert::Infallible, marker::PhantomData};

#[derive(Debug)]
pub struct Error {
    msg: String,
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
    pub fn save_errors(self) -> Serializer<Persist> {
        let Self {
            config:
                Config {
                    is_human_readable,
                    _error_discipline,
                },
        } = self;
        Serializer {
            config: Config {
                is_human_readable,
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
    // protocol_errors: bool,
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
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(imp::SerializeTuple {
            config: self.config,
            inner: Vec::with_capacity(len),
        })
    }
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(imp::SerializeTupleStruct {
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
        })
    }
}

mod imp {
    use serde::ser::Error as _;

    use super::*;

    pub struct SerializeSeq<E: ErrorDiscipline> {
        pub(super) config: Config<E>,
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
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Save::Seq(self.inner))
        }
    }
    pub struct SerializeTuple<E: ErrorDiscipline> {
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
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Save::Tuple(self.inner))
        }
    }
    pub struct SerializeTupleStruct<E: ErrorDiscipline> {
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

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Save::TupleStruct {
                name: self.name,
                values: self.values,
            })
        }
    }
    pub struct SerializeTupleVariant<E: ErrorDiscipline> {
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
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Save::TupleVariant {
                variant: self.variant,
                values: self.values,
            })
        }
    }
    pub struct SerializeMap<E: ErrorDiscipline> {
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
            match self.keys.len() == self.values.len() {
                true => Ok(Save::Map(iter::zip(self.keys, self.values).collect())),
                false => Err(Error::custom(
                    "number of keys and values in map doesn't match",
                )),
            }
        }
    }
    pub struct SerializeStruct<E: ErrorDiscipline> {
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
