pub mod set {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use crate::file::{Interface, GlobalVarEnvironment};

    fn set_global_vars(interface: &Interface) {
        GlobalVarEnvironment.set(interface.keys().cloned())
    }

    pub fn serialize<S: Serializer>(table: &Interface, serializer: S) -> Result<S::Ok, S::Error> {
        let result = table.serialize(serializer)?;
        set_global_vars(table);
        Ok(result)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Interface, D::Error> {
        let table = Interface::deserialize(deserializer)?;
        set_global_vars(&table);
        Ok(table)
    }
}

pub mod clear {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use crate::file::GlobalVarEnvironment;

    fn clear_global_vars() {
        GlobalVarEnvironment.clear()
    }

    pub fn serialize<T: Serialize, S: Serializer>(table: &T, serializer: S) -> Result<S::Ok, S::Error> {
        let result = table.serialize(serializer)?;
        clear_global_vars();
        Ok(result)
    }

    pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(deserializer: D) -> Result<T, D::Error> {
        let result = T::deserialize(deserializer)?;
        clear_global_vars();
        Ok(result)
    }
}