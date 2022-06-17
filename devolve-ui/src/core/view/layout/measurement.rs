use std::fmt::{Display, Formatter};
use std::ops::{Add, Div, DivAssign, Mul, MulAssign, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Measurement {
    adds: [Measurement1; Measurement::MAX_NUM_ADDS],
    pub store: Option<&'static str>
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Measurement1 {
    pub value: MeasurementValue,
    pub unit: MeasurementUnit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MeasurementValue {
    scalar: f32,
    debug_symbol: MeasurementDebugSymbol,
    debug_is_neg: bool
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MeasurementDebugSymbol {
    Empty,
    Literal,
    Expr(&'static str),
    Lost
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MeasurementUnit {
    Units,
    Pixels,
    PercentOfParent,
    OfPrev,
    OfLoad(&'static str)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooManyAddsError;

pub type MeasurementResult = Result<Measurement, TooManyAddsError>;

impl Measurement {
    pub const MAX_NUM_ADDS: usize = 5;

    pub const ZERO: Measurement = Measurement {
        adds: [Measurement1::BLANK; Measurement::MAX_NUM_ADDS],
        store: None
    };

    pub fn iter_adds(&self) -> impl Iterator<Item = Measurement1> + '_ {
        self.adds.iter().filter(|add| !add.is_blank()).copied()
    }
}

impl Measurement1 {
    pub const BLANK: Measurement1 = Measurement1 {
        value: MeasurementValue::BLANK,
        unit: MeasurementUnit::Units
    };

    /// Whether the measurement value represents a blank space in `Measurement::adds`
    pub const fn is_blank(self) -> bool {
        self.value.is_blank()
    }

    pub const fn is_neg(self) -> bool {
        self.value.is_neg()
    }
}

impl MeasurementValue {
    pub const BLANK: MeasurementValue = MeasurementValue {
        scalar: 0f32,
        debug_symbol: MeasurementDebugSymbol::Empty,
        debug_is_neg: false
    };

    pub const fn new(scalar: f32, debug_symbol: MeasurementDebugSymbol) -> Self {
        MeasurementValue {
            scalar,
            debug_symbol,
            debug_is_neg: false
        }
    }

    /// Whether the measurement value represents a blank space in `Measurement::adds`
    pub const fn is_blank(self) -> bool {
        match self.debug_symbol {
            MeasurementDebugSymbol::Empty => true,
            _ => false
        }
    }

    /// Whether the measurement value is a literal
    pub const fn is_literal(self) -> bool {
        match self.debug_symbol {
            MeasurementDebugSymbol::Literal => true,
            _ => false
        }
    }

    pub const fn is_neg(self) -> bool {
        self.scalar.is_sign_negative()
    }

    pub const fn scalar(self) -> f32 {
        self.scalar
    }
}

impl TryFrom<Vec<Measurement1>> for Measurement {
    type Error = TooManyAddsError;

    fn try_from(adds: Vec<Measurement1>) -> MeasurementResult {
        let mut measurement = Measurement::ZERO;
        if adds.len() > Measurement::MAX_NUM_ADDS {
            return Err(TooManyAddsError);
        }
        for (i, add) in adds.into_iter().enumerate() {
            measurement.adds[i] = add;
        }
        Ok(measurement)
    }
}

// region Display impls
impl Display for Measurement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(store) = self.store {
            write!(f, "{} = ", store)?;
        }
        let mut written = false;
        for add in self.adds.iter() {
            if !add.is_blank() {
                if written {
                    if add.is_neg() {
                        write!(f, " - {}", -*add)?;
                    } else {
                        write!(f, " + {}", add)?;
                    }
                } else {
                    written = true;
                    write!(f, "{}", add)?;
                }
            }
        }
        if !written {
            write!(f, "0")?;
        }
        Ok(())
    }
}

impl Display for Measurement1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_blank() {
            return Ok(());
        }

        match self.unit {
            MeasurementUnit::Units => write!(f, "{}", self.value),
            MeasurementUnit::Pixels => write!(f, "{}px", self.value),
            MeasurementUnit::PercentOfParent => write!(f, "{}%", self.value),
            MeasurementUnit::OfPrev if self.value.scalar == 1f32 && self.value.is_literal() => write!(f, "prev"),
            MeasurementUnit::OfPrev => write!(f, "{}*prev", self.value),
            MeasurementUnit::OfLoad(id) if self.value.scalar == 1f32 && self.value.is_literal() => write!(f, "load({})", id),
            MeasurementUnit::OfLoad(id) => write!(f, "{}*load({})", self.value, id)
        }
    }
}

impl Display for MeasurementValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.debug_symbol {
            MeasurementDebugSymbol::Empty => write!(f, "0"),
            MeasurementDebugSymbol::Literal => write!(f, "{}", self.scalar),
            MeasurementDebugSymbol::Expr(debug_symbol) if self.debug_is_neg => write!(f, "{}{{-({})}}", self.scalar, debug_symbol),
            MeasurementDebugSymbol::Expr(debug_symbol) => write!(f, "{}{{{}}}", self.scalar, debug_symbol),
            MeasurementDebugSymbol::Lost => write!(f, "{}{{...}}", self.scalar)
        }
    }
}

impl Display for MeasurementUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MeasurementUnit::Units => write!(f, ""),
            MeasurementUnit::Pixels => write!(f, "px"),
            MeasurementUnit::PercentOfParent => write!(f, "parent"),
            MeasurementUnit::OfPrev => write!(f, "prev"),
            MeasurementUnit::OfLoad(load) => write!(f, "load({})", load)
        }
    }
}

impl Display for TooManyAddsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "too many addition operands in measurement, we only allow up to {}", Measurement::MAX_NUM_ADDS)
    }
}
// endregion

// region Default impls
impl const Default for Measurement {
    fn default() -> Self {
        Measurement::ZERO
    }
}

impl const Default for Measurement1 {
    fn default() -> Self {
        Measurement1::BLANK
    }
}

impl const Default for MeasurementValue {
    fn default() -> Self {
        MeasurementValue::BLANK
    }
}

impl const Default for MeasurementUnit {
    fn default() -> Self {
        MeasurementUnit::Units
    }
}
// endregion

// region arithmetic impls
impl const Add<Measurement> for Measurement {
    type Output = MeasurementResult;

    fn add(mut self, other: Measurement) -> Self::Output {
        let mut i = 0;
        while i < Measurement::MAX_NUM_ADDS {
            let add = other.adds[i];
            self = (self + add)?;
            i += 1;
        }
        Ok(self)
    }
}

impl const Add<Measurement1> for Measurement {
    type Output = MeasurementResult;

    fn add(mut self, add: Measurement1) -> Self::Output {
        if add.is_blank() {
            return Ok(self);
        }
        let mut i = 0;
        while i < Measurement::MAX_NUM_ADDS {
            if self.adds[i].is_blank() {
                self.adds[i] = add;
                return Ok(self);
            }
            i += 1;
        }
        return Err(TooManyAddsError)
    }
}

impl const Sub<Measurement> for Measurement {
    type Output = MeasurementResult;

    fn sub(mut self, other: Measurement) -> Self::Output {
        let mut i = 0;
        while i < Measurement::MAX_NUM_ADDS {
            let add = other.adds[i];
            self = (self - add)?;
            i += 1;
        }
        Ok(self)
    }
}

impl const Sub<Measurement1> for Measurement {
    type Output = MeasurementResult;

    fn sub(mut self, add: Measurement1) -> Self::Output {
        if add.is_blank() {
            return Ok(self);
        }
        let mut i = 0;
        while i < Measurement::MAX_NUM_ADDS {
            if self.adds[i].is_blank() {
                self.adds[i] = -add;
                return Ok(self);
            }
            i += 1;
        }
        Err(TooManyAddsError)
    }
}

impl const Mul<f32> for Measurement {
    type Output = Measurement;

    fn mul(mut self, other: f32) -> Self::Output {
        self *= other;
        self
    }
}

impl const Div<f32> for Measurement {
    type Output = Measurement;

    fn div(mut self, other: f32) -> Self::Output {
        self /= other;
        self
    }
}

impl const MulAssign<f32> for Measurement {
    fn mul_assign(&mut self, other: f32) {
        let mut i = 0;
        while i < Measurement::MAX_NUM_ADDS {
            self.adds[i] *= other;
            i += 1;
        }
    }
}

impl const DivAssign<f32> for Measurement {
    fn div_assign(&mut self, other: f32) {
        let mut i = 0;
        while i < Measurement::MAX_NUM_ADDS {
            self.adds[i] /= other;
            i += 1;
        }
    }
}

impl const Mul<f32> for Measurement1 {
    type Output = Measurement1;

    fn mul(mut self, other: f32) -> Self::Output {
        self *= other;
        self
    }
}

impl const Div<f32> for Measurement1 {
    type Output = Measurement1;

    fn div(mut self, other: f32) -> Self::Output {
        self /= other;
        self
    }
}

impl const MulAssign<f32> for Measurement1 {
    fn mul_assign(&mut self, other: f32) {
        self.value *= other;
    }
}

impl const DivAssign<f32> for Measurement1 {
    fn div_assign(&mut self, other: f32) {
        self.value /= other;
    }
}

impl const Mul<f32> for MeasurementValue {
    type Output = MeasurementValue;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}

impl const Div<f32> for MeasurementValue {
    type Output = MeasurementValue;

    fn div(mut self, rhs: f32) -> Self::Output {
        self /= rhs;
        self
    }
}

impl const MulAssign<f32> for MeasurementValue {
    fn mul_assign(&mut self, rhs: f32) {
        if rhs != 1f32 {
            self.debug_symbol = MeasurementDebugSymbol::Lost;
            self.scalar *= rhs;
        }
    }
}

impl const DivAssign<f32> for MeasurementValue {
    fn div_assign(&mut self, rhs: f32) {
        if rhs != 1f32 {
            self.debug_symbol = MeasurementDebugSymbol::Lost;
            self.scalar /= rhs;
        }
    }
}

impl const Neg for Measurement {
    type Output = Measurement;

    fn neg(mut self) -> Self::Output {
        let mut i = 0;
        while i < Measurement::MAX_NUM_ADDS {
            self.adds[i] = -self.adds[i];
            i += 1;
        }
        self
    }
}

impl const Neg for Measurement1 {
    type Output = Measurement1;

    fn neg(mut self) -> Self::Output {
        self.value = -self.value;
        self
    }
}

impl const Neg for MeasurementValue {
    type Output = MeasurementValue;

    fn neg(mut self) -> Self::Output {
        self.debug_is_neg = !self.debug_is_neg;
        self.scalar = -self.scalar;
        self
    }
}
// endregion