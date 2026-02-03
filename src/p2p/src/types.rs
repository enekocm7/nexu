//! # Shared Types
//!
//! This module defines common types used across the p2p crate, such as invitation tickets.

use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A ticket used to invite peers to a specific gossip topic.
///
/// It contains the `TopicId` to join and a list of `EndpointAddr`s (bootstrap nodes)
/// that are already part of the topic to help with initial connection.
///
/// Tickets can be serialized to a base58 string for easy sharing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ticket {
    /// The unique identifier of the gossip topic.
    pub topic: TopicId,
    /// A list of peers (with their addresses) to connect to for this topic.
    pub endpoints: Vec<EndpointAddr>,
}

impl fmt::Display for Ticket {
    /// Formats the ticket as a base58 string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = postcard::to_stdvec(self).map_err(|_| fmt::Error)?;
        let text = bs58::encode(bytes).into_string();
        write!(f, "{text}")
    }
}

impl FromStr for Ticket {
    type Err = anyhow::Error;

    /// Parses a ticket from a base58 string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(s).into_vec()?;
        let ticket = postcard::from_bytes(&bytes)?;
        Ok(ticket)
    }
}
