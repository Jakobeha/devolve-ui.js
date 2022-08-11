use std::cell::RefCell;
use bimap::BiHashMap;
use log::info;
use derive_more::Display;

thread_local! {
    static VAR_ENVIRONMENT: RefCell<Option<BiHashMap<String, VarIndex>>> = RefCell::new(None);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub struct VarIndex(pub usize);

/// Thread-local environment to associate strings with [VarIndex]s,
/// used in deserialization.
pub struct GlobalVarEnvironment;

impl GlobalVarEnvironment {
    pub fn set(&self, entries: impl IntoIterator<Item = String>) {
        let entries = entries.into_iter();
        VAR_ENVIRONMENT.with_borrow_mut(|var_environment| {
            assert!(var_environment.is_none(), "var environment was already set");
            let mut new_environment = BiHashMap::new();
            for (index, key) in entries.enumerate() {
                let index = VarIndex(index);
                let overwritten = new_environment.insert(key, index);
                if overwritten.did_overwrite() {
                    info!("var environment contains duplicate keys")
                }
            }
            *var_environment = Some(new_environment);
        });
    }

    pub fn clear(&self) {
        VAR_ENVIRONMENT.with_borrow_mut(|var_environment| {
            assert!(var_environment.is_some(), "var environment was not set");
            *var_environment = None;
        });
    }

    pub fn get(&self, key: &str) -> Option<VarIndex> {
        VAR_ENVIRONMENT.with_borrow(|var_environment| {
            var_environment.as_ref().expect("var environment was not set").get_by_left(key).copied()
        })
    }

    pub fn with_name<R>(&self, index: VarIndex, fun: impl FnOnce(Option<&str>) -> R) -> R {
        VAR_ENVIRONMENT.with_borrow(|var_environment| {
            fun(var_environment.as_ref().expect("var environment was not set").get_by_right(&index).map(|string| string.as_str()))
        })
    }
}
