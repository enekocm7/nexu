use super::desktop_web_components::DEFAULT_AVATAR;
use super::models::{AppState, Profile, ProfileChat, RemovalType, Topic};
use super::utils::format_relative_time;
use dioxus::prelude::*;
use dioxus_primitives::context_menu::{
    ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger,
};

#[component]
pub fn TopicColumn(
    search_query: Signal<String>,
    selected_topic_id: Signal<Option<String>>,
    show_topic_details: Signal<Option<Topic>>,
    show_leave_confirmation: Signal<Option<(String, String, RemovalType)>>,
    app_state: Signal<AppState>,
) -> Element {
    let topic_list: Vec<Topic> = {
        let state = app_state();
        let mut topics = state.get_all_topics().into_iter().collect::<Vec<Topic>>();
        topics.sort_by(|a, b| b.last_connection.cmp(&a.last_connection));
        topics
    };

    rsx! {
        ul {
            {
                topic_list
                    .into_iter()
                    .filter(|topic| {
                        topic.name.to_lowercase().contains(&search_query().to_lowercase())
                            || topic.id.to_lowercase().contains(&search_query().to_lowercase())
                    })
                    .map(|topic| {
                        let topic_id = topic.id;
                        let topic_name = topic.name;
                        let topic_id_open = topic_id.clone();
                        let topic_id_details = topic_id.clone();
                        let topic_id_leave = topic_id.clone();
                        let topic_name_leave = topic_name.clone();
                        rsx! {
                            ContextMenu {
                                ContextMenuTrigger {
                                    ColumnItem {
                                        id: topic_id,
                                        name: topic_name,
                                        avatar_url: topic.avatar_url,
                                        last_message: topic.last_message,
                                        last_connection: topic.last_connection,
                                        on_select: selected_topic_id,
                                        highlight: search_query(),
                                    }
                                }
                                ContextMenuContent { class: "context-menu",
                                    ContextMenuItem {
                                        class: "context-menu-item",
                                        value: "Open Chat".to_string(),
                                        index: 0usize,
                                        on_select: move |_| {
                                            selected_topic_id.set(Some(topic_id_open.clone()));
                                        },
                                        "Open Chat"
                                    }
                                    ContextMenuItem {
                                        class: "context-menu-item",
                                        value: "Open Details".to_string(),
                                        index: 1usize,
                                        on_select: move |_| {
                                            if let Some(topic) = app_state().get_topic(&topic_id_details) {
                                                show_topic_details.set(Some(topic.clone()));
                                            }
                                        },
                                        "Open Details"
                                    }
                                    ContextMenuItem {
                                        class: "context-menu-item-danger",
                                        value: "Leave Topic".to_string(),
                                        index: 2usize,
                                        on_select: move |_| {
                                            show_leave_confirmation.set(Some((
                                                topic_id_leave.clone(),
                                                topic_name_leave.clone(),
                                                RemovalType::Topic,
                                            )));
                                        },
                                        "Leave Topic"
                                    }
                                }
                            }
                        }
                    })
            }
        }
    }
}

#[component]
pub fn ContactColumn(
    search_query: Signal<String>,
    selected_topic_id: Signal<Option<String>>,
    show_profile_details: Signal<Option<Profile>>,
    show_leave_confirmation: Signal<Option<(String, String, RemovalType)>>,
    app_state: Signal<AppState>,
) -> Element {
    let contact_list: Vec<ProfileChat> = {
        let state = app_state();
        let mut contacts = state.get_all_contacts_chat();
        contacts.sort_by(|a, b| b.profile.last_connection.cmp(&a.profile.last_connection));
        contacts
    };

    let filtered_contacts = contact_list
        .into_iter()
        .filter(|contact_chat| {
            contact_chat
                .profile
                .name
                .to_lowercase()
                .contains(&search_query().to_lowercase())
                || contact_chat
                    .profile
                    .id
                    .to_lowercase()
                    .contains(&search_query().to_lowercase())
        })
        .collect::<Vec<ProfileChat>>();

    if filtered_contacts.is_empty() {
        let message = if search_query().is_empty() {
            "You have no contacts. Add some to start chatting!"
        } else {
            "No contacts found."
        };
        return rsx! {
            div { class: "p-4 text-text-secondary text-center", "{message}" }
        };
    }

    rsx! {
        ul {
            {
                filtered_contacts
                    .into_iter()
                    .map(|contact_chat| {
                        let last_message = contact_chat.last_message();
                        let profile_id = contact_chat.profile.id;
                        let profile_name = contact_chat.profile.name;
                        let avatar_url = contact_chat.profile.avatar;
                        let last_connection = contact_chat.profile.last_connection.get_u64();
                        let profile_id_open = profile_id.clone();
                        let profile_id_details = profile_id.clone();
                        let profile_id_leave = profile_id.clone();
                        let profile_name_leave = profile_name.clone();
                        rsx! {
                            ContextMenu {
                                ContextMenuTrigger {
                                    ColumnItem {
                                        id: profile_id,
                                        name: profile_name,
                                        avatar_url,
                                        last_message,
                                        last_connection,
                                        on_select: selected_topic_id,
                                        highlight: search_query(),
                                    }
                                }
                                ContextMenuContent { class: "context-menu",
                                    ContextMenuItem {
                                        class: "context-menu-item",
                                        value: "Open Chat".to_string(),
                                        index: 0usize,
                                        on_select: move |_| {
                                            selected_topic_id.set(Some(profile_id_open.clone()));
                                        },
                                        "Open Chat"
                                    }
                                    ContextMenuItem {
                                        class: "context-menu-item",
                                        value: "Open Details".to_string(),
                                        index: 1usize,
                                        on_select: move |_| {
                                            if let Some(contact) = app_state().get_contact(&profile_id_details) {
                                                show_profile_details.set(Some(contact.clone()));
                                            }
                                        },
                                        "Open Details"
                                    }
                                    ContextMenuItem {
                                        class: "context-menu-item-danger",
                                        value: "Remove Contact".to_string(),
                                        index: 2usize,
                                        on_select: move |_| {
                                            show_leave_confirmation.set(Some((
                                                profile_id_leave.clone(),
                                                profile_name_leave.clone(),
                                                RemovalType::Contact,
                                            )));
                                        },
                                        "Remove Contact"
                                    }
                                }
                            }
                        }
                    })
            }
        }
    }
}

#[component]
pub fn ColumnItem(
    id: String,
    name: String,
    avatar_url: Option<String>,
    last_message: Option<String>,
    last_connection: Option<u64>,
    on_select: Signal<Option<String>>,
    #[props(default)] highlight: Option<String>,
) -> Element {
    let last_message_display = last_message.unwrap_or_default();

    let avatar_display = if let Some(url) = &avatar_url
        && !url.is_empty()
    {
        url.clone()
    } else {
        DEFAULT_AVATAR.to_string()
    };

    let time_display = if let Some(timestamp) = last_connection {
        format_relative_time(timestamp as i64)
    } else {
        String::from("")
    };

    let name_display = if let Some(query) = highlight.as_ref().filter(|q| !q.is_empty()) {
        let name_lower = name.to_lowercase();
        let query_lower = query.to_lowercase();
        if let Some(idx) = name_lower.find(&query_lower) {
            if name.len() == name_lower.len() {
                let end = idx + query_lower.len();
                let pre = &name[..idx];
                let mat = &name[idx..end];
                let post = &name[end..];
                rsx! {
                    span {
                        "{pre}"
                        span { class: "text-accent", "{mat}" }
                        "{post}"
                    }
                }
            } else {
                rsx! { "{name}" }
            }
        } else {
            rsx! { "{name}" }
        }
    } else {
        rsx! { "{name}" }
    };

    rsx! {
        div {
            class: "list-item group",
            onclick: move |_| {
                on_select.set(Some(id.clone()));
            },
            oncontextmenu: move |e| {
                e.prevent_default();
            },
            img {
                class: "avatar w-12.5 h-12.5 shrink-0 transition-colors duration-200 group-hover:border-text-muted",
                src: "{avatar_display}",
                alt: "{name}",
                draggable: "false",
            }
            div { class: "flex-1 min-w-0 flex flex-col gap-1",
                h3 { class: "m-0 text-[clamp(14px,2vw,16px)] font-semibold text-text-primary whitespace-nowrap overflow-hidden text-ellipsis",
                    {name_display}
                }
                p { class: "m-0 text-[clamp(12px,1.8vw,14px)] text-text-secondary whitespace-nowrap overflow-hidden text-ellipsis",
                    "{last_message_display}"
                }
            }
            h3 { class: "m-0 text-[clamp(11px,1.5vw,12px)] font-normal text-text-muted shrink-0 self-start",
                "{time_display}"
            }
        }
    }
}
