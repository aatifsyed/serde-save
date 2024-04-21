pub mod ser;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Save {
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
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variant {
    pub name: &'static str,
    pub variant_index: u32,
    pub variant: &'static str,
}
