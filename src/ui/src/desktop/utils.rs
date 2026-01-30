use crate::desktop::models::AppState;
use arboard::Clipboard;
use chrono::{DateTime, Local, TimeDelta};
use dioxus_primitives::toast::ToastOptions;
use dioxus_primitives::toast::Toasts;

pub fn truncate_id(id: &str) -> String {
    if id.len() > 12 {
        let start = &id[..6];
        let end = &id[id.len() - 6..];
        format!("{}...{}", start, end)
    } else {
        id.to_string()
    }
}

pub fn get_sender_display_name(app_state: &AppState, sender_id: &str) -> String {
    let profile = app_state.get_profile();
    if sender_id == profile.id {
        return profile.name;
    }
    if let Some(contact) = app_state.get_contact(sender_id) {
        if contact.name == contact.id {
            truncate_id(sender_id)
        } else {
            contact.name.clone()
        }
    } else {
        truncate_id(sender_id)
    }
}

pub fn format_message_timestamp(timestamp: u64) -> String {
    let timestamp_secs = (timestamp / 1000) as i64;
    let datetime = match DateTime::from_timestamp(timestamp_secs, 0) {
        Some(dt) => dt.with_timezone(&Local),
        None => return String::from(""),
    };

    let now = Local::now();
    let duration = now.signed_duration_since(datetime);

    if duration < TimeDelta::days(1) {
        return datetime.format("%I:%M %p").to_string();
    }

    if duration < TimeDelta::days(2) {
        return format!("Yesterday {}", datetime.format("%I:%M %p"));
    }

    if duration < TimeDelta::weeks(1) {
        return datetime.format("%a %I:%M %p").to_string();
    }

    datetime.format("%m/%d/%y %I:%M %p").to_string()
}

pub fn format_relative_time(timestamp: i64) -> String {
    let last_connection = match DateTime::from_timestamp(timestamp, 0) {
        Some(dt) => dt.with_timezone(&Local),
        None => return String::from(""),
    };

    let now = Local::now();
    let duration = now.signed_duration_since(last_connection);

    if duration < TimeDelta::minutes(1) {
        return String::from("Just now");
    }

    if duration < TimeDelta::hours(1) {
        let minutes = duration.num_minutes();
        return format!("{}m ago", minutes);
    }

    if duration < TimeDelta::days(1) {
        let hours = duration.num_hours();
        return format!("{}h ago", hours);
    }

    if duration < TimeDelta::days(2) {
        return String::from("Yesterday");
    }

    if duration < TimeDelta::weeks(1) {
        let days = duration.num_days();
        return format!("{} days ago", days);
    }

    last_connection.format("%m/%d/%Y").to_string()
}

pub fn copy_to_clipboard(mut clipboard: Clipboard, text: &str, toast: Toasts) {
    match clipboard.set_text(text) {
        Ok(_) => {
            toast.success(
                "Topic ID copied to clipboard!".to_owned(),
                ToastOptions::default(),
            );
        }
        Err(_) => {
            toast.error(
                "Error copying Topic ID.".to_owned(),
                ToastOptions::default(),
            );
        }
    }
}

pub fn is_video_file(name: &str) -> bool {
    let video_extensions = ["mp4", "webm", "mov", "avi", "mkv", "m4v", "ogv"];
    if let Some(ext) = name.split('.').next_back() {
        video_extensions.contains(&ext.to_lowercase().as_str())
    } else {
        false
    }
}

// pub fn process_image(file_bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
//     let image = image::load_from_memory(file_bytes)?;
//     let resized = image.thumbnail(500, 500);
//     let mut buffer = std::io::Cursor::new(Vec::new());
//     resized.write_to(&mut buffer, image::ImageFormat::WebP)?;
//     Ok(buffer.into_inner())
// }
