use libp2p::gossipsub::TopicHash;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipConfig {
    /// Time between heartbeats
    pub heartbeat_interval: std::time::Duration,
    /// Number of peers to maintain in mesh
    pub mesh_n: usize,
    /// Number of peers to include in gossip propagation
    pub gossip_factor: f64,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: std::time::Duration::from_secs(1),
            mesh_n: 6,
            gossip_factor: 0.25,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GossipMessage {
    pub source: PeerId,
    pub topics: Vec<TopicHash>,
    pub payload: Vec<u8>,
    pub data_type: MessageType,
    pub timestamp: i64,
}

// Add serialization manually
impl Serialize for GossipMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("GossipMessage", 5)?;
        state.serialize_field("source", &self.source.to_string())?;
        state.serialize_field("topics", &self.topics.iter().map(|t| t.to_string()).collect::<Vec<_>>())?;
        state.serialize_field("payload", &self.payload)?;
        state.serialize_field("data_type", &self.data_type)?;
        state.serialize_field("timestamp", &self.timestamp)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for GossipMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { Source, Topics, Payload, DataType, Timestamp }

        struct GossipMessageVisitor;

        impl<'de> Visitor<'de> for GossipMessageVisitor {
            type Value = GossipMessage;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct GossipMessage")
            }

            fn visit_map<V>(self, mut map: V) -> Result<GossipMessage, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut source = None;
                let mut topics = None;
                let mut payload = None;
                let mut data_type = None;
                let mut timestamp = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Source => {
                            let s: String = map.next_value()?;
                            source = Some(s.parse().map_err(de::Error::custom)?);
                        }
                        Field::Topics => {
                            let t: Vec<String> = map.next_value()?;
                            topics = Some(t.into_iter().map(|s| TopicHash::from_raw(s)).collect());
                        }
                        Field::Payload => payload = Some(map.next_value()?),
                        Field::DataType => data_type = Some(map.next_value()?),
                        Field::Timestamp => timestamp = Some(map.next_value()?),
                    }
                }

                Ok(GossipMessage {
                    source: source.ok_or_else(|| de::Error::missing_field("source"))?,
                    topics: topics.ok_or_else(|| de::Error::missing_field("topics"))?,
                    payload: payload.ok_or_else(|| de::Error::missing_field("payload"))?,
                    data_type: data_type.ok_or_else(|| de::Error::missing_field("data_type"))?,
                    timestamp: timestamp.ok_or_else(|| de::Error::missing_field("timestamp"))?,
                })
            }
        }

        const FIELDS: &[&str] = &["source", "topics", "payload", "data_type", "timestamp"];
        deserializer.deserialize_struct("GossipMessage", FIELDS, GossipMessageVisitor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    BlockData,
    AccountUpdate,
    Transaction,
    ConsensusVote,
    PeerAnnouncement,
    HeartBeat,
}