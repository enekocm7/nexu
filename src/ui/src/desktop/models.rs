use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Topic {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub last_connection: Option<u64>,
    pub last_message: Option<String>,
}

impl PartialEq for Topic {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum TopicCreationMode {
    Create,
    Join
}

#[cfg(feature = "desktop-web")]
#[derive(Debug, Clone)]
pub struct AppState {
    topics: HashMap<String, Topic>,
    current_topic_id: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            topics: HashMap::new(),
            current_topic_id: None,
        }
    }
    
    pub fn add_topic(&mut self, topic: Topic) {
        self.topics.insert(topic.id.clone(), topic);
    }
    
    pub fn set_current_topic(&mut self, topic_id: String) {
        self.current_topic_id = Some(topic_id);
    }
    
    pub fn get_current_topic(&self) -> Option<&Topic> {
        match &self.current_topic_id {
            Some(id) => self.topics.get(id),
            None => None,
        }
    }
    
    pub fn get_all_topics(&self) -> Vec<&Topic> {
        self.topics.values().collect()
    }
}