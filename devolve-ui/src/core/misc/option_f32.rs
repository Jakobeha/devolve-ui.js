/// f32 with required checks for NaN
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct OptionF32(f32);

impl OptionF32 {
    pub fn into_option(self) -> Option<f32> {
        if self.0.is_nan() {
            None
        } else {
            Some(self.0)
        }
    }

    pub fn unwrap(self) -> f32 {
        if self.0.is_nan() {
            panic!("unwrap on NaN")
        } else {
            self.0
        }
    }

    pub fn unwrap_or(self, default: f32) -> f32 {
        if self.0.is_nan() {
            default
        } else {
            self.0
        }
    }

    pub fn unwrap_or_else(self, get_default: impl FnOnce() -> f32) -> f32 {
        if self.0.is_nan() {
            get_default()
        } else {
            self.0
        }
    }

    pub fn is_none(&self) -> bool {
        self.0.is_nan()
    }

    pub fn is_some(&self) -> bool {
        !self.0.is_nan()
    }

    pub fn map<F: FnOnce(f32) -> T, T>(self, f: F) -> Option<T> {
        if self.0.is_nan() {
            None
        } else {
            Some(f(self.0))
        }
    }

    pub fn map_or<F: FnOnce(f32) -> T, T>(self, default: T, f: F) -> T {
        if self.0.is_nan() {
            default
        } else {
            f(self.0)
        }
    }

    pub fn map_or_else<F: FnOnce(f32) -> T, T>(self, get_default: impl FnOnce() -> T, f: F) -> T {
        if self.0.is_nan() {
            get_default()
        } else {
            f(self.0)
        }
    }

    pub fn or(self, default: OptionF32) -> OptionF32 {
        if self.0.is_nan() {
            default
        } else {
            self
        }
    }

    pub fn or_else(self, get_default: impl FnOnce() -> OptionF32) -> OptionF32 {
        if self.0.is_nan() {
            get_default()
        } else {
            self
        }
    }

    pub fn is_some_and<F: FnOnce(f32) -> bool>(self, f: F) -> bool {
        if self.0.is_nan() {
            false
        } else {
            f(self.0)
        }
    }
}

impl From<f32> for OptionF32 {
    fn from(f: f32) -> Self {
        Self(f)
    }
}

impl From<Option<f32>> for OptionF32 {
    fn from(x: Option<f32>) -> Self {
        match x {
            Some(f) => Self(f),
            None => Self(f32::NAN),
        }
    }
}

impl From<OptionF32> for Option<f32> {
    fn from(x: OptionF32) -> Self {
        x.into_option()
    }
}