use std::borrow::Cow;
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::component::path::VComponentKey;
use crate::view::view::VViewType;

#[derive(Debug, Clone)]
pub struct LayoutError {
    message: Cow<'static, str>,
    path: String,
}

pub type LayoutResult<T> = Result<T, LayoutError>;

impl LayoutError {
    pub fn new(message: impl Into<Cow<'static, str>>) -> LayoutError {
        LayoutError {
            message: message.into(),
            path: String::new(),
        }
    }

    pub fn add_dimension(&self, dimension: &str) -> LayoutError {
        LayoutError {
            message: self.message.clone(),
            path: format!("{}{}", dimension, self.path),
        }
    }

    pub fn add_store(&self, store: &str) -> LayoutError {
        LayoutError {
            message: self.message.clone(),
            path: format!("{}${}", self.path, store),
        }
    }

    pub fn add_component(&self, parent_key: &VComponentKey, parent_id: usize) -> LayoutError {
        LayoutError {
            message: self.message.clone(),
            path: format!("{}#{}.{}", parent_key.to_string(), parent_id, self.path),
        }
    }

    pub fn add_view(&self, parent_type: VViewType, parent_id: usize, index: usize) -> LayoutError {
        LayoutError {
            message: self.message.clone(),
            path: format!("{}#{}[{}].{}", parent_type.to_string(), parent_id, index, self.path),
        }
    }

     pub fn add_description(&self, description: &str) -> LayoutError {
         LayoutError {
             message: Cow::Owned(format!("{}: {}", description, self.message)),
             path: self.path.clone()
         }
     }
}

impl Display for LayoutError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}\nin {}", self.message, self.path)
    }
}