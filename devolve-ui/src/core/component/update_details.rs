#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};
#[cfg(feature = "logging")]
use serde::{Serialize, Deserialize};
use crate::core::component::path::VComponentKey;
use crate::core::hooks::provider::UntypedProviderId;

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
    CreateNew {
        key: VComponentKey,
        backtrace: UpdateBacktrace
    },
    Reuse {
        key: VComponentKey,
        backtrace: UpdateBacktrace
    },
    SetState {
        index: usize,
        backtrace: UpdateBacktrace
    },
    SetContextState {
        id: UntypedProviderId,
        backtrace: UpdateBacktrace
    },
    SetAtomicState {
        index: usize,
        backtrace: UpdateBacktrace
    },
    SetTreeState {
        origin: String
    },
    Custom {
        message: String
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct UpdateStack(Vec<UpdateFrame>);

impl UpdateStack {
    pub(super) fn new() -> Self {
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

    pub(super) fn add_to_last(&mut self, details: UpdateDetails) {
        self.last_open_or_make().add(details);
    }

    pub(super) fn add_all_to_last(&mut self, details: impl Iterator<Item=UpdateDetails>) {
        let mut last_open_or_make = None;
        for details in details {
            if last_open_or_make.is_none() {
                last_open_or_make = Some(self.last_open_or_make());
            }
            last_open_or_make.as_mut().unwrap().add(details);
        }
    }

    pub(super) fn append_to_last(&mut self, details: &mut Vec<UpdateDetails>) {
        if !details.is_empty() {
            self.last_open_or_make().append(details);
        }
    }

    pub(super) fn close_last(&mut self) {
        if let Some(last) = self.last_open() {
            last.close();
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn len(&self) -> usize {
        self.0.len()
    }

    pub(super) fn has_pending(&self) -> bool {
        self.0.last().is_some_and(|frame| frame.is_open)
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

    fn append(&mut self, detailss: &mut Vec<UpdateDetails>) {
        assert!(self.is_open, "closed");
        self.simultaneous.append(detailss);
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
            UpdateDetails::CreateNew { key, backtrace } => {
                write!(f, "create-new:{} {}", key, backtrace)
            },
            UpdateDetails::Reuse { key, backtrace } => {
                write!(f, "reuse:{} {}", key, backtrace)
            },
            UpdateDetails::SetState { index, backtrace } => {
                write!(f, "set:state:{} {}", index, backtrace)
            }
            UpdateDetails::SetContextState { id, backtrace } => {
                write!(f, "set:context-state:{:?} {}", *id, backtrace)
            }
            UpdateDetails::SetAtomicState { index, backtrace } => {
                write!(f, "set:atomic-state:{} {}", index, backtrace)
            }
            UpdateDetails::SetTreeState { origin } => {
                write!(f, "set:tree-state {}", origin)
            }
            UpdateDetails::Custom { message } => {
                write!(f, "custom {}", message)
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