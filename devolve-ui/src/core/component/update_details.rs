#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
#[cfg(feature = "logging")]
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct UpdateBacktrace(
    #[cfg(feature = "backtrace")]
    Option<String>,
    #[cfg(not(feature = "backtrace"))]
    (),
);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UpdateDetails {
    SetState {
        index: usize,
        backtrace: UpdateBacktrace
    },
    SetContextState {
        index: usize,
        backtrace: UpdateBacktrace
    }
}

impl UpdateBacktrace {
    #[cfg(feature = "backtrace")]
    fn from(backtrace: Option<&Backtrace>) -> Self {
        Self(backtrace.map(|backtrace| backtrace.to_string()))
    }

    #[cfg(not(feature = "backtrace"))]
    fn disabled() -> Self {
        Self(())
    }

    pub(in crate::core) fn here() -> Self {
        #[cfg(feature = "backtrace")]
        {
            Self::from(Some(&Backtrace::capture()))
        }
        #[cfg(not(feature = "backtrace"))]
        Self::disabled()
    }
}

#[derive(Debug)]
pub(super) struct UpdateStack(Vec<UpdateFrame>);

impl UpdateStack {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    fn last_open(&mut self) -> Option<&mut UpdateFrame> {
        self.0.last_mut().filter(|frame| frame.is_open)
    }

    fn last_open_or_make(&mut self) -> &mut UpdateFrame {
        if !self.0.last_mut().is_some_and(|frame| frame.is_open) {
            self.0.push(UpdateFrame::new())
        }
        self.last_open().unwrap()
    }

    pub fn add_to_last(&mut self, details: UpdateDetails) {
        self.last_open_or_make().add(details);
    }

    pub fn close_last(&mut self, log_last: impl FnOnce(&UpdateFrame)) {
        if let Some(last) = self.last_open() {
            last.close();
            log_last(last);
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct UpdateFrame {
    simultaneous: Vec<UpdateDetails>,
    #[cfg_attr(feature = "serde", serde(skip))]
    is_open: bool
}

impl UpdateFrame {
    fn new() -> Self {
        Self {
            simultaneous: Vec::new(),
            is_open: true
        }
    }

    fn close(&mut self) {
        assert!(self.is_open, "already closed");
        self.is_open = false;
    }

    fn add(&mut self, details: UpdateDetails) {
        assert!(self.is_open, "closed");
        self.simultaneous.push(details);
    }

    pub fn simultaneous(&self) -> &Vec<UpdateDetails> {
        &self.simultaneous
    }
}

impl Display for UpdateStack {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for frame in self.0.iter() {
            write!(f, "{}\n", frame)?;
        }
        write!(f, "---")?;
        Ok(())
    }
}

impl Display for UpdateFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut is_first = true;
        for detail in self.simultaneous.iter() {
            if is_first {
                is_first = false
            } else {
                write!(f, ", ")?;
            }
            write!(f, "{}", detail)?;
        }
        Ok(())
    }
}


impl Display for UpdateDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateDetails::SetState { index, backtrace } => {
                write!(f, "set:state:{} {}", index, backtrace)
            }
            UpdateDetails::SetContextState { index, backtrace } => {
                write!(f, "set:context-state:{} {}", index, backtrace)
            }
        }
    }
}

impl Display for UpdateBacktrace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        #[cfg(feature = "backtrace")]
        match &self.0 {
            None => write!(f, "(backtrace missing)"),
            Some(backtrace) => write!(f, "(backtrace)\n{}", backtrace)
        }
        #[cfg(not(feature = "backtrace"))]
        write!(f, "(backtrace disabled)")
    }
}