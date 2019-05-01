use crate::app::AppId;
use serde::{Deserialize, Serialize};

use super::Clock;

pub type MsgId = u64;
pub type Date = u64;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Header {
    Private(AppId),
    Public,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Msg {
    pub id: MsgId,
    pub header: Header,
    pub content: String,
    pub clock: Clock,
}

impl Msg {
    pub fn new(id: MsgId, header: Header, content: String, clock: Clock) -> Self {
        Msg {
            id,
            header,
            content,
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
            header: Header::Private("42".to_string()),
            content: "I like trains !".to_string(),
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
