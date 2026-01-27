const DIR_NAME: &str = "nexu";

pub mod topics {
    use std::path::PathBuf;
    use std::{fs, io};
    use ui::desktop::models::Topic;

    use crate::utils::DIR_NAME;

    const TOPICS_FILE_PATH: &str = "topics_data.bin";

    pub fn save_topics_to_file(topics: &Vec<Topic>) -> io::Result<()> {
        let path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(DIR_NAME)
            .join(TOPICS_FILE_PATH);
        fs::create_dir_all(path.parent().unwrap())?;
        save_topics_to_file_with_path(topics, &path)
    }

    pub fn save_topics_to_file_with_path(topics: &Vec<Topic>, path: &PathBuf) -> io::Result<()> {
        fs::create_dir_all(path.parent().unwrap())?;
        let encoded_topics = postcard::to_stdvec(topics)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, encoded_topics)
    }

    pub fn load_topics_from_file() -> io::Result<Vec<Topic>> {
        let path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(DIR_NAME)
            .join(TOPICS_FILE_PATH);
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
}
pub mod contacts {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use tokio::io;
    use ui::desktop::models::{Profile, ProfileChat};

    use crate::utils::DIR_NAME;

    const CONTACTS_NAME_FILE: &str = "contacts.bin";
    const MY_PROFILE_NAME_FILE: &str = "profile.bin";

    pub fn save_profile(profile: &Profile) -> io::Result<()> {
        let path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(DIR_NAME)
            .join(MY_PROFILE_NAME_FILE);
        fs::create_dir_all(path.parent().unwrap())?;
        save_profile_to_path(profile, &path)
    }

    pub fn save_profile_to_path(profile: &Profile, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path.parent().unwrap())?;
        let encoded_profile = postcard::to_stdvec(profile)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, encoded_profile)
    }

    pub fn load_profile() -> io::Result<Profile> {
        let path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(DIR_NAME)
            .join(MY_PROFILE_NAME_FILE);
        load_profile_from_path(&path)
    }

    pub fn load_profile_from_path(path: &Path) -> io::Result<Profile> {
        let data = fs::read(path)?;
        let profile: Profile = postcard::from_bytes(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(profile)
    }

    pub fn save_contacts(contacts: &[ProfileChat]) -> io::Result<()> {
        let path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(DIR_NAME)
            .join(CONTACTS_NAME_FILE);
        fs::create_dir_all(path.parent().unwrap())?;
        save_contacts_to_path(contacts, &path)
    }

    pub fn save_contacts_to_path(contacts: &[ProfileChat], path: &Path) -> io::Result<()> {
        fs::create_dir_all(path.parent().unwrap())?;
        let encoded_contacts = postcard::to_stdvec(contacts)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, encoded_contacts)
    }

    pub fn load_contacts() -> io::Result<Vec<ProfileChat>> {
        let path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(DIR_NAME)
            .join(CONTACTS_NAME_FILE);
        load_contacts_from_path(&path)
    }

    pub fn load_contacts_from_path(path: &Path) -> io::Result<Vec<ProfileChat>> {
        let data = fs::read(path)?;
        let contacts: Vec<ProfileChat> = postcard::from_bytes(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(contacts)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use tempfile::TempDir;
        use ui::desktop::models::ConnectionStatus::Offline;
        use ui::desktop::models::{DmMessage, Profile, ProfileChat};

        fn create_test_profile(id: &str, name: &str, avatar: Option<&str>) -> Profile {
            Profile {
                id: id.to_string(),
                name: name.to_string(),
                avatar: avatar.map(|s| s.to_string()),
                last_connection: Offline(1234567890),
            }
        }

        fn create_test_profile_chat(
            id: &str,
            name: &str,
            avatar: Option<&str>,
            messages: Vec<DmMessage>,
        ) -> ProfileChat {
            ProfileChat {
                profile: Profile {
                    id: id.to_string(),
                    name: name.to_string(),
                    avatar: avatar.map(|s| s.to_string()),
                    last_connection: Offline(1234567890),
                },
                messages,
                last_changed: 1234567890,
            }
        }

        #[test]
        fn test_save_and_load_profile_with_path() {
            let temp_dir = TempDir::new().unwrap();
            let test_file_path = temp_dir.path().join("test_profile.bin");

            let profile = create_test_profile(
                "user123",
                "John Doe",
                Some("https://example.com/avatar.png"),
            );

            let save_result = save_profile_to_path(&profile, &test_file_path);
            assert!(save_result.is_ok(), "Failed to save profile");

            let load_result = load_profile_from_path(&test_file_path);
            assert!(load_result.is_ok(), "Failed to load profile");

            let loaded_profile = load_result.unwrap();
            assert_eq!(loaded_profile.id, "user123");
            assert_eq!(loaded_profile.name, "John Doe");
            assert_eq!(
                loaded_profile.avatar,
                Some("https://example.com/avatar.png".to_string())
            );
            assert_eq!(loaded_profile.last_connection, Offline(1234567890));
        }

        #[test]
        fn test_save_and_load_profile_without_avatar() {
            let temp_dir = TempDir::new().unwrap();
            let test_file_path = temp_dir.path().join("test_profile_no_avatar.bin");

            let profile = create_test_profile("user456", "Jane Smith", None);

            save_profile_to_path(&profile, &test_file_path).unwrap();
            let loaded_profile = load_profile_from_path(&test_file_path).unwrap();

            assert_eq!(loaded_profile.id, "user456");
            assert_eq!(loaded_profile.name, "Jane Smith");
            assert_eq!(loaded_profile.avatar, None);
        }

        #[test]
        fn test_save_and_load_contacts_with_path() {
            let temp_dir = TempDir::new().unwrap();
            let test_file_path = temp_dir.path().join("test_contacts.bin");

            let contacts = vec![
                create_test_profile_chat(
                    "contact1",
                    "Alice",
                    Some("https://example.com/alice.png"),
                    Vec::new(),
                ),
                create_test_profile_chat(
                    "contact2",
                    "Bob",
                    Some("https://example.com/bob.png"),
                    Vec::new(),
                ),
                create_test_profile_chat("contact3", "Charlie", None, Vec::new()),
            ];

            let save_result = save_contacts_to_path(&contacts, &test_file_path);
            assert!(save_result.is_ok(), "Failed to save contacts");

            let load_result = load_contacts_from_path(&test_file_path);
            assert!(load_result.is_ok(), "Failed to load contacts");

            let loaded_contacts = load_result.unwrap();
            assert_eq!(loaded_contacts.len(), 3, "Loaded contacts count mismatch");

            let contact1 = loaded_contacts
                .iter()
                .find(|c| c.profile.id == "contact1")
                .unwrap();
            assert_eq!(contact1.profile.name, "Alice");
            assert_eq!(
                contact1.profile.avatar,
                Some("https://example.com/alice.png".to_string())
            );

            let contact2 = loaded_contacts
                .iter()
                .find(|c| c.profile.id == "contact2")
                .unwrap();
            assert_eq!(contact2.profile.name, "Bob");

            let contact3 = loaded_contacts
                .iter()
                .find(|c| c.profile.id == "contact3")
                .unwrap();
            assert_eq!(contact3.profile.name, "Charlie");
            assert_eq!(contact3.profile.avatar, None);
        }

        #[test]
        fn test_save_empty_contacts() {
            let temp_dir = TempDir::new().unwrap();
            let test_file_path = temp_dir.path().join("empty_contacts.bin");

            let contacts: Vec<ProfileChat> = Vec::new();

            let save_result = save_contacts_to_path(&contacts, &test_file_path);
            assert!(save_result.is_ok(), "Failed to save empty contacts");

            let loaded_contacts = load_contacts_from_path(&test_file_path).unwrap();
            assert_eq!(loaded_contacts.len(), 0, "Expected empty contacts");
        }

        #[test]
        fn test_load_nonexistent_profile() {
            let temp_dir = TempDir::new().unwrap();
            let nonexistent_path = temp_dir.path().join("nonexistent_profile.bin");

            let result = load_profile_from_path(&nonexistent_path);
            assert!(
                result.is_err(),
                "Expected error when loading nonexistent profile"
            );
        }

        #[test]
        fn test_load_nonexistent_contacts() {
            let temp_dir = TempDir::new().unwrap();
            let nonexistent_path = temp_dir.path().join("nonexistent_contacts.bin");

            let result = load_contacts_from_path(&nonexistent_path);
            assert!(
                result.is_err(),
                "Expected error when loading nonexistent contacts"
            );
        }

        #[test]
        fn test_save_profile_to_invalid_path() {
            let invalid_path = PathBuf::from("/nonexistent/directory/test_profile.bin");
            let profile = create_test_profile("user123", "John Doe", None);

            let result = save_profile_to_path(&profile, &invalid_path);
            assert!(
                result.is_err(),
                "Expected error when saving to invalid path"
            );
        }

        #[test]
        fn test_save_contacts_to_invalid_path() {
            let invalid_path = PathBuf::from("/nonexistent/directory/test_contacts.bin");
            let contacts = vec![create_test_profile_chat(
                "contact1",
                "Alice",
                None,
                Vec::new(),
            )];

            let result = save_contacts_to_path(&contacts, &invalid_path);
            assert!(
                result.is_err(),
                "Expected error when saving to invalid path"
            );
        }

        #[test]
        fn test_load_corrupted_profile() {
            let temp_dir = TempDir::new().unwrap();
            let test_file_path = temp_dir.path().join("corrupted_profile.bin");

            fs::write(&test_file_path, b"corrupted data").unwrap();

            let result = load_profile_from_path(&test_file_path);
            assert!(
                result.is_err(),
                "Expected error when loading corrupted profile"
            );
        }

        #[test]
        fn test_load_corrupted_contacts() {
            let temp_dir = TempDir::new().unwrap();
            let test_file_path = temp_dir.path().join("corrupted_contacts.bin");

            fs::write(&test_file_path, b"corrupted data").unwrap();

            let result = load_contacts_from_path(&test_file_path);
            assert!(
                result.is_err(),
                "Expected error when loading corrupted contacts"
            );
        }

        #[test]
        fn test_save_profile_overwrites_existing() {
            let temp_dir = TempDir::new().unwrap();
            let test_file_path = temp_dir.path().join("overwrite_profile.bin");

            let profile1 = create_test_profile("user123", "John Doe", None);
            save_profile_to_path(&profile1, &test_file_path).unwrap();

            let profile2 = create_test_profile(
                "user456",
                "Jane Smith",
                Some("https://example.com/avatar.png"),
            );
            save_profile_to_path(&profile2, &test_file_path).unwrap();

            let loaded_profile = load_profile_from_path(&test_file_path).unwrap();
            assert_eq!(
                loaded_profile.id, "user456",
                "Profile should be overwritten"
            );
            assert_eq!(loaded_profile.name, "Jane Smith");
        }

        #[test]
        fn test_save_contacts_overwrites_existing() {
            let temp_dir = TempDir::new().unwrap();
            let test_file_path = temp_dir.path().join("overwrite_contacts.bin");

            let contacts1 = vec![create_test_profile_chat(
                "contact1",
                "Alice",
                None,
                Vec::new(),
            )];
            save_contacts_to_path(&contacts1, &test_file_path).unwrap();

            let contacts2 = vec![
                create_test_profile_chat("contact2", "Bob", None, Vec::new()),
                create_test_profile_chat("contact3", "Charlie", None, Vec::new()),
            ];
            save_contacts_to_path(&contacts2, &test_file_path).unwrap();

            let loaded_contacts = load_contacts_from_path(&test_file_path).unwrap();
            assert_eq!(
                loaded_contacts.len(),
                2,
                "Expected 2 contacts after overwrite"
            );
            assert!(
                !loaded_contacts.iter().any(|c| c.profile.id == "contact1"),
                "contact1 should be gone"
            );
            assert!(
                loaded_contacts.iter().any(|c| c.profile.id == "contact2"),
                "contact2 should exist"
            );
            assert!(
                loaded_contacts.iter().any(|c| c.profile.id == "contact3"),
                "contact3 should exist"
            );
        }

        #[test]
        fn test_round_trip_preserves_all_profile_fields() {
            let temp_dir = TempDir::new().unwrap();
            let test_file_path = temp_dir.path().join("round_trip_profile.bin");

            let profile = Profile {
                id: "user789".to_string(),
                name: "Test User".to_string(),
                avatar: Some("https://example.com/test.png".to_string()),
                last_connection: Offline(1234567890),
            };

            save_profile_to_path(&profile, &test_file_path).unwrap();
            let loaded_profile = load_profile_from_path(&test_file_path).unwrap();

            assert_eq!(loaded_profile.id, "user789");
            assert_eq!(loaded_profile.name, "Test User");
            assert_eq!(
                loaded_profile.avatar,
                Some("https://example.com/test.png".to_string())
            );
            assert_eq!(loaded_profile.last_connection, Offline(1234567890));
        }

        #[test]
        fn test_contacts_preserve_order() {
            let temp_dir = TempDir::new().unwrap();
            let test_file_path = temp_dir.path().join("ordered_contacts.bin");

            let contacts = vec![
                create_test_profile_chat("contact1", "Alice", None, Vec::new()),
                create_test_profile_chat("contact2", "Bob", None, Vec::new()),
                create_test_profile_chat("contact3", "Charlie", None, Vec::new()),
            ];

            save_contacts_to_path(&contacts, &test_file_path).unwrap();
            let loaded_contacts = load_contacts_from_path(&test_file_path).unwrap();

            assert_eq!(loaded_contacts[0].profile.id, "contact1");
            assert_eq!(loaded_contacts[1].profile.id, "contact2");
            assert_eq!(loaded_contacts[2].profile.id, "contact3");
        }
    }
}
pub mod video {
    use dioxus::desktop::AssetRequest;
    use dioxus::desktop::wry::http::Response;
    use dioxus::desktop::wry::http::header::{CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE};
    use http::{response::Builder as ResponseBuilder, status::StatusCode};
    use std::io::SeekFrom;
    use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

    pub async fn get_stream_response(
        asset: &mut (impl tokio::io::AsyncSeek + tokio::io::AsyncRead + Unpin + Send + Sync),
        request: &AssetRequest,
    ) -> anyhow::Result<Response<Vec<u8>>> {
        let len = {
            let old_pos = asset.stream_position().await?;
            let len = asset.seek(SeekFrom::End(0)).await?;
            asset.seek(SeekFrom::Start(old_pos)).await?;
            len
        };

        let mut resp = ResponseBuilder::new().header(CONTENT_TYPE, "video/mp4");

        // if the webview sent a range header, we need to send a 206 in return
        // Actually only macOS and Windows are supported. Linux will ALWAYS return empty headers.
        let http_response = if let Some(range_header) = request.headers().get("range") {
            let not_satisfiable = || {
                ResponseBuilder::new()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(CONTENT_RANGE, format!("bytes */{len}"))
                    .body(vec![])
            };

            // parse range header
            let ranges =
                if let Ok(ranges) = http_range::HttpRange::parse(range_header.to_str()?, len) {
                    ranges
                        .iter()
                        // map the output back to spec range <start-end>, example: 0-499
                        .map(|r| (r.start, r.start + r.length - 1))
                        .collect::<Vec<_>>()
                } else {
                    return Ok(not_satisfiable()?);
                };

            /// The Maximum bytes we send in one range
            const MAX_LEN: u64 = 1000 * 1024;

            if ranges.len() == 1 {
                let &(start, mut end) = ranges.first().unwrap();

                // check if a range is not satisfiable
                //
                // this should be already taken care of by HttpRange::parse
                // but checking here again for extra assurance
                if start >= len || end >= len || end < start {
                    return Ok(not_satisfiable()?);
                }

                // adjust end byte for MAX_LEN
                end = start + (end - start).min(len - start).min(MAX_LEN - 1);

                // calculate number of bytes needed to be read
                let bytes_to_read = end + 1 - start;

                // allocate a buf with a suitable capacity
                let mut buf = Vec::with_capacity(bytes_to_read as usize);
                // seek the file to the starting byte
                asset.seek(SeekFrom::Start(start)).await?;
                // read the needed bytes
                asset.take(bytes_to_read).read_to_end(&mut buf).await?;

                resp = resp.header(CONTENT_RANGE, format!("bytes {start}-{end}/{len}"));
                resp = resp.header(CONTENT_LENGTH, end + 1 - start);
                resp = resp.status(StatusCode::PARTIAL_CONTENT);
                resp.body(buf)
            } else {
                let mut buf = Vec::new();
                let ranges = ranges
                    .iter()
                    .filter_map(|&(start, mut end)| {
                        // filter out unsatisfiable ranges
                        //
                        // this should be already taken care of by HttpRange::parse
                        // but checking here again for extra assurance
                        if start >= len || end >= len || end < start {
                            None
                        } else {
                            // adjust end byte for MAX_LEN
                            end = start + (end - start).min(len - start).min(MAX_LEN - 1);
                            Some((start, end))
                        }
                    })
                    .collect::<Vec<_>>();

                let boundary = format!("{:x}", rand::random::<u64>());
                let boundary_sep = format!("\r\n--{boundary}\r\n");
                let boundary_closer = format!("\r\n--{boundary}\r\n");

                resp = resp.header(
                    CONTENT_TYPE,
                    format!("multipart/byteranges; boundary={boundary}"),
                );

                for (end, start) in ranges {
                    // a new range is being written, write the range boundary
                    buf.write_all(boundary_sep.as_bytes()).await?;

                    // write the needed headers `Content-Type` and `Content-Range`
                    buf.write_all(format!("{CONTENT_TYPE}: video/mp4\r\n").as_bytes())
                        .await?;
                    buf.write_all(
                        format!("{CONTENT_RANGE}: bytes {start}-{end}/{len}\r\n").as_bytes(),
                    )
                    .await?;

                    // write the separator to indicate the start of the range body
                    buf.write_all("\r\n".as_bytes()).await?;

                    // calculate number of bytes needed to be read
                    let bytes_to_read = end + 1 - start;

                    let mut local_buf = vec![0_u8; bytes_to_read as usize];
                    asset.seek(SeekFrom::Start(start)).await?;
                    asset.read_exact(&mut local_buf).await?;
                    buf.extend_from_slice(&local_buf);
                }
                // all ranges have been written, write the closing boundary
                buf.write_all(boundary_closer.as_bytes()).await?;

                resp.body(buf)
            }
        } else {
            resp = resp.header(CONTENT_LENGTH, len);
            let mut buf = Vec::with_capacity(len as usize);
            asset.read_to_end(&mut buf).await?;
            resp.body(buf)
        };

        http_response.map_err(Into::into)
    }
}
