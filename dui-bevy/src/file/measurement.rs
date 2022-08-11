use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter};
use arrayvec::ArrayVec;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use logos::Logos;
use derive_more::{Display, Error, From};
use serde::de::Visitor;
use slicevec::SliceVec;
use crate::file::{VarIndex, GlobalVarEnvironment};
use crate::misc::parse::ParseError;

// region type defs
#[derive(Debug, Clone, Copy, PartialEq)]
/// An expression of constants and variables. Postfix and guaranteed to be well-formed (won't have syntax errors)
pub struct Measurement {
    terms: [MeasurementTerm; Measurement::MAX_NUM_TERMS]
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// An expression of constants and variables. Infix and not guaranteed to be well-formed (may have syntax errors).
pub struct InfixUnformedMeasurement {
    pub terms: [InfixMeasurementTerm; InfixUnformedMeasurement::MAX_NUM_TERMS]
}

#[derive(Debug, Clone, Copy, PartialEq, Display)]
pub enum MeasurementTerm {
    #[display(fmt = "")]
    Null,
    #[display]
    Const(f64),
    #[display(fmt = "#{}", _0)]
    Var(VarIndex),
    #[display(fmt = " {} ", _0)]
    Operand(MeasurementOp)
}

#[derive(Debug, Clone, Copy, PartialEq, Display)]
pub enum InfixMeasurementTerm {
    #[display(fmt = "")]
    Null,
    #[display]
    Const(f64),
    #[display(fmt = "#{}", _0)]
    Var(VarIndex),
    #[display(fmt = "(")]
    LParen,
    #[display(fmt = ")")]
    RParen,
    #[display(fmt = " {} ", _0)]
    Operand(MeasurementOp)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum MeasurementOp {
    #[display(fmt = "+")]
    Add,
    #[display(fmt = "-")]
    Sub,
    #[display(fmt = "*")]
    Mul,
    #[display(fmt = "/")]
    Div,
}
// endregion

impl Measurement {
    pub const MAX_NUM_TERMS: usize = 8;
    pub const NULL: Measurement = Measurement {
        terms: [MeasurementTerm::Null; Self::MAX_NUM_TERMS]
    };

    pub fn const_(value: f64) -> Measurement {
        let mut this = Measurement::NULL.clone();
        this.terms[0] = MeasurementTerm::Const(value);
        this
    }

    pub fn as_const(&self) -> Option<f64> {
        if !self.terms[1].is_null() {
            return None;
        }
        self.terms[0].as_const()
    }
}

impl Measurement {
    pub fn eval_result<Error>(&self, mut read_var: impl FnMut(VarIndex) -> Result<f64, Error>) -> Result<f64, Error> {
        let mut stack = ArrayVec::<f64, { Self::MAX_NUM_TERMS }>::new();
        for term in self.terms {
            match term {
                MeasurementTerm::Null => break,
                MeasurementTerm::Const(const_) => stack.push(const_),
                MeasurementTerm::Var(var) => stack.push(read_var(var)?),
                MeasurementTerm::Operand(op) => {
                    let lhs = stack.pop().expect("malformed measurement (no lhs)");
                    let rhs = stack.pop().expect("malformed measurement (no rhs)");
                    let compute = op.compute(lhs, rhs);
                    stack.push(compute);
                }
            }
        }
        debug_assert!(stack.len() == 1, "malformed measurement (final stack len != 1)");
        Ok(stack.pop().unwrap())
    }

    pub fn eval(&self, mut read_var: impl FnMut(VarIndex) -> f64) -> f64 {
        self.eval_result::<Infallible>(|var| Ok(read_var(var))).unwrap()
    }

    pub fn check<Error>(&self, mut check_var: impl FnMut(VarIndex) -> Result<(), Error>) -> Result<(), Error> {
        self.eval_result::<Error>(|var| {
            check_var(var)?;
            Ok(f64::NAN)
        }).map(|_| ())
    }
}

impl InfixUnformedMeasurement {
    // Needs to be at least something like this or converting Measurement to InfixUnformedMeasurement will crash
    pub const MAX_NUM_TERMS: usize = Measurement::MAX_NUM_TERMS * 2;
    pub const NULL: InfixUnformedMeasurement = InfixUnformedMeasurement {
        terms: [InfixMeasurementTerm::Null; Self::MAX_NUM_TERMS]
    };

    pub fn const_(value: f64) -> InfixUnformedMeasurement {
        let mut this = InfixUnformedMeasurement::NULL.clone();
        this.terms[0] = InfixMeasurementTerm::Const(value);
        this
    }

    pub fn as_const(&self) -> Option<f64> {
        if !self.terms[1].is_null() {
            return None;
        }
        self.terms[0].as_const()
    }
}

impl MeasurementTerm {
    pub fn is_null(&self) -> bool {
        match self {
            MeasurementTerm::Null => true,
            _ => false
        }
    }

    pub fn as_const(&self) -> Option<f64> {
        match self {
            MeasurementTerm::Const(c) => Some(*c),
            _ => None
        }
    }
}

impl InfixMeasurementTerm {
    pub fn is_null(&self) -> bool {
        match self {
            InfixMeasurementTerm::Null => true,
            _ => false
        }
    }

    pub fn as_const(&self) -> Option<f64> {
        match self {
            InfixMeasurementTerm::Const(c) => Some(*c),
            _ => None
        }
    }
}

impl MeasurementOp {
    pub fn compute(&self, lhs: f64, rhs: f64) -> f64 {
        match self {
            MeasurementOp::Add => lhs + rhs,
            MeasurementOp::Sub => lhs - rhs,
            MeasurementOp::Mul => lhs * rhs,
            MeasurementOp::Div => lhs / rhs,
        }
    }
}

// region convert to/from infix unformed measurement
#[derive(Debug, Display, Error)]
pub enum MeasurementCompileError {
    #[display = "too many terms"]
    TooManyTerms,
    #[display = "expected op"]
    ExpectedOp,
    #[display = "unexpected op"]
    UnexpectedOp,
    #[display = "unexpected right paren"]
    UnexpectedRParen,
    #[display = "unclosed ("]
    UnclosedLParen,
    #[display = "unopened )"]
    UnopenedRParen,
    #[display = "expected rhs"]
    UnexpectedEnd
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MeasurementTermPos {
    Lhs,
    Op,
    Rhs
}

enum MeasurementTermTree {
    Leaf(MeasurementTerm),
    Node {
        lhs_idx: i32,
        op: MeasurementOp,
        rhs_idx: i32
    }
}

impl MeasurementTermTree {
    fn lhs_idx(&self) -> i32 {
        match self {
            MeasurementTermTree::Node { lhs_idx, .. } => *lhs_idx,
            MeasurementTermTree::Leaf(_) => -1
        }
    }

    fn set_rhs_idx(&mut self, new_rhs_idx: i32) {
        match self {
            MeasurementTermTree::Leaf(_) => panic!("MeasurementTermTree::set_rhs_idx called on leaf"),
            MeasurementTermTree::Node { rhs_idx, .. } => {
                *rhs_idx = new_rhs_idx;
            }
        }
    }
}

impl<'a> TryFrom<&'a InfixUnformedMeasurement> for Measurement {
    type Error = MeasurementCompileError;

    fn try_from(unformed: &'a InfixUnformedMeasurement) -> Result<Self, Self::Error> {
        let mut measurement = Self::NULL;

        // Converting from infix to postfix is done:
        // 1) construct a tree
        // 2) run a postorder traversal
        // To avoid allocations, we use indices instead of boxes

        // 1) construct a tree
        // TODO: order of operations
        let mut tree_arena = ArrayVec::<MeasurementTermTree, { InfixUnformedMeasurement::MAX_NUM_TERMS }>::new();
        let root_idx = {
            let mut idx_stack = ArrayVec::<i32, { Measurement::MAX_NUM_TERMS }>::new();
            let mut open_parens = ArrayVec::<MeasurementTermPos, { Measurement::MAX_NUM_TERMS }>::new();
            let mut pos = MeasurementTermPos::Lhs;
            for term in unformed.terms {
                match term {
                    InfixMeasurementTerm::Null => break,
                    InfixMeasurementTerm::Const(c) => {
                        if pos == MeasurementTermPos::Op {
                            return Err(MeasurementCompileError::ExpectedOp);
                        }

                        let new_idx = tree_arena.len() as i32;
                        tree_arena.try_push(MeasurementTermTree::Leaf(MeasurementTerm::Const(c))).map_err(|_| MeasurementCompileError::TooManyTerms)?;

                        match pos {
                            MeasurementTermPos::Lhs => {
                                idx_stack.push(new_idx);
                            }
                            MeasurementTermPos::Op => unreachable!(),
                            MeasurementTermPos::Rhs => {
                                let op_idx = idx_stack.pop().expect("pos == MeasurementTermPos::Rhs should imply there is an idx on the stack");
                                let lhs_idx = idx_stack.pop().expect("pos == MeasurementTermPos::Rhs should imply there are 2 idxs on the stack");
                                debug_assert!(tree_arena[op_idx as usize].lhs_idx() == lhs_idx, "pos == MeasurementTermPos::Rhs should imply the op has lhs_idx set");
                                tree_arena[op_idx as usize].set_rhs_idx(new_idx);
                                idx_stack.push(op_idx);
                            }
                        }
                        pos = MeasurementTermPos::Op;
                    },
                    InfixMeasurementTerm::Var(var) => {
                        if pos == MeasurementTermPos::Op {
                            return Err(MeasurementCompileError::ExpectedOp);
                        }

                        let new_idx = tree_arena.len() as i32;
                        tree_arena.try_push(MeasurementTermTree::Leaf(MeasurementTerm::Var(var))).map_err(|_| MeasurementCompileError::TooManyTerms)?;

                        match pos {
                            MeasurementTermPos::Lhs => {
                                idx_stack.push(new_idx);
                            }
                            MeasurementTermPos::Op => unreachable!(),
                            MeasurementTermPos::Rhs => {
                                let op_idx = idx_stack.pop().expect("pos == MeasurementTermPos::Rhs should imply there is an idx on the stack");
                                let lhs_idx = idx_stack.pop().expect("pos == MeasurementTermPos::Rhs should imply there are 2 idxs on the stack");
                                debug_assert!(tree_arena[op_idx as usize].lhs_idx() == lhs_idx, "pos == MeasurementTermPos::Rhs should imply the op has lhs_idx set");
                                tree_arena[op_idx as usize].set_rhs_idx(new_idx);
                                idx_stack.push(op_idx);
                            }
                        }
                        pos = MeasurementTermPos::Op;
                    },
                    InfixMeasurementTerm::Operand(op) => {
                        if pos != MeasurementTermPos::Op {
                            return Err(MeasurementCompileError::UnexpectedOp)
                        }

                        let new_idx = tree_arena.len() as i32;
                        tree_arena.try_push(MeasurementTermTree::Node {
                            lhs_idx: *idx_stack.last().expect("pos == MeasurementTermPos::Op should imply there is an idx on the stack"),
                            op,
                            rhs_idx: -1
                        }).map_err(|_| MeasurementCompileError::TooManyTerms)?;

                        idx_stack.push(new_idx);
                        pos = MeasurementTermPos::Rhs;
                    },
                    InfixMeasurementTerm::LParen => {
                        if pos == MeasurementTermPos::Op {
                            return Err(MeasurementCompileError::ExpectedOp)
                        }
                        // Note that parend expression in lhs position is semantically meaningless
                        // as we are left-associative by default
                        open_parens.push(pos);
                        pos = MeasurementTermPos::Lhs;
                    }
                    InfixMeasurementTerm::RParen => {
                        if pos != MeasurementTermPos::Op {
                            return Err(MeasurementCompileError::UnexpectedRParen)
                        }
                        if open_parens.is_empty() {
                            return Err(MeasurementCompileError::UnopenedRParen)
                        }
                        let open_paren_pos = open_parens.pop().unwrap();
                        // Note that parend expression in lhs position is semantically meaningless
                        // as we are left-associative by default
                        match open_paren_pos {
                            MeasurementTermPos::Lhs => {},
                            MeasurementTermPos::Op => unreachable!(),
                            MeasurementTermPos::Rhs => {
                                let new_idx = idx_stack.pop().expect("pos == MeasurementTermPos::Rhs should imply there is an idx on the stack");
                                let op_idx = idx_stack.pop().expect("pos == MeasurementTermPos::Rhs should imply there are 3 idxs on the stack");
                                let lhs_idx = idx_stack.pop().expect("pos == MeasurementTermPos::Rhs in InfixMeasurementTerm::RParen should imply there are 4 idxs on the stack");
                                debug_assert!(tree_arena[op_idx as usize].lhs_idx() == lhs_idx, "pos == MeasurementTermPos::Rhs should imply the op has lhs_idx set");
                                tree_arena[op_idx as usize].set_rhs_idx(new_idx);
                                idx_stack.push(op_idx);
                            }
                        }
                    }
                }
            }
            if !open_parens.is_empty() {
                return Err(MeasurementCompileError::UnclosedLParen);
            }
            if idx_stack.len() != 1 {
                return Err(MeasurementCompileError::UnexpectedEnd)
            }
            idx_stack.pop().unwrap()
        };

        // 2) run a postorder traversal
        {
            let mut terms = SliceVec::new(&mut measurement.terms);
            fn traverse(tree_arena: &mut ArrayVec<MeasurementTermTree, { InfixUnformedMeasurement::MAX_NUM_TERMS }>, terms: &mut SliceVec<MeasurementTerm>, idx: i32) -> Result<(), MeasurementCompileError> {
                match tree_arena[idx as usize] {
                    MeasurementTermTree::Leaf(term) => {
                        terms.push(term).map_err(|_| MeasurementCompileError::TooManyTerms)
                    },
                    MeasurementTermTree::Node { lhs_idx, op, rhs_idx } => {
                        traverse(tree_arena, terms, lhs_idx)?;
                        traverse(tree_arena, terms, rhs_idx)?;
                        terms.push(MeasurementTerm::Operand(op)).map_err(|_| MeasurementCompileError::TooManyTerms)
                    }
                }
            }
            traverse(&mut tree_arena, &mut terms, root_idx)?;
        }

        Ok(measurement)
    }
}

impl<'a> From<&'a Measurement> for InfixUnformedMeasurement {
    fn from(measurement: &'a Measurement) -> Self {
        // Converting from postfix to infix is done:
        // 1) construct a tree
        // 2) run an inorder traversal
        // To avoid allocations, we use indices instead of boxes

        // 1) construct a tree
        let mut tree_arena = ArrayVec::<MeasurementTermTree, { InfixUnformedMeasurement::MAX_NUM_TERMS }>::new();
        let root_idx = {
            let mut idx_stack = ArrayVec::<i32, { Measurement::MAX_NUM_TERMS }>::new();
            for term in measurement.terms {
                let new_idx = tree_arena.len() as i32;
                match term {
                    MeasurementTerm::Null => break,
                    MeasurementTerm::Const(c) => tree_arena.push(MeasurementTermTree::Leaf(MeasurementTerm::Const(c))),
                    MeasurementTerm::Var(var) => tree_arena.push(MeasurementTermTree::Leaf(MeasurementTerm::Var(var))),
                    MeasurementTerm::Operand(op) => {
                        let rhs_idx = idx_stack.pop().expect("measurement is malformed: operation with no lhs");
                        let lhs_idx = idx_stack.pop().expect("measurement is malformed: operation with no rhs");
                        tree_arena.push(MeasurementTermTree::Node {
                            lhs_idx,
                            op,
                            rhs_idx
                        });
                    }
                }
                idx_stack.push(new_idx);
            }
            debug_assert!(idx_stack.len() == 1);
            idx_stack.pop().unwrap()
        };

        // 2) run an inorder postorder traversal
        // TODO: remove extra parens
        let mut this = InfixUnformedMeasurement::NULL.clone();
        {
            let mut terms = SliceVec::new(&mut this.terms);
            fn traverse(tree_arena: &mut ArrayVec<MeasurementTermTree, { InfixUnformedMeasurement::MAX_NUM_TERMS }>, terms: &mut SliceVec<InfixMeasurementTerm>, idx: i32) {
                match tree_arena[idx as usize] {
                    MeasurementTermTree::Leaf(term) => {
                        terms.push(term.into()).expect("infix measurement len is not large enough relative to measurement len, so we ran out of space encoding a measurement");
                    },
                    MeasurementTermTree::Node { lhs_idx, op, rhs_idx } => {
                        terms.push(InfixMeasurementTerm::LParen).expect("infix measurement len is not large enough relative to measurement len, so we ran out of space encoding a measurement");
                        traverse(tree_arena, terms, lhs_idx);
                        terms.push(InfixMeasurementTerm::Operand(op)).expect("infix measurement len is not large enough relative to measurement len, so we ran out of space encoding a measurement");
                        traverse(tree_arena, terms, rhs_idx);
                        terms.push(InfixMeasurementTerm::RParen).expect("infix measurement len is not large enough relative to measurement len, so we ran out of space encoding a measurement");
                    }
                }
            }
            traverse(&mut tree_arena, &mut terms, root_idx);
        }
        this
    }
}

impl Into<InfixMeasurementTerm> for MeasurementTerm {
    fn into(self) -> InfixMeasurementTerm {
        match self {
            MeasurementTerm::Null => InfixMeasurementTerm::Null,
            MeasurementTerm::Const(const_) => InfixMeasurementTerm::Const(const_),
            MeasurementTerm::Var(var) => InfixMeasurementTerm::Var(var),
            MeasurementTerm::Operand(op) => InfixMeasurementTerm::Operand(op)
        }
    }
}
// endregion

// region serde
#[derive(Debug, PartialEq, Logos)]
enum MeasurementToken {
    #[regex(r"[0-9]+|[0-9]*\.?[0-9]+")]
    Const,

    #[regex("[a-zA-Z_][a-zA-Z0-9_.]*")]
    Var,

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,

    #[token("+")]
    Add,
    #[token("-")]
    Sub,
    #[token("*")]
    Mul,
    #[token("/")]
    Div,

    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum MeasurementParseErrorType {
    #[display(fmt = "too many terms")]
    TooManyTerms,
    #[display(fmt = "syntax error")]
    InvalidToken,
    #[display(fmt = "var not found")]
    VarNotFound
}

#[derive(Debug, Display, Error, From)]
pub enum MeasurementBuildError {
    Parse(MeasurementParseError),
    Compile(MeasurementCompileError)
}

pub type MeasurementParseError = ParseError<MeasurementParseErrorType>;

impl Serialize for InfixUnformedMeasurement {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.as_const() {
            None => serializer.serialize_str(&self.to_string()),
            Some(const_) => serializer.serialize_f64(const_)
        }
    }
}

impl<'de> Deserialize<'de> for InfixUnformedMeasurement {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MyVisitor;

        impl<'de> Visitor<'de> for MyVisitor {
            type Value = InfixUnformedMeasurement;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a number or string")
            }

            fn visit_i32<E: serde::de::Error>(self, v: i32) -> Result<Self::Value, E> {
                Ok(InfixUnformedMeasurement::const_(v as f64))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(InfixUnformedMeasurement::const_(v as f64))
            }

            fn visit_u32<E: serde::de::Error>(self, v: u32) -> Result<Self::Value, E> {
                Ok(InfixUnformedMeasurement::const_(v as f64))
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(InfixUnformedMeasurement::const_(v as f64))
            }

            fn visit_f32<E: serde::de::Error>(self, v: f32) -> Result<Self::Value, E> {
                Ok(InfixUnformedMeasurement::const_(v as f64))
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(InfixUnformedMeasurement::const_(v))
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                InfixUnformedMeasurement::try_from(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(MyVisitor)
    }
}

impl<'a> TryFrom<&'a str> for InfixUnformedMeasurement {
    type Error = MeasurementParseError;

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        let mut result = InfixUnformedMeasurement::NULL.clone();

        let mut lex = MeasurementToken::lexer(str);
        let mut idx = 0;
        while let Some(token) = lex.next() {
            let span = lex.span();
            if idx >= InfixUnformedMeasurement::MAX_NUM_TERMS {
                return Err(ParseError {
                    reason: MeasurementParseErrorType::TooManyTerms,
                    span
                });
            }

            let slice = lex.slice();
            let term = match token {
                MeasurementToken::Const => {
                    InfixMeasurementTerm::Const(slice.parse::<f64>().map_err(|_err| ParseError {
                        reason: MeasurementParseErrorType::InvalidToken,
                        span
                    })?)
                }
                MeasurementToken::Var => {
                    let index = GlobalVarEnvironment.get(slice).ok_or(ParseError {
                        reason: MeasurementParseErrorType::VarNotFound,
                        span
                    })?;
                    InfixMeasurementTerm::Var(index)
                }
                MeasurementToken::LParen => InfixMeasurementTerm::LParen,
                MeasurementToken::RParen => InfixMeasurementTerm::RParen,
                MeasurementToken::Add => InfixMeasurementTerm::Operand(MeasurementOp::Add),
                MeasurementToken::Sub => InfixMeasurementTerm::Operand(MeasurementOp::Sub),
                MeasurementToken::Mul => InfixMeasurementTerm::Operand(MeasurementOp::Mul),
                MeasurementToken::Div => InfixMeasurementTerm::Operand(MeasurementOp::Div),
                MeasurementToken::Error => return Err(ParseError {
                    reason: MeasurementParseErrorType::InvalidToken,
                    span
                })
            };
            result.terms[idx] = term;

            idx += 1
        }

        Ok(result)
    }
}

impl Into<String> for InfixUnformedMeasurement {
    fn into(self) -> String {
        self.to_string()
    }
}

impl Display for InfixUnformedMeasurement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for term in self.terms.iter() {
            match term {
                InfixMeasurementTerm::Null => continue,
                InfixMeasurementTerm::Const(c) => write!(f, "{}", c),
                InfixMeasurementTerm::Var(v) => GlobalVarEnvironment.with_name(*v, |name| match name {
                    None => write!(f, "#{}", v),
                    Some(name) => write!(f, "{}", name)
                }),
                InfixMeasurementTerm::LParen => write!(f, "("),
                InfixMeasurementTerm::RParen => write!(f, ")"),
                InfixMeasurementTerm::Operand(op) => write!(f, " {} ", op),
            }?;
        }
        Ok(())
    }
}

impl<'a> TryFrom<&'a str> for Measurement {
    type Error = MeasurementBuildError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Ok(Self::try_from(&InfixUnformedMeasurement::try_from(value)?)?)
    }
}

impl Into<String> for Measurement {
    fn into(self) -> String {
        InfixUnformedMeasurement::from(&self).into()
    }
}

impl Display for Measurement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", InfixUnformedMeasurement::from(self))
    }
}

impl Serialize for Measurement {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        InfixUnformedMeasurement::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Measurement {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let infix = InfixUnformedMeasurement::deserialize(deserializer)?;
        Self::try_from(&infix).map_err(|err| serde::de::Error::custom(err))
    }
}
// endregion