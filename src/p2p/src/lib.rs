//! # P2P Networking Layer for Nexu
//!
//! This crate implements the peer-to-peer networking logic for the Nexu application,
//! built on top of [iroh](https://iroh.computer/).
//!
//! It provides a [`ChatClient`] struct that handles:
//! - Connecting to the iroh network.
//! - Subscribing to gossip topics for group chats.
//! - Direct messaging (DM) between peers.
//! - File/Blob transfer (uploading and downloading) using iroh-blobs.
//! - Peer discovery and management.
//!
//! ## Key Components
//!
//! - **Client**: The [`ChatClient`] is the central struct managing connections,
//!   message sending/receiving, and blob storage.
//! - **Messages**: Defines the protocol message structures (e.g., [`ChatMessage`], [`BlobMessage`])
//!   serialized via `postcard`.
//! - **Protocol**: Implements the direct messaging protocol handler.
//! - **Types**: Shared types and utilities, such as invitation [`Ticket`]s.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use p2p::{ChatClient, load_secret_key};
//! use std::path::PathBuf;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let key_path = PathBuf::from("secret.key");
//! let secret_key = load_secret_key(key_path).await?;
//!
//! // Initialize the client
//! let client = ChatClient::new(secret_key, None).await?;
//!
//! // Start listening for incoming messages
//! client.listen();
//!
//! // Join a topic or send a DM...
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod messages;
pub mod protocol;
pub mod types;
pub mod utils;

pub use client::ChatClient;
pub use iroh::{EndpointAddr, EndpointId};
pub use messages::{
    BlobMessage, ChatMessage, DisconnectMessage, DmBlobMessage, DmChatMessage, DmJoinMessage,
    DmMessageTypes, DmProfileMetadataMessage, GossipMessage, JoinMessage, LeaveMessage,
    MessageTypes, TopicMessagesMessage, TopicMetadataMessage,
};
pub use types::Ticket;
pub use utils::load_secret_key;

pub use iroh_blobs::api::blobs::{AddProgress, AddProgressItem, ExportProgress};
pub use iroh_blobs::api::downloader::{DownloadProgress, DownloadProgressItem};

pub use iroh_blobs::BlobFormat::Raw;
pub use iroh_blobs::Hash;
pub use iroh_blobs::ticket::BlobTicket;
