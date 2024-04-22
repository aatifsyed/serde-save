use core::fmt;
use std::convert::Infallible;

use serde::{
    ser::{
        Error as _, SerializeMap as _, SerializeStruct as _, SerializeStructVariant as _,
        SerializeTuple as _, SerializeTupleStruct as _, SerializeTupleVariant as _,
    },
    Serialize,
};

pub mod ser;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Save<E = Infallible> {
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
    Option(Option<Box<Self>>),

    Unit,
    UnitStruct(&'static str),
    UnitVariant(Variant),

    NewTypeStruct {
        name: &'static str,
        value: Box<Self>,
    },
    NewTypeVariant {
        variant: Variant,
        value: Box<Self>,
    },

    Seq(Vec<Self>),
    Map(Vec<(Self, Self)>),

    Tuple(Vec<Self>),
    TupleStruct {
        name: &'static str,
        values: Vec<Self>,
    },
    TupleVariant {
        variant: Variant,
        values: Vec<Self>,
    },

    Struct {
        name: &'static str,
        fields: Vec<(&'static str, Self)>,
        skipped_fields: Vec<&'static str>,
    },
    StructVariant {
        variant: Variant,
        fields: Vec<(&'static str, Self)>,
        skipped_fields: Vec<&'static str>,
    },

    Error(E),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variant {
    pub name: &'static str,
    pub variant_index: u32,
    pub variant: &'static str,
}

impl<E> Serialize for Save<E>
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
            Save::Struct {
                name,
                fields,
                skipped_fields,
            } => {
                let mut strukt = serializer.serialize_struct(name, fields.len())?;
                for (k, v) in fields {
                    strukt.serialize_field(k, v)?
                }
                for it in skipped_fields {
                    strukt.skip_field(it)?
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
                skipped_fields,
            } => {
                let mut var = serializer.serialize_struct_variant(
                    name,
                    *variant_index,
                    variant,
                    fields.len(),
                )?;
                for (k, v) in fields {
                    var.serialize_field(k, v)?
                }
                for it in skipped_fields {
                    var.skip_field(it)?
                }
                var.end()
            }
            Save::Error(e) => Err(S::Error::custom(e)),
        }
    }
}

pub fn save<T: Serialize>(t: T) -> Result<Save, ser::Error> {
    t.serialize(ser::Serializer::new())
}

pub fn save_errors<T: Serialize>(t: T) -> Save<ser::Error> {
    t.serialize(ser::Serializer::new().save_errors())
        .unwrap_or_else(Save::Error)
}
