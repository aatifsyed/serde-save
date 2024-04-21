use crate::{Save, Variant};
use core::{fmt, iter};

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

pub struct Serializer {
    config: Config,
}

#[derive(Copy, Clone)]
struct Config {
    is_human_readable: bool,
}

macro_rules! simple {
    ($($method:ident($ty:ty) -> $variant:ident);* $(;)?) => {
        $(
            fn $method(self, v: $ty) -> Result<Self::Ok, Self::Error> {
                Ok(Save::$variant(v))
            }
        )*
    };
}

impl serde::Serializer for Serializer {
    type Ok = Save;
    type Error = Error;
    type SerializeSeq = imp::SerializeSeq;
    type SerializeTuple = imp::SerializeTuple;
    type SerializeTupleStruct = imp::SerializeTupleStruct;
    type SerializeTupleVariant = imp::SerializeTupleVariant;
    type SerializeMap = imp::SerializeMap;
    type SerializeStruct = imp::SerializeStruct;
    type SerializeStructVariant = imp::SerializeStructVariant;

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
        Ok(Save::Option(Some(Box::new(value.serialize(self)?))))
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
            value: Box::new(value.serialize(self)?),
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
            value: Box::new(value.serialize(self)?),
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

    pub struct SerializeSeq {
        pub(super) config: Config,
        pub(super) inner: Vec<Save>,
    }
    impl serde::ser::SerializeSeq for SerializeSeq {
        type Ok = Save;
        type Error = Error;
        fn serialize_element<T: ?Sized + serde::Serialize>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            self.inner.push(value.serialize(Serializer {
                config: self.config,
            })?);
            Ok(())
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Save::Seq(self.inner))
        }
    }
    pub struct SerializeTuple {
        pub(super) config: Config,
        pub(super) inner: Vec<Save>,
    }
    impl serde::ser::SerializeTuple for SerializeTuple {
        type Ok = Save;
        type Error = Error;
        fn serialize_element<T: ?Sized + serde::Serialize>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            self.inner.push(value.serialize(Serializer {
                config: self.config,
            })?);
            Ok(())
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Save::Tuple(self.inner))
        }
    }
    pub struct SerializeTupleStruct {
        pub(super) config: Config,
        pub(super) name: &'static str,
        pub(super) values: Vec<Save>,
    }
    impl serde::ser::SerializeTupleStruct for SerializeTupleStruct {
        type Ok = Save;
        type Error = Error;
        fn serialize_field<T: ?Sized + serde::Serialize>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            self.values.push(value.serialize(Serializer {
                config: self.config,
            })?);
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Save::TupleStruct {
                name: self.name,
                values: self.values,
            })
        }
    }
    pub struct SerializeTupleVariant {
        pub(super) config: Config,
        pub(super) variant: Variant,
        pub(super) values: Vec<Save>,
    }
    impl serde::ser::SerializeTupleVariant for SerializeTupleVariant {
        type Ok = Save;
        type Error = Error;
        fn serialize_field<T: ?Sized + serde::Serialize>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            self.values.push(value.serialize(Serializer {
                config: self.config,
            })?);
            Ok(())
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Save::TupleVariant {
                variant: self.variant,
                values: self.values,
            })
        }
    }
    pub struct SerializeMap {
        pub(super) config: Config,
        pub(super) keys: Vec<Save>,
        pub(super) values: Vec<Save>,
    }
    impl serde::ser::SerializeMap for SerializeMap {
        type Ok = Save;
        type Error = Error;
        fn serialize_key<T: ?Sized + serde::Serialize>(
            &mut self,
            key: &T,
        ) -> Result<(), Self::Error> {
            self.keys.push(key.serialize(Serializer {
                config: self.config,
            })?);
            Ok(())
        }
        fn serialize_value<T: ?Sized + serde::Serialize>(
            &mut self,
            value: &T,
        ) -> Result<(), Self::Error> {
            self.values.push(value.serialize(Serializer {
                config: self.config,
            })?);
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
        fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
        where
            K: ?Sized + serde::Serialize,
            V: ?Sized + serde::Serialize,
        {
            self.serialize_key(key)?;
            self.serialize_value(value)
        }
    }
    pub struct SerializeStruct {
        pub(super) config: Config,
        pub(super) name: &'static str,
        pub(super) fields: Vec<(&'static str, Save)>,
        pub(super) skipped_fields: Vec<&'static str>,
    }
    impl serde::ser::SerializeStruct for SerializeStruct {
        type Ok = Save;
        type Error = Error;
        fn serialize_field<T: ?Sized + serde::Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            self.fields.push((
                key,
                value.serialize(Serializer {
                    config: self.config,
                })?,
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
    pub struct SerializeStructVariant {
        pub(super) config: Config,
        pub(super) variant: Variant,
        pub(super) fields: Vec<(&'static str, Save)>,
        pub(super) skipped_fields: Vec<&'static str>,
    }
    impl serde::ser::SerializeStructVariant for SerializeStructVariant {
        type Ok = Save;
        type Error = Error;
        fn serialize_field<T: ?Sized + serde::Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            self.fields.push((
                key,
                value.serialize(Serializer {
                    config: self.config,
                })?,
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
