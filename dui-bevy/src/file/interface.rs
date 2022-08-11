use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::ops::Range;
use arrayvec::ArrayVec;
use logos::Logos;
use derive_more::Display;
use join_lazy_fmt::Join;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use devolve_ui_core::dui::{DuiMetaFieldKind, DuiMetaIOType, DuiMetaType};
use crate::misc::array_vec::arrayvec;
use crate::misc::parse::ParseError;

// region type defs
#[derive(Debug, Clone, PartialEq, Display)]
#[display(fmt = "[\n\t{}\n]", "\",\t\".join(_0)")]
pub struct Interface(Vec<InterfaceField>);

#[derive(Debug, Clone, PartialEq, Display)]
#[display(fmt = "{} => {}", name, value)]
pub struct InterfaceField {
    pub name: String,
    pub value: InterfaceValue
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Display)]
#[serde(try_from = "String", into = "String")]
pub enum InterfaceValue {
    Const(InterfaceDataType),
    #[display(fmt = "in:{}", _0)]
    In(InterfaceDataType),
    #[display(fmt = "out={}", _0)]
    Out(InterfaceOutput)
}

#[derive(Debug, Display)]
pub enum InterfaceFieldType<'a> {
    #[display(fmt = "{}", _0)]
    Const(&'a InterfaceDataType),
    #[display(fmt = "In<{}>" _0)]
    In(&'a InterfaceDataType),
    #[display(fmt = "Out<{}>", _0)]
    Out(InterfaceDataType)
}

#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum InterfaceDataType {
    #[display(fmt = "{{hole}}")]
    Hole,
    Atom(InterfaceDataTypeAtom),
    #[display(fmt = "({})", "\", \".join(_0)")]
    Tuple(Vec<InterfaceDataType>),
    #[display(fmt = "[{}; {}]", _0, _1)]
    Array(Box<InterfaceDataType>, usize),
    #[display(fmt = "Vec<{}>", _0)]
    Vec(Box<InterfaceDataType>),
    #[display(fmt = "Map<{}, {}>", _0, _1)]
    Map(Box<InterfaceDataType>, Box<InterfaceDataType>),
    #[display(fmt = "Option<{}>", _0)]
    Option(Box<InterfaceDataType>)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceDataTypeAtom {
    U8,
    U16,
    U32,
    U64,
    USize,
    I8,
    I16,
    I32,
    I64,
    ISize,
    F32,
    F64,
    Bool,
    String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[display(fmt = "invalid atom type")]
pub struct ErrorInterfaceDataTypeAtomInvalid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum InterfaceOutput {
    #[display(fmt = "mouse")]
    Mouse,
    #[display(fmt = "keys")]
    Keys
}
// endregion

impl Interface {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item=&InterfaceField> {
        self.0.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item=&String> {
        self.iter().map(|field| &field.name)
    }

    pub fn get(&self, name: &str) -> Option<&InterfaceValue> {
        self.0.iter().find(|field| field.name == name).map(|field| &field.value)
    }
}

impl InterfaceValue {
    pub fn type_(&self) -> InterfaceFieldType<'_> {
        match self {
            InterfaceValue::Const(type_) => InterfaceFieldType::Const(type_),
            InterfaceValue::In(type_) => InterfaceFieldType::In(type_),
            InterfaceValue::Out(value) => InterfaceFieldType::Out(value.type_())
        }
    }
}

impl<'a> PartialEq<DuiMetaFieldKind> for InterfaceFieldType<'a> {
    fn eq(&self, other: &DuiMetaFieldKind) -> bool {
        match other {
            DuiMetaFieldKind::Atom { io_type, inner_type } => {
                let (expected_io_type, expected_data_type) = match self {
                    InterfaceFieldType::Const(type_) => (DuiMetaIOType::Const, *type_),
                    InterfaceFieldType::In(type_) => (DuiMetaIOType::In, *type_),
                    InterfaceFieldType::Out(type_) => (DuiMetaIOType::Out, type_)
                };
                expected_io_type == *io_type && expected_data_type == inner_type
            },
            DuiMetaFieldKind::Compound { .. } => false,
        }
    }
}

impl PartialEq<DuiMetaType> for InterfaceDataType {
    fn eq(&self, other: &DuiMetaType) -> bool {
        // Maybe a better comparison in the future, but this is fine for now
        self.to_string() == other.simple_type_name()
    }
}

impl InterfaceDataType {
    pub fn is_hole(&self) -> bool {
        match self {
            InterfaceDataType::Hole => true,
            _ => false
        }
    }

    pub fn as_mut_tuple(&mut self) -> Option<&mut Vec<InterfaceDataType>> {
        match self {
            InterfaceDataType::Tuple(tuple) => Some(tuple),
            _ => None
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            InterfaceDataType::Array(_, _) => true,
            _ => false
        }
    }

    pub fn as_mut_array(&mut self) -> Option<(&mut InterfaceDataType, &mut usize)> {
        match self {
            InterfaceDataType::Array(data, size) => Some((data, size)),
            _ => None
        }
    }

    pub fn as_mut_generic(&mut self) -> Option<ArrayVec<&mut InterfaceDataType, 2>> {
        match self {
            InterfaceDataType::Vec(data) => Some(arrayvec![data.as_mut()]),
            InterfaceDataType::Map(key, value) => Some(arrayvec![key.as_mut(), value.as_mut()]),
            InterfaceDataType::Option(data) => Some(arrayvec![data.as_mut()]),
            _ => None
        }
    }
}

impl InterfaceOutput {
    pub fn type_(&self) -> InterfaceDataType {
        match self {
            InterfaceOutput::Mouse => Self::mouse_type(),
            InterfaceOutput::Keys => Self::keys_type()
        }
    }

    fn mouse_type() -> InterfaceDataType {
        InterfaceDataType::Tuple(vec![
            InterfaceDataType::Atom(InterfaceDataTypeAtom::F32),
            InterfaceDataType::Atom(InterfaceDataTypeAtom::F32)
        ])
    }

    fn keys_type() -> InterfaceDataType {
        InterfaceDataType::Array(
            Box::new(InterfaceDataType::Atom(InterfaceDataTypeAtom::U32)),
            32
        )
    }
}

// region serde
#[derive(Debug, Clone, Copy, PartialEq, Logos)]
pub enum InterfaceFieldToken {
    #[regex("[0-9]+")]
    Integer,

    #[regex("[A-Za-z_][A-Za-z0-9_]*")]
    Type,

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBrack,
    #[token("]")]
    RBrack,
    #[token("Vec<")]
    VecLAngle,
    #[token("Map<")]
    MapLAngle,
    #[token("Option<")]
    OptionLAngle,
    #[token(">")]
    RAngle,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,

    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error
}

pub type InterfaceFieldParseError = ParseError<InterfaceFieldParseErrorType>;

#[derive(Debug, Display)]
pub enum InterfaceFieldParseErrorType {
    #[display(fmt = "syntax error, invalid token")]
    InvalidToken,
    #[display(fmt = "syntax error, can't close here")]
    InvalidCloseToken,
    #[display(fmt = "syntax error, nesting issue")]
    NestingIssue,
    #[display(fmt = "syntax error, array must be the form [type; size]")]
    BadArrayForm,
    #[display(fmt = "syntax error, generic has wrong number of arguments")]
    WrongNumParams,
    #[display(fmt = "zero or multiple types separated by comma, expected only one")]
    NotOneType,
    #[display(fmt = "type not found")]
    TypeNotFound,
    #[display(fmt = "not a valid output")]
    OutputNotFound
}

impl Serialize for Interface {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for field in self.0.iter() {
            map.serialize_entry(&field.name, &field.value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Interface {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MyVisitor;

        impl<'de> Visitor<'de> for MyVisitor {
            type Value = Interface;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut file = Interface(Vec::new());
                let mut found_names = HashSet::new();
                while let Some((name, value)) = map.next_entry::<String, InterfaceValue>()? {
                    if !found_names.insert(name.clone()) {
                        return Err(serde::de::Error::custom(format!("duplicate field name: {}", name)));
                    }
                    file.0.push(InterfaceField {
                        name,
                        value
                    });
                }
                Ok(file)
            }
        }

        deserializer.deserialize_map(MyVisitor)
    }
}

impl TryFrom<String> for InterfaceValue {
    type Error = InterfaceFieldParseError;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        Self::try_from(string.as_str())
    }
}

impl<'a> TryFrom<&'a str> for InterfaceValue {
    type Error = InterfaceFieldParseError;

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        if str.starts_with("in:") {
            str[3..].try_into()
                .map_err(|mut err: InterfaceFieldParseError| { err.offset(3); err })
                .map(InterfaceValue::In)
        } else if str.starts_with("out=") {
            str[4..].try_into()
                .map_err(|mut err: InterfaceFieldParseError| { err.offset(4); err })
                .map(InterfaceValue::Out)
        } else {
            str.try_into().map(InterfaceValue::Const)
        }
    }
}

impl Into<String> for InterfaceValue {
    fn into(self) -> String {
        self.to_string()
    }
}

impl<'a> TryFrom<&'a str> for InterfaceDataType {
    type Error = InterfaceFieldParseError;

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        let mut lexer = InterfaceFieldToken::lexer(str);

        let mut stack: Vec<(usize, InterfaceDataType)> = Vec::new();
        let mut atoms: Vec<InterfaceDataType> = Vec::new();
        let mut expects_comma_semi = false;
        fn err_unknown_type(span: &Range<usize>) -> InterfaceFieldParseError {
            ParseError {
                reason: InterfaceFieldParseErrorType::TypeNotFound,
                span: span.clone()
            }
        }
        fn err_nesting(span: &Range<usize>) -> InterfaceFieldParseError {
            ParseError {
                reason: InterfaceFieldParseErrorType::NestingIssue,
                span: span.clone()
            }
        }
        fn err_invalid_token(span: &Range<usize>) -> InterfaceFieldParseError {
            ParseError {
                reason: InterfaceFieldParseErrorType::InvalidToken,
                span: span.clone()
            }
        }
        fn err_invalid_close_token(span: &Range<usize>) -> InterfaceFieldParseError {
            ParseError {
                reason: InterfaceFieldParseErrorType::InvalidCloseToken,
                span: span.clone()
            }
        }
        fn err_bad_array_form(span: &Range<usize>) -> InterfaceFieldParseError {
            ParseError {
                reason: InterfaceFieldParseErrorType::BadArrayForm,
                span: span.clone()
            }
        }
        fn err_wrong_num_params(span: &Range<usize>) -> InterfaceFieldParseError {
            ParseError {
                reason: InterfaceFieldParseErrorType::WrongNumParams,
                span: span.clone()
            }
        }
        fn err_not_one_type(span: &Range<usize>) -> InterfaceFieldParseError {
            ParseError {
                reason: InterfaceFieldParseErrorType::NotOneType,
                span: span.clone()
            }
        }
        while let Some(token) = lexer.next() {
            let span = lexer.span();
            let slice = lexer.slice();
            if expects_comma_semi {
                expects_comma_semi = false;
                match token {
                    InterfaceFieldToken::Comma => {},
                    InterfaceFieldToken::Semicolon => {
                        // Must be array
                        let (start, mut data) = stack.pop().ok_or(err_invalid_token(&span))?;
                        let (elem, size) = data.as_mut_array().ok_or(err_invalid_token(&span))?;

                        // Must have one element after start
                        debug_assert!(elem.is_hole() && *size == 0);
                        if atoms.len() != start + 1 {
                            return Err(err_bad_array_form(&span));
                        }
                        *elem = atoms.pop().unwrap();

                        // Parse size
                        let token = lexer.next().ok_or(err_invalid_token(&span))?;
                        let span = lexer.span();
                        let slice = lexer.slice();
                        *size = match token {
                            InterfaceFieldToken::Integer => slice.parse::<usize>().map_err(|_| err_invalid_token(&span))?,
                            _ => return Err(err_invalid_token(&span))
                        };

                        // Parse rbrack
                        let token = lexer.next().ok_or(err_invalid_token(&span))?;
                        let span = lexer.span();
                        match token {
                            InterfaceFieldToken::LBrack => {},
                            _ => return Err(err_bad_array_form(&span))
                        }

                        // Ok we finished array
                        atoms.push(data);
                    }
                    InterfaceFieldToken::RParen => {
                        let (start, mut data) = stack.pop().ok_or(err_nesting(&span))?;
                        let tuple = data.as_mut_tuple().ok_or(err_nesting(&span))?;
                        tuple.reserve(atoms.len() - start);
                        for atom in atoms.drain(start..) {
                            tuple.push(atom);
                        }
                        atoms.push(data);
                    }
                    InterfaceFieldToken::RAngle => {
                        let (start, mut data) = stack.pop().ok_or(err_nesting(&span))?;
                        {
                            let mut params = data.as_mut_generic().ok_or(err_nesting(&span))?;
                            if atoms.len() != start + params.len() {
                                return Err(err_wrong_num_params(&span));
                            }
                            for (idx, atom) in atoms.drain(start..).enumerate() {
                                *params[idx] = atom;
                            }
                        }
                        atoms.push(data);
                    }
                    _ => return Err(err_invalid_token(&span))
                }
            } else {
                match token {
                    InterfaceFieldToken::Type => {
                        let atom_type = InterfaceDataTypeAtom::try_from(slice).map_err(|_| err_unknown_type(&span))?;
                        atoms.push(InterfaceDataType::Atom(atom_type));
                        expects_comma_semi = true;
                    }
                    InterfaceFieldToken::LParen => {
                        stack.push((atoms.len(), InterfaceDataType::Tuple(Vec::new())));
                    }
                    InterfaceFieldToken::LBrack => {
                        stack.push((atoms.len(), InterfaceDataType::Array(Box::new(InterfaceDataType::Hole), 0)));
                    }
                    InterfaceFieldToken::VecLAngle => {
                        stack.push((atoms.len(), InterfaceDataType::Vec(Box::new(InterfaceDataType::Hole))));
                    }
                    InterfaceFieldToken::MapLAngle => {
                        stack.push((atoms.len(), InterfaceDataType::Map(Box::new(InterfaceDataType::Hole), Box::new(InterfaceDataType::Hole))));
                    }
                    InterfaceFieldToken::OptionLAngle => {
                        stack.push((atoms.len(), InterfaceDataType::Option(Box::new(InterfaceDataType::Hole))));
                    }
                    InterfaceFieldToken::RParen | InterfaceFieldToken::RBrack | InterfaceFieldToken::RAngle => return Err(err_invalid_close_token(&span)),
                    InterfaceFieldToken::Integer | InterfaceFieldToken::Comma | InterfaceFieldToken::Semicolon | InterfaceFieldToken::Error => return Err(err_invalid_token(&span))
                }
            }
        }

        if let Some(last) = stack.last() {
            return Err(err_nesting(&(last.0..str.len())));
        }
        if atoms.len() != 1 {
            return Err(err_not_one_type(&(0..str.len())));
        }

        Ok(atoms.into_iter().next().unwrap())
    }
}

impl<'a> TryFrom<&'a str> for InterfaceOutput {
    type Error = InterfaceFieldParseError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "mouse" => Ok(InterfaceOutput::Mouse),
            "keys" => Ok(InterfaceOutput::Keys),
            _ => Err(ParseError {
                reason: InterfaceFieldParseErrorType::OutputNotFound,
                span: 0..value.len()
            })
        }
    }
}

impl Display for InterfaceDataTypeAtom {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InterfaceDataTypeAtom::U8 => write!(f, "u8"),
            InterfaceDataTypeAtom::U16 => write!(f, "u16"),
            InterfaceDataTypeAtom::U32 => write!(f, "u32"),
            InterfaceDataTypeAtom::U64 => write!(f, "u64"),
            InterfaceDataTypeAtom::USize => write!(f, "usize"),
            InterfaceDataTypeAtom::I8 => write!(f, "i8"),
            InterfaceDataTypeAtom::I16 => write!(f, "i16"),
            InterfaceDataTypeAtom::I32 => write!(f, "i32"),
            InterfaceDataTypeAtom::I64 => write!(f, "i64"),
            InterfaceDataTypeAtom::ISize => write!(f, "isize"),
            InterfaceDataTypeAtom::F32 => write!(f, "f32"),
            InterfaceDataTypeAtom::F64 => write!(f, "f64"),
            InterfaceDataTypeAtom::Bool => write!(f, "bool"),
            InterfaceDataTypeAtom::String => write!(f, "String"),
        }
    }
}

impl<'a> TryFrom<&'a str> for InterfaceDataTypeAtom {
    type Error = ErrorInterfaceDataTypeAtomInvalid;

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        match str {
            "u8" => Ok(InterfaceDataTypeAtom::U8),
            "u16" => Ok(InterfaceDataTypeAtom::U16),
            "u32" => Ok(InterfaceDataTypeAtom::U32),
            "u64" => Ok(InterfaceDataTypeAtom::U64),
            "usize" => Ok(InterfaceDataTypeAtom::USize),
            "i8" => Ok(InterfaceDataTypeAtom::I8),
            "i16" => Ok(InterfaceDataTypeAtom::I16),
            "i32" => Ok(InterfaceDataTypeAtom::I32),
            "i64" => Ok(InterfaceDataTypeAtom::I64),
            "isize" => Ok(InterfaceDataTypeAtom::ISize),
            "f32" => Ok(InterfaceDataTypeAtom::F32),
            "f64" => Ok(InterfaceDataTypeAtom::F64),
            "bool" => Ok(InterfaceDataTypeAtom::Bool),
            "String" => Ok(InterfaceDataTypeAtom::String),
            _ => Err(ErrorInterfaceDataTypeAtomInvalid)
        }
    }
}
// endregion