//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 07_network#web-ws-runtime
//!
//! JSON payload adapters for binary protocol frames.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

pub(crate) mod vec {
    use super::*;

    pub fn serialize<S>(value: &[serde_json::Value], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            value.serialize(serializer)
        } else {
            serde_json::to_string(value)
                .map_err(serde::ser::Error::custom)?
                .serialize(serializer)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<serde_json::Value>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            Vec::<serde_json::Value>::deserialize(deserializer)
        } else {
            let raw = String::deserialize(deserializer)?;
            serde_json::from_str(&raw).map_err(D::Error::custom)
        }
    }
}

pub(crate) mod option {
    use super::*;

    pub fn serialize<S>(value: &Option<serde_json::Value>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            value.serialize(serializer)
        } else {
            value
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(serde::ser::Error::custom)?
                .serialize(serializer)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<serde_json::Value>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            Option::<serde_json::Value>::deserialize(deserializer)
        } else {
            let raw = Option::<String>::deserialize(deserializer)?;
            raw.map(|raw| serde_json::from_str(&raw).map_err(D::Error::custom))
                .transpose()
        }
    }
}
