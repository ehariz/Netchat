use crate::app::AppId;
use serde::{Deserialize, Serialize};

use super::Clock;

pub type MsgId = u64;
pub type Date = u64;

/// Header(Content)
/// Defines message type
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum Header {
    Private(AppId, String),
    Public(String),
    Connection,
    Disconnection,
    SnapshotRequest(AppId), // AppId used to identify snapshot requester
    SnapshotResponse(AppId, Vec<Msg>),
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Msg {
    pub id: MsgId,
    pub sender_id: AppId,
    pub header: Header,
    pub clock: Clock,
}

impl Msg {
    pub fn new(id: MsgId, sender_id: AppId, header: Header, clock: Clock) -> Self {
        Msg {
            id,
            sender_id,
            header,
            clock,
        }
    }
    pub fn serialize(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
    pub fn from_str(json: &str) -> serde_json::Result<Msg> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn message_serde() {
        let msg = Msg {
            id: 1,
            sender_id: "asdasdw".to_owned(),
            header: Header::Private("42".to_string(), "I like trains !".to_string()),
            clock: Clock(
                [("1".to_string(), 2), ("3".to_string(), 4)]
                    .iter()
                    .cloned()
                    .collect(),
            ),
        };

        let serialized = msg.serialize().expect("failed to serialize");
        println!("serialized = {}", serialized);

        // Convert the JSON string back to a Msg.
        let deserialized = Msg::from_str(&serialized).expect("failed to deserialize");

        println!("deserialized = {:?}", deserialized);

        assert_eq!(msg, deserialized);
    }
}
