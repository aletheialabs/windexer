use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::fmt;

pub use agave_geyser_plugin_interface::geyser_plugin_interface::SlotStatus;

#[derive(Clone, Debug)]
pub struct SerializableSlotStatus(pub SlotStatus);

impl Serialize for SerializableSlotStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.as_str())
    }
}

impl<'de> Deserialize<'de> for SerializableSlotStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "processed" => Ok(SerializableSlotStatus(SlotStatus::Processed)),
            "confirmed" => Ok(SerializableSlotStatus(SlotStatus::Confirmed)),
            "rooted" => Ok(SerializableSlotStatus(SlotStatus::Rooted)),
            "firstShredReceived" => Ok(SerializableSlotStatus(SlotStatus::FirstShredReceived)),
            "completed" => Ok(SerializableSlotStatus(SlotStatus::Completed)),
            "createdBank" => Ok(SerializableSlotStatus(SlotStatus::CreatedBank)),
            "dead" => Ok(SerializableSlotStatus(SlotStatus::Dead(String::new()))),
            _ => Err(serde::de::Error::custom(format!("Unknown slot status: {}", s))),
        }
    }
}

impl From<SlotStatus> for SerializableSlotStatus {
    fn from(status: SlotStatus) -> Self {
        SerializableSlotStatus(status)
    }
}

impl From<SerializableSlotStatus> for SlotStatus {
    fn from(status: SerializableSlotStatus) -> Self {
        status.0
    }
}

impl fmt::Display for SerializableSlotStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
} 