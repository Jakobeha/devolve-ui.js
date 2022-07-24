//! Small string stored inline

use arrayvec::{ArrayString, CapacityError};
use std::fmt::{Display, Formatter, Write};
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

/// Small string stored inline
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", repr(transparent))]
pub struct Ident(ArrayString<{ Ident::SIZE }>);

impl Ident {
    pub const SIZE: usize = 8;
}

impl Display for Ident {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl <'a> TryFrom<&'a str> for Ident {
    type Error = CapacityError<&'a str>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Ok(Ident(ArrayString::try_from(value)?))
    }
}

impl TryFrom<String> for Ident {
    type Error = CapacityError<String>;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        let mut str = ArrayString::new();
        write!(str, "{}", string).map_err(|_err| CapacityError::new(string))?;
        Ok(Ident(str))
    }
}