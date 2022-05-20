use std::borrow::Cow;
use std::collections::HashMap;
use crate::core::component::component::VComponentKey;
use crate::core::view::view::VViewType;

#[derive(Debug, Clone)]
pub struct LayoutError<'a> {
    message: Cow<'a, str>,
    path: String,
}

pub type LayoutResult<'a, T> = Result<T, LayoutError<'a>>;

impl LayoutError {
    pub fn new<'a>(message: impl Into<Cow<'a, str>>) -> LayoutError<'a> {
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
