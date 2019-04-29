use serde::{Deserialize, Serialize};

type MsgId = u64;
type UserId = u64;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Header {
    Private(UserId),
    Public,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Msg {
    pub id: MsgId,
    pub header: Header,
    pub content: String,
}

impl Msg {
    pub fn new(id: MsgId, header: Header, content: String) -> Self {
        Msg {
            id,
            header,
            content,
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
            header: Header::Private(42),
            content: "I like trains !".to_string(),
        };

        // Convert the Msg to a JSON string.
        let serialized = msg.serialize().unwrap();

        println!("serialized = {}", serialized);

        // Convert the JSON string back to a Msg.
        let deserialized = Msg::from_str(&serialized).unwrap();

        println!("deserialized = {:?}", deserialized);

        assert_eq!(msg, deserialized);
    }
}
