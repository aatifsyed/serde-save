use serde_save::{Save, Variant};
use std::{error::Error, path::PathBuf};
use valuable::{
    EnumDef, Enumerable, Fields, Listable, Mappable, NamedField, NamedValues, StructDef,
    Structable, Tuplable, TupleDef, Valuable, Value, VariantDef, Visit,
};

pub enum OwnedValue {
    Bool(bool),
    Char(char),
    F32(f32),
    F64(f64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    String(String),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    Path(PathBuf),
    Error(Box<dyn Error + Send + Sync>),
    Listable(Box<dyn Listable + Send + Sync>),
    Mappable(Box<dyn Mappable + Send + Sync>),
    Structable(Box<dyn Structable + Send + Sync>),
    Enumerable(Box<dyn Enumerable + Send + Sync>),
    Tuplable(Box<dyn Tuplable + Send + Sync>),
    Unit,
}

impl Valuable for OwnedValue {
    fn as_value(&self) -> Value<'_> {
        match self {
            OwnedValue::Bool(it) => Value::Bool(*it),
            OwnedValue::Char(it) => Value::Char(*it),
            OwnedValue::F32(it) => Value::F32(*it),
            OwnedValue::F64(it) => Value::F64(*it),
            OwnedValue::I8(it) => Value::I8(*it),
            OwnedValue::I16(it) => Value::I16(*it),
            OwnedValue::I32(it) => Value::I32(*it),
            OwnedValue::I64(it) => Value::I64(*it),
            OwnedValue::I128(it) => Value::I128(*it),
            OwnedValue::Isize(it) => Value::Isize(*it),
            OwnedValue::String(it) => Value::String(it),
            OwnedValue::U8(it) => Value::U8(*it),
            OwnedValue::U16(it) => Value::U16(*it),
            OwnedValue::U32(it) => Value::U32(*it),
            OwnedValue::U64(it) => Value::U64(*it),
            OwnedValue::U128(it) => Value::U128(*it),
            OwnedValue::Usize(it) => Value::Usize(*it),
            OwnedValue::Path(it) => Value::Path(it),
            OwnedValue::Error(it) => Value::Error(&**it),
            OwnedValue::Listable(it) => Value::Listable(it),
            OwnedValue::Mappable(it) => Value::Mappable(it),
            OwnedValue::Structable(it) => Value::Structable(it),
            OwnedValue::Enumerable(it) => Value::Enumerable(it),
            OwnedValue::Tuplable(it) => Value::Tuplable(it),
            OwnedValue::Unit => Value::Unit,
        }
    }

    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(self.as_value())
    }
}

impl<E> From<Save<E>> for OwnedValue
where
    E: Error + Send + Sync + 'static,
{
    fn from(value: Save<E>) -> Self {
        match value {
            Save::Bool(it) => Self::Bool(it),
            Save::I8(it) => Self::I8(it),
            Save::I16(it) => Self::I16(it),
            Save::I32(it) => Self::I32(it),
            Save::I64(it) => Self::I64(it),
            Save::I128(it) => Self::I128(it),
            Save::U8(it) => Self::U8(it),
            Save::U16(it) => Self::U16(it),
            Save::U32(it) => Self::U32(it),
            Save::U64(it) => Self::U64(it),
            Save::U128(it) => Self::U128(it),
            Save::F32(it) => Self::F32(it),
            Save::F64(it) => Self::F64(it),
            Save::Char(it) => Self::Char(it),
            Save::String(it) => Self::String(it),
            Save::ByteArray(_) => todo!(),
            Save::Option(it) => {
                use valuable::Variant;
                const NONE: VariantDef = VariantDef::new("None", Fields::Unnamed(0));
                const SOME: VariantDef = VariantDef::new("Some", Fields::Unnamed(1));
                struct Helper(Option<OwnedValue>);
                impl Enumerable for Helper {
                    fn definition(&self) -> EnumDef<'_> {
                        const VARIANTS: &[VariantDef] = &[NONE, SOME];
                        EnumDef::new_static("Option", VARIANTS)
                    }
                    fn variant(&self) -> Variant<'_> {
                        match &self.0 {
                            Some(_) => Variant::Static(&SOME),
                            None => Variant::Static(&SOME),
                        }
                    }
                }
                impl Valuable for Helper {
                    fn as_value(&self) -> Value<'_> {
                        match &self.0 {
                            Some(it) => it.as_value(),
                            None => Value::Unit,
                        }
                    }
                    fn visit(&self, visit: &mut dyn Visit) {
                        visit.visit_value(self.as_value())
                    }
                }
                Self::Enumerable(Box::new(Helper(it.map(|it| (*it).into()))))
            }
            Save::Unit => Self::Unit,
            Save::UnitStruct(name) => {
                struct Helper(&'static str);
                impl Structable for Helper {
                    fn definition(&self) -> StructDef<'_> {
                        StructDef::new_static(self.0, Fields::Unnamed(0))
                    }
                }
                impl Valuable for Helper {
                    fn as_value(&self) -> Value<'_> {
                        Value::Unit
                    }
                    fn visit(&self, visit: &mut dyn Visit) {
                        visit.visit_value(self.as_value())
                    }
                }
                Self::Structable(Box::new(Helper(name)))
            }
            Save::UnitVariant(Variant {
                name,
                variant_index: _,
                variant,
            }) => {
                struct Helper {
                    name: &'static str,
                    variants: [VariantDef<'static>; 1],
                }
                impl Enumerable for Helper {
                    fn definition(&self) -> EnumDef<'_> {
                        EnumDef::new_dynamic(self.name, &self.variants)
                    }
                    fn variant(&self) -> valuable::Variant<'_> {
                        // TODO(aatifsyed): valuable::Variant::Static { &'a VariantDef<'static> }
                        // TODO(aatifsyed): this doesn't actually follow the documentation for
                        //                  VariantDef::Dynamic - we return the same variant from
                        //                  `fn definition` and `fn variant`
                        valuable::Variant::Dynamic(VariantDef::new(
                            self.variants[0].name(),
                            Fields::Unnamed(0),
                        ))
                    }
                }
                impl Valuable for Helper {
                    fn as_value(&self) -> Value<'_> {
                        Value::Unit
                    }
                    fn visit(&self, visit: &mut dyn Visit) {
                        visit.visit_value(self.as_value())
                    }
                }
                Self::Enumerable(Box::new(Helper {
                    name,
                    variants: [VariantDef::new(variant, Fields::Unnamed(0))],
                }))
            }
            Save::NewTypeStruct { name, value } => {
                struct Helper {
                    name: &'static str,
                    value: OwnedValue,
                }
                impl Valuable for Helper {
                    fn as_value(&self) -> Value<'_> {
                        self.value.as_value()
                    }

                    fn visit(&self, visit: &mut dyn Visit) {
                        visit.visit_value(self.as_value())
                    }
                }
                impl Structable for Helper {
                    fn definition(&self) -> StructDef<'_> {
                        StructDef::new_dynamic(self.name, Fields::Unnamed(1))
                    }
                }

                Self::Structable(Box::new(Helper {
                    name,
                    value: (*value).into(),
                }))
            }
            Save::NewTypeVariant {
                variant:
                    Variant {
                        name,
                        variant_index: _,
                        variant,
                    },
                value,
            } => {
                struct Helper {
                    name: &'static str,
                    variants: [VariantDef<'static>; 1],
                    value: OwnedValue,
                }
                impl Valuable for Helper {
                    fn as_value(&self) -> Value<'_> {
                        self.value.as_value()
                    }
                    fn visit(&self, visit: &mut dyn Visit) {
                        visit.visit_value(self.as_value())
                    }
                }
                impl Enumerable for Helper {
                    fn definition(&self) -> EnumDef<'_> {
                        EnumDef::new_dynamic(self.name, &self.variants)
                    }
                    fn variant(&self) -> valuable::Variant<'_> {
                        // TODO(aatifsyed): same as for UnitVariant
                        valuable::Variant::Dynamic(VariantDef::new(
                            self.variants[0].name(),
                            Fields::Unnamed(0),
                        ))
                    }
                }
                Self::Enumerable(Box::new(Helper {
                    name,
                    variants: [VariantDef::new(variant, Fields::Unnamed(1))],
                    value: (*value).into(),
                }))
            }
            Save::Seq(it) => Self::Listable(Box::new(
                // TODO(aatifsyed): shouldn't need double-indirection here
                it.into_iter().map(OwnedValue::from).collect::<Box<[_]>>(),
            )),
            Save::Map(it) => {
                struct Helper(Box<[(OwnedValue, OwnedValue)]>);
                impl Mappable for Helper {
                    fn size_hint(&self) -> (usize, Option<usize>) {
                        self.0.size_hint()
                    }
                }
                impl Valuable for Helper {
                    fn as_value(&self) -> Value<'_> {
                        Value::Mappable(self)
                    }

                    fn visit(&self, visit: &mut dyn Visit) {
                        for (k, v) in &*self.0 {
                            visit.visit_entry(k.as_value(), v.as_value())
                        }
                    }
                }
                Self::Mappable(Box::new(Helper(
                    it.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
                )))
            }
            Save::Tuple(it) => {
                struct Helper(Box<[OwnedValue]>);
                impl Valuable for Helper {
                    fn as_value(&self) -> Value<'_> {
                        Value::Tuplable(self)
                    }

                    fn visit(&self, visit: &mut dyn Visit) {
                        for it in &*self.0 {
                            visit.visit_value(it.as_value())
                        }
                    }
                }
                impl Tuplable for Helper {
                    fn definition(&self) -> TupleDef {
                        TupleDef::new_static(self.0.len())
                    }
                }
                Self::Tuplable(Box::new(Helper(it.into_iter().map(Into::into).collect())))
            }
            Save::TupleStruct { name, values } => {
                struct Helper {
                    name: &'static str,
                    values: Box<[OwnedValue]>,
                }
                impl Structable for Helper {
                    fn definition(&self) -> StructDef<'_> {
                        StructDef::new_static(self.name, Fields::Unnamed(self.values.len()))
                    }
                }
                impl Valuable for Helper {
                    fn as_value(&self) -> Value<'_> {
                        Value::Structable(self)
                    }

                    fn visit(&self, visit: &mut dyn Visit) {
                        for it in &*self.values {
                            visit.visit_value(it.as_value())
                        }
                    }
                }
                Self::Structable(Box::new(Helper {
                    name,
                    values: values.into_iter().map(Into::into).collect(),
                }))
            }
            Save::TupleVariant {
                variant:
                    Variant {
                        name,
                        variant_index: _,
                        variant,
                    },
                values,
            } => {
                struct Helper {
                    name: &'static str,
                    variants: [VariantDef<'static>; 1],
                    values: Box<[OwnedValue]>,
                }
                impl Enumerable for Helper {
                    fn definition(&self) -> EnumDef<'_> {
                        EnumDef::new_dynamic(self.name, &self.variants)
                    }

                    fn variant(&self) -> valuable::Variant<'_> {
                        // TODO(aatifsyed): same as for UnitVariant
                        valuable::Variant::Dynamic(VariantDef::new(
                            self.variants[0].name(),
                            Fields::Unnamed(0),
                        ))
                    }
                }
                impl Valuable for Helper {
                    fn as_value(&self) -> Value<'_> {
                        Value::Enumerable(self)
                    }

                    fn visit(&self, visit: &mut dyn Visit) {
                        for it in &*self.values {
                            visit.visit_value(it.as_value())
                        }
                    }
                }
                Self::Enumerable(Box::new(Helper {
                    name,
                    variants: [VariantDef::new(variant, Fields::Unnamed(values.len()))],
                    values: values.into_iter().map(Into::into).collect(),
                }))
            }
            Save::Struct { name, fields } => {
                struct Helper {
                    name: &'static str,
                    all: Box<[NamedField<'static>]>,
                    present: Box<[NamedField<'static>]>,
                    values: Box<[OwnedValue]>,
                }
                impl Structable for Helper {
                    fn definition(&self) -> StructDef<'_> {
                        StructDef::new_dynamic(self.name, Fields::Named(&self.all))
                    }
                }
                impl Valuable for Helper {
                    fn as_value(&self) -> Value<'_> {
                        Value::Structable(self)
                    }
                    fn visit(&self, visit: &mut dyn Visit) {
                        visit.visit_named_fields(&NamedValues::new(
                            &self.present,
                            &self
                                .values
                                .iter()
                                .map(valuable::Valuable::as_value)
                                .collect::<Box<_>>(),
                        ))
                    }
                }
                Self::Structable(Box::new(Helper {
                    name,
                    all: collect_fields(&fields),
                    present: fields.iter().map(|(it, _)| NamedField::new(it)).collect(),
                    values: fields
                        .into_iter()
                        .flat_map(|(_, it)| it.map(Into::into))
                        .collect(),
                }))
            }
            Save::StructVariant {
                variant:
                    Variant {
                        name,
                        variant_index: _,
                        variant,
                    },
                fields,
            } => {
                struct Helper {
                    name: &'static str,
                    variants: [VariantDef<'static>; 1],
                    all: Box<[NamedField<'static>]>,
                    present: Box<[NamedField<'static>]>,
                    values: Box<[OwnedValue]>,
                }
                impl Enumerable for Helper {
                    fn definition(&self) -> EnumDef<'_> {
                        EnumDef::new_dynamic(self.name, &self.variants)
                    }

                    fn variant(&self) -> valuable::Variant<'_> {
                        valuable::Variant::Dynamic(VariantDef::new(
                            self.variants[0].name(),
                            Fields::Named(&self.all),
                        ))
                    }
                }
                impl Valuable for Helper {
                    fn as_value(&self) -> Value<'_> {
                        Value::Enumerable(self)
                    }

                    fn visit(&self, visit: &mut dyn Visit) {
                        visit.visit_named_fields(&NamedValues::new(
                            &self.present,
                            &self
                                .values
                                .iter()
                                .map(valuable::Valuable::as_value)
                                .collect::<Box<_>>(),
                        ))
                    }
                }
                // TODO(aatifsyed): is there any way to plumb the field names through?
                const MARKER: &[NamedField] = &[NamedField::new("!missing")];
                Self::Enumerable(Box::new(Helper {
                    name,
                    variants: [VariantDef::new(variant, Fields::Named(MARKER))],
                    all: collect_fields(&fields),
                    present: fields.iter().map(|(it, _)| NamedField::new(it)).collect(),
                    values: fields
                        .into_iter()
                        .flat_map(|(_, it)| it.map(Into::into))
                        .collect(),
                }))
            }
            Save::Error(e) => Self::Error(Box::new(e)),
        }
    }
}

fn collect_fields<E>(
    fields: &[(&'static str, Option<Save<E>>)],
) -> Box<[valuable::NamedField<'static>]> {
    let fields = fields
        .iter()
        .map(|(it, _)| valuable::NamedField::new(it))
        .collect::<Box<_>>();
    fields
}

fn main() {}
