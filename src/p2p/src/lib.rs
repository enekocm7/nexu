use futures::StreamExt;
use libp2p::gossipsub::ValidationMode::Strict;
use libp2p::gossipsub::{IdentTopic, MessageId};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{gossipsub, mdns, noise, tcp, yamux, PeerId, Swarm, SwarmBuilder};
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: u64,
}

impl ChatMessage {
    pub fn new(sender: String, content: String, timestamp: u64) -> Self {
        ChatMessage {
            sender,
            content,
            timestamp,
        }
    }
}

#[derive(Debug)]
pub enum ChatEvent {
    MessageReceived(ChatMessage),
    PeerDiscovered(PeerId),
    PeerExpired(PeerId),
}

#[derive(NetworkBehaviour)]
pub struct ChatBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub struct ChatClient {
    swarm: Swarm<ChatBehaviour>,
    peer_id: PeerId,
    topic: IdentTopic,
    event_sender: UnboundedSender<ChatEvent>,
    event_receiver: Option<UnboundedReceiver<ChatEvent>>,
    message_receiver: Option<UnboundedReceiver<String>>,
    message_sender: UnboundedSender<String>,
}

impl ChatClient {
    pub async fn new(topic_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (message_sender, message_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let peer_id = key.public().to_peer_id();

                let message_id_fn = |message: &gossipsub::Message| {
                    let mut hasher = DefaultHasher::new();
                    message.data.hash(&mut hasher);
                    MessageId::from(hasher.finish().to_string())
                };

                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(1))
                    .validation_mode(Strict)
                    .message_id_fn(message_id_fn)
                    .build()
                    .map_err(|e| format!("Failed to create gossipsub config: {}", e))?;

                let mut gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )
                .expect("Failed to create gossipsub behaviour");

                let topic = IdentTopic::new(topic_name);

                gossipsub
                    .subscribe(&topic)
                    .map_err(|e| format!("Failed to subscribe to topic: {}", e))?;

                let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)
                    .expect("Failed to create mDNS behaviour");

                Ok(ChatBehaviour { gossipsub, mdns })
            })?
            .build();

        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        let local_peer_id = *swarm.local_peer_id();

        Ok(ChatClient {
            swarm,
            peer_id: local_peer_id,
            topic: IdentTopic::new(topic_name),
            event_sender,
            event_receiver: Some(event_receiver),
            message_receiver: Some(message_receiver),
            message_sender,
        })
    }
    pub fn event_sender(&self) -> UnboundedSender<ChatEvent> {
        self.event_sender.clone()
    }
    pub fn event_receiver(&mut self) -> UnboundedReceiver<ChatEvent> {
        self.event_receiver.take().unwrap()
    }
    pub fn message_receiver(&mut self) -> UnboundedReceiver<String> {
        self.message_receiver.take().unwrap()
    }
    pub fn message_sender(&self) -> UnboundedSender<String> {
        self.message_sender.clone()
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }
    pub fn listen_on(&mut self, addr: libp2p::Multiaddr) -> Result<(), Box<dyn std::error::Error>> {
        self.swarm.listen_on(addr)?;
        Ok(())
    }
    fn send_message(&mut self, content: String) -> Result<(), Box<dyn std::error::Error>> {
        let chat_message = ChatMessage::new(
            self.peer_id.to_string(),
            content,
            chrono::Utc::now().timestamp_millis() as u64,
        );

        let message_data = serde_json::to_vec(&chat_message)?;

        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.topic.clone(), message_data)?;
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut message_receiver = self.message_receiver();
        loop {
            tokio::select! {
                Some(content) = message_receiver.recv() => {
                    if let Err(e) = self.send_message(content) {
                        eprintln!("Error sending message: {}", e);
                    }
                }
                event = self.swarm.select_next_some() => {
                    match event {
                        libp2p::swarm::SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. })) => {
                            if let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(&message.data) {
                                self.event_sender.send(ChatEvent::MessageReceived(chat_message))?;
                            }
                        }
                        libp2p::swarm::SwarmEvent::Behaviour(ChatBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                            for (peer_id, _) in list {
                                self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                                self.event_sender.send(ChatEvent::PeerDiscovered(peer_id))?;
                            }
                        }
                        libp2p::swarm::SwarmEvent::Behaviour(ChatBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                            for (peer_id, _) in list {
                                self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                                self.event_sender.send(ChatEvent::PeerExpired(peer_id))?;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::time::sleep;

    struct TestClients {
        client1: (ChatClient, Arc<Mutex<Vec<ChatEvent>>>),
        client2: (ChatClient, Arc<Mutex<Vec<ChatEvent>>>),
    }

    async fn client_with_events() -> (ChatClient, Arc<Mutex<Vec<ChatEvent>>>) {
        let mut client = get_test_client().await;

        let events = Arc::new(Mutex::new(Vec::<ChatEvent>::new()));
        let events_clone = events.clone();

        let mut client_event_receiver = client.event_receiver();

        tokio::spawn(async move {
            while let Some(event) = client_event_receiver.recv().await {
                events_clone.lock().unwrap().push(event)
            }
        });

        (client, events)
    }

    async fn connected_clients() -> TestClients {
        let (client1, events1) = client_with_events().await;
        let (client2, events2) = client_with_events().await;

        TestClients {
            client1: (client1, events1),
            client2: (client2, events2),
        }
    }

    #[tokio::test]
    async fn test_chat_message_serialization() {
        let original_message = ChatMessage::new(
            "peer1".to_string(),
            "Hello, world!".to_string(),
            1625247600000,
        );

        let serialized = serde_json::to_string(&original_message).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original_message.sender, deserialized.sender);
        assert_eq!(original_message.content, deserialized.content);
        assert_eq!(original_message.timestamp, deserialized.timestamp);
    }

    #[tokio::test]
    async fn test_chat_client_creation() {
        let client = get_test_client().await;
        assert_eq!(client.topic.hash().as_str(), "test_topic");
        assert_eq!(client.peer_id(), *client.swarm.local_peer_id());
    }

    #[tokio::test]
    async fn test_peer_discovery() {
        let connected_clients = connected_clients().await;
        let (mut client1, events1) = connected_clients.client1;
        let (mut client2, events2) = connected_clients.client2;

        let handle1 = tokio::spawn(async move {
            client1.run().await.expect("Failed to run client1");
        });
        let handle2 = tokio::spawn(async move {
            client2.run().await.expect("Failed to run client2");
        });

        sleep(Duration::from_millis(300)).await;

        let events1_final = events1.lock().unwrap();
        let events2_final = events2.lock().unwrap();

        let peer_discovered1 = events1_final
            .iter()
            .any(|event| matches!(event, ChatEvent::PeerDiscovered(_)));
        let peer_discovered2 = events2_final
            .iter()
            .any(|event| matches!(event, ChatEvent::PeerDiscovered(_)));

        assert!(peer_discovered1);
        assert!(peer_discovered2);

        drop(handle1);
        drop(handle2);
    }

    #[tokio::test]
    async fn test_message_sending() {
        let connected_clients = connected_clients().await;
        let (mut client1, _events1) = connected_clients.client1;
        let (mut client2, events2) = connected_clients.client2;
        let message_sender1 = client1.message_sender();

        let handle1 = tokio::spawn(async move {
            client1.run().await.expect("Failed to run client1");
        });

        let handle2 = tokio::spawn(async move {
            client2.run().await.expect("Failed to run client2");
        });

        sleep(Duration::from_millis(300)).await;

        message_sender1.send("Hello from client1".to_string()).unwrap();

        sleep(Duration::from_millis(300)).await;

        let events2_final = events2.lock().unwrap();

        let message_received = events2_final.iter().any(|event| {
            if let ChatEvent::MessageReceived(msg) = event {
                msg.content == "Hello from client1"
            } else {
                false
            }
        });

        assert!(message_received);

        drop(handle1);
        drop(handle2);
    }

    async fn get_test_client() -> ChatClient {
        ChatClient::new("test_topic").await.unwrap()
    }
}
