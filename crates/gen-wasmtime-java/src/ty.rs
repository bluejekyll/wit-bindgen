use std::{borrow::Cow, convert::TryFrom};

use wit_bindgen_gen_core::wit_parser::{ResourceId, Type, TypeId};

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum JavaType {
    /// Signed 8 bit integer, i.e. S8
    Byte,
    /// Signed 16 bit integer
    Short,
    /// Signed 32 bit integer
    Int,
    /// Signed 64 bit integer
    Long,
    /// 32 bit floating point number
    Float,
    /// 64 bit floating point number
    Double,
    /// Unsigned 16 bits
    Char,
    /// True | False
    Boolean,
}

impl From<Type> for JavaType {
    fn from(ty: Type) -> Self {
        match ty {
            Type::U8 => JavaType::Short,
            Type::U16 => JavaType::Int,
            Type::U32 => JavaType::Long,
            Type::U64 => JavaType::Long,
            Type::S8 => JavaType::Byte,
            Type::S16 => JavaType::Short,
            Type::S32 => JavaType::Int,
            Type::S64 => JavaType::Long,
            Type::F32 => JavaType::Float,
            Type::F64 => JavaType::Double,
            Type::Char => JavaType::Char,
            Type::CChar => JavaType::Char,
            Type::Usize => JavaType::Long,
            Type::Handle(ResourceId) => unimplemented!("Handle not yet supported"),
            Type::Id(TypeId) => unimplemented!("TypeId not yet supported"),
        }
    }
}

impl JavaType {
    pub fn for_fn_param(self) -> Cow<'static, str> {
        match self {
            JavaType::Short => "short".into(),
            JavaType::Int => "int".into(),
            JavaType::Long => "long".into(),
            JavaType::Byte => "byte".into(),
            JavaType::Float => "float".into(),
            JavaType::Double => "double".into(),
            JavaType::Char => "char".into(),
            JavaType::Boolean => "boolean".into(),
        }
    }

    pub fn for_fn_return(self) -> Cow<'static, str> {
        self.for_fn_param()
    }

    pub fn for_type_param(self) -> Cow<'static, str> {
        match self {
            JavaType::Short => "Short".into(),
            JavaType::Int => "Integer".into(),
            JavaType::Long => "Long".into(),
            JavaType::Byte => "Byte".into(),
            JavaType::Float => "Float".into(),
            JavaType::Double => "Double".into(),
            JavaType::Char => "Character".into(),
            JavaType::Boolean => "Boolean".into(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum JavaTuple {
    Unit,
    Pair,
    Triplet,
    Quartet,
    Quintet,
    Sextet,
    Septet,
    Octet,
    Ennead,
    Decade,
}

impl JavaTuple {
    fn to_str(self) -> &'static str {
        match self {
            JavaTuple::Unit => "Unit",
            JavaTuple::Pair => "Pair",
            JavaTuple::Triplet => "Triplet",
            JavaTuple::Quartet => "Quartet",
            JavaTuple::Quintet => "Quintet",
            JavaTuple::Sextet => "Sextet",
            JavaTuple::Septet => "Septet",
            JavaTuple::Octet => "Octet",
            JavaTuple::Ennead => "Ennead",
            JavaTuple::Decade => "Decade",
        }
    }
}

pub struct JavaTupleType(Vec<JavaType>);

impl JavaTupleType {
    pub fn from(types: Vec<JavaType>) -> Self {
        JavaTupleType(types)
    }

    pub fn for_ty(&self) -> String {
        let tuple = match self.0.len() {
            0 => panic!("no empty tuples"),
            1 => JavaTuple::Unit,
            2 => JavaTuple::Pair,
            3 => JavaTuple::Triplet,
            4 => JavaTuple::Quartet,
            5 => JavaTuple::Quintet,
            6 => JavaTuple::Sextet,
            7 => JavaTuple::Septet,
            8 => JavaTuple::Octet,
            9 => JavaTuple::Ennead,
            10 => JavaTuple::Decade,
            // consider a List of Objects?
            _ => panic!("tuples cannot exceed 10 items in Java"),
        };

        let type_list = self
            .0
            .iter()
            .map(|ty| ty.for_type_param())
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "{tuple}<{type_list}>",
            tuple = tuple.to_str(),
            type_list = type_list
        )
    }
}
