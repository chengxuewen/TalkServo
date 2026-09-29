//! Identity newtypes. `Arc<str>` keeps clones O(1) with no allocator churn on
//! the hot path; every wire struct embeds these by value.

use std::fmt;

macro_rules! id_newtype {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        pub struct $name(#[serde(rename = "id")] pub std::sync::Arc<str>);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(std::sync::Arc::from(s))
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(std::sync::Arc::from(s.as_str()))
            }
        }
    };
}

id_newtype!(
    /// Stable identifier of a connected peer (JWT `sub` claim echo).
    PeerId
);
id_newtype!(
    /// Stable identifier of a room (dispatch scope key).
    RoomId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_roundtrip_and_display() {
        let p = PeerId::from("peer-1");
        assert_eq!(p.to_string(), "peer-1");
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, r#""peer-1""#); // Arc<str> serializes as its string content
        let back: PeerId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn room_id_from_string_owns() {
        let owned = String::from("room-7");
        let r = RoomId::from(owned);
        assert_eq!(r.0.as_ref(), "room-7");
    }
}
