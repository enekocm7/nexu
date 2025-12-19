use std::path::PathBuf;
use std::{fs, io};
use ui::desktop::models::Topic;

const TOPICS_DIR_NAME: &str = "nexu";
const TOPICS_FILE_PATH: &str = "topics_data.bin";

pub fn save_topics_to_file(topics: &Vec<Topic>) -> io::Result<()> {
    let path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(TOPICS_DIR_NAME)
        .join(TOPICS_FILE_PATH);
    fs::create_dir_all(path.parent().unwrap())?;
    save_topics_to_file_with_path(topics, &path)
}

pub fn save_topics_to_file_with_path(topics: &Vec<Topic>, path: &PathBuf) -> io::Result<()> {
    fs::create_dir_all(path.parent().unwrap())?;
    let encoded_topics =
        postcard::to_stdvec(topics).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, encoded_topics)
}

pub fn load_topics_from_file() -> io::Result<Vec<Topic>> {
    let path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(TOPICS_DIR_NAME)
        .join(TOPICS_FILE_PATH);
    println!("Loading topics from file, path: {:?}", path);
    load_topics_from_file_with_path(&path)
}

pub fn load_topics_from_file_with_path(path: &PathBuf) -> io::Result<Vec<Topic>> {
    let data = fs::read(path)?;
    let topics: Vec<Topic> = postcard::from_bytes(&data).unwrap_or_default();
    Ok(topics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use ui::desktop::models::{ChatMessage, Topic};

    fn create_test_topic(id: &str, name: &str) -> Topic {
        Topic::new(id.to_string(), name.to_string(), None)
    }

    fn create_test_topic_with_message(id: &str, name: &str) -> Topic {
        let mut topic = create_test_topic(id, name);
        let message = ChatMessage::new(
            "sender123".to_string(),
            id.to_string(),
            "Hello, World!".to_string(),
            1234567890,
            true,
        );
        topic.add_message(message);
        topic
    }

    #[test]
    fn test_save_and_load_topics_with_path() {
        let temp_dir = TempDir::new().unwrap();
        let test_file_path = temp_dir.path().join("test_topics.json");

        let topics = vec![
            create_test_topic("topic1", "Topic One"),
            create_test_topic("topic2", "Topic Two"),
        ];

        let save_result = save_topics_to_file_with_path(&topics, &test_file_path);
        assert!(save_result.is_ok(), "Failed to save topics");

        let load_result = load_topics_from_file_with_path(&test_file_path);
        assert!(load_result.is_ok(), "Failed to load topics");

        let loaded_topics = load_result.unwrap();
        assert_eq!(loaded_topics.len(), 2, "Loaded topics count mismatch");

        let topic1 = loaded_topics.iter().find(|t| t.id == "topic1").unwrap();
        assert_eq!(topic1.id, "topic1");
        assert_eq!(topic1.name, "Topic One");

        let topic2 = loaded_topics.iter().find(|t| t.id == "topic2").unwrap();
        assert_eq!(topic2.id, "topic2");
        assert_eq!(topic2.name, "Topic Two");
    }

    #[test]
    fn test_save_and_load_topics_with_messages() {
        let temp_dir = TempDir::new().unwrap();
        let test_file_path = temp_dir.path().join("test_topics_messages.json");

        let topics = vec![create_test_topic_with_message("topic1", "Topic One")];

        save_topics_to_file_with_path(&topics, &test_file_path).unwrap();

        let loaded_topics = load_topics_from_file_with_path(&test_file_path).unwrap();

        let topic1 = loaded_topics.iter().find(|t| t.id == "topic1").unwrap();
        assert_eq!(topic1.messages.len(), 1, "Message count mismatch");
        assert_eq!(topic1.last_message, Some("Hello, World!".to_string()));

        if let ui::desktop::models::Message::Chat(chat_msg) = &topic1.messages[0] {
            assert_eq!(chat_msg.content, "Hello, World!");
            assert_eq!(chat_msg.sender_id, "sender123");
        } else {
            panic!("Expected ChatMessage variant");
        }
    }

    #[test]
    fn test_round_trip_preserves_all_topic_fields() {
        let temp_dir = TempDir::new().unwrap();
        let test_file_path = temp_dir.path().join("round_trip_test.json");

        let mut topic = create_test_topic("topic1", "Topic One");
        topic.avatar_url = Some("https://example.com/avatar.png".to_string());
        topic.last_connection = Some(9876543210);

        let message = ChatMessage::new(
            "sender1".to_string(),
            "topic1".to_string(),
            "Test message".to_string(),
            1234567890,
            true,
        );
        topic.add_message(message);

        let topics = vec![topic];

        save_topics_to_file_with_path(&topics, &test_file_path).unwrap();
        let loaded_topics = load_topics_from_file_with_path(&test_file_path).unwrap();

        let loaded_topic = loaded_topics.iter().find(|t| t.id == "topic1").unwrap();
        assert_eq!(loaded_topic.id, "topic1");
        assert_eq!(loaded_topic.name, "Topic One");
        assert_eq!(
            loaded_topic.avatar_url,
            Some("https://example.com/avatar.png".to_string())
        );
        assert_eq!(loaded_topic.last_connection, Some(9876543210));
        assert_eq!(loaded_topic.last_message, Some("Test message".to_string()));
        assert_eq!(loaded_topic.messages.len(), 1);
    }

    #[test]
    fn test_save_empty_topics() {
        let temp_dir = TempDir::new().unwrap();
        let test_file_path = temp_dir.path().join("empty_topics.json");

        let topics: Vec<Topic> = Vec::new();

        let save_result = save_topics_to_file_with_path(&topics, &test_file_path);
        assert!(save_result.is_ok(), "Failed to save empty topics");

        let loaded_topics = load_topics_from_file_with_path(&test_file_path).unwrap();
        assert_eq!(loaded_topics.len(), 0, "Expected empty topics");
    }

    #[test]
    fn test_load_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.json");

        let result = load_topics_from_file_with_path(&nonexistent_path);
        assert!(
            result.is_err(),
            "Expected error when loading nonexistent file"
        );
    }

    #[test]
    fn test_save_to_invalid_path() {
        let invalid_path = PathBuf::from("/nonexistent/directory/test.json");
        let topics = vec![create_test_topic("topic1", "Topic One")];

        let result = save_topics_to_file_with_path(&topics, &invalid_path);
        assert!(
            result.is_err(),
            "Expected error when saving to invalid path"
        );
    }

    #[test]
    fn test_load_corrupted_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file_path = temp_dir.path().join("corrupted.json");

        fs::write(&test_file_path, b"corrupted data").unwrap();

        let loaded_topics = load_topics_from_file_with_path(&test_file_path).unwrap();
        assert_eq!(
            loaded_topics.len(),
            0,
            "Expected empty Vec for corrupted data"
        );
    }

    #[test]
    fn test_save_topics_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();
        let test_file_path = temp_dir.path().join("overwrite_test.json");

        let topics1 = vec![create_test_topic("topic1", "Topic One")];
        save_topics_to_file_with_path(&topics1, &test_file_path).unwrap();

        let topics2 = vec![
            create_test_topic("topic2", "Topic Two"),
            create_test_topic("topic3", "Topic Three"),
        ];
        save_topics_to_file_with_path(&topics2, &test_file_path).unwrap();

        let loaded_topics = load_topics_from_file_with_path(&test_file_path).unwrap();
        assert_eq!(loaded_topics.len(), 2, "Expected 2 topics after overwrite");
        assert!(
            !loaded_topics.iter().any(|t| t.id == "topic1"),
            "topic1 should be gone"
        );
        assert!(
            loaded_topics.iter().any(|t| t.id == "topic2"),
            "topic2 should exist"
        );
        assert!(
            loaded_topics.iter().any(|t| t.id == "topic3"),
            "topic3 should exist"
        );
    }
}
