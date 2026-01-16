use super::desktop_web_components::{CLIP_ICON, DEFAULT_AVATAR};
use super::models::{AppState, Message};
use super::utils::{format_message_timestamp, get_sender_display_name, process_image};
use dioxus::html::FileData;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn Chat(
    app_state: Signal<AppState>,
    topic_id: Option<String>,
    on_send_message: EventHandler<(String, String)>,
    on_send_message_dm: EventHandler<(String, String)>,
    on_image_send: EventHandler<(String, Vec<u8>)>,
    show_image_details: Signal<Option<String>>,
) -> Element {
    let state = app_state();
    let mut show_attachment = use_signal(|| false);

    let topic = if let Some(id) = &topic_id {
        state.get_topic(id)
    } else {
        None
    };

    let contact = if let Some(id) = &topic_id {
        state.get_contact_chat(id)
    } else {
        None
    };

    let (messages, title_text, avatar_url, chat_id) = if let Some(topic) = topic {
        (
            topic.messages.clone(),
            topic.name.clone(),
            topic
                .avatar_url
                .clone()
                .unwrap_or(DEFAULT_AVATAR.to_string()),
            topic.id.clone(),
        )
    } else if let Some(contact) = contact {
        (
            contact
                .messages
                .iter()
                .cloned()
                .map(Message::from)
                .collect::<Vec<Message>>(),
            contact.profile.name.clone(),
            contact
                .profile
                .avatar
                .clone()
                .unwrap_or(DEFAULT_AVATAR.to_string()),
            contact.profile.id.clone(),
        )
    } else {
        (
            Vec::<Message>::new(),
            String::new(),
            String::new(),
            String::new(),
        )
    };

    if !chat_id.is_empty() {
        let mut message_input = use_signal(String::new);

        let mut tracked_id = use_signal(|| chat_id.clone());
        let mut last_msg_count = use_signal(|| 0);

        if *tracked_id.read() != chat_id {
            tracked_id.set(chat_id.clone());
            last_msg_count.set(0);
        }

        use_effect(move || {
            let state = app_state();
            let current_id = tracked_id.read();

            let count = if let Some(t) = state.get_topic(&current_id) {
                t.messages.len()
            } else if let Some(c) = state.get_contact_chat(&current_id) {
                c.messages.len()
            } else {
                0
            };

            if count != *last_msg_count.read() {
                last_msg_count.set(count);
                document::eval(
                    r#"
                        requestAnimationFrame(() => {
                            const element = document.getElementById("chat-messages-container");
                            if (element) {
                                element.scrollTop = element.scrollHeight;
                            }
                        });
                    "#,
                );
            }
        });

        let send_message = use_callback({
            let id = chat_id.clone();
            let is_dm = contact.is_some();
            move |_| {
                let content = message_input().trim().to_string();
                if !content.is_empty() {
                    if is_dm {
                        on_send_message_dm.call((id.clone(), content));
                    } else {
                        on_send_message.call((id.clone(), content));
                    }
                    message_input.set(String::new());
                }
            }
        });

        let handle_media_submit = {
            move |files: Vec<FileData>| {
                let chat_id = chat_id.clone();
                let mut show_attachment = show_attachment;
                spawn(async move {
                    for file in files {
                        if let Ok(data) = file.read_bytes().await {
                            let processed_data =
                                process_image(&data).expect("Failed to process image");
                            on_image_send.call((chat_id.clone(), processed_data));
                        }
                    }
                    show_attachment.set(false);
                });
            }
        };

        rsx! {
            div { class: "flex-1 flex flex-col bg-bg-input h-full",
                div { class: "bg-bg-panel py-3.75 px-5 shadow-md flex items-center gap-3.75 border-b border-border",
                    img { class: "avatar w-11.25 h-11.25", src: "{avatar_url}" }
                    h2 {
                        class: "m-0 text-[clamp(1.1rem,2.5vw,1.4rem)] font-semibold text-text-primary max-w-100 overflow-hidden text-ellipsis whitespace-nowrap",
                        title: "{title_text}",
                        "{title_text}"
                    }
                }
                div {
                    class: "flex-1 overflow-y-auto p-5 flex flex-col gap-3 bg-bg-dark scrollbar-custom",
                    id: "chat-messages-container",
                    for message in messages.iter() {
                        ChatMessageComponent {
                            message: message.clone(),
                            app_state,
                            show_image_details,
                        }
                    }
                }
                if show_attachment() {
                    AttachComponent {
                        on_select_media: handle_media_submit,
                        on_close: move |_| show_attachment.set(false),
                    }
                }
                div { class: "bg-bg-dark py-3.75 px-5 flex gap-3 items-center",
                    div { class: "flex-1 flex gap-2 items-center bg-bg-input rounded-lg border border-border px-4 py-2.5 transition-all duration-200 focus-within:border-accent-primary focus-within:shadow-[0_0_0_2px_rgba(59,130,246,0.2)]",
                        input {
                            class: "flex-1 bg-transparent border-none outline-none text-text-primary placeholder:text-text-secondary",
                            r#type: "text",
                            placeholder: "Type a message...",
                            value: "{message_input()}",
                            oninput: move |e| {
                                message_input.set(e.value());
                            },
                            onkeypress: move |e| {
                                if e.key() == Key::Enter {
                                    send_message(());
                                }
                            },
                        }
                        button {
                            class: "btn-icon w-8 h-8 rounded-lg [&>img]:w-5 [&>img]:h-5 [&>img]:brightness-0 [&>img]:saturate-100 [&>img]:invert-73 [&>img]:sepia-0 [&>img]:hue-rotate-180 [&>img]:contrast-88 [&>img]:transition-[filter] [&>img]:duration-200 [&:hover>img]:saturate-7500",
                            onclick: move |_| show_attachment.set(true),
                            img { src: CLIP_ICON }
                        }
                    }
                    button {
                        class: "btn-primary py-3 px-6",
                        onclick: move |_| {
                            send_message(());
                        },
                        "Send"
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "flex-1 flex items-center justify-center bg-bg-input text-text-secondary",
                h2 { class: "text-[clamp(1.2rem,3vw,1.8rem)] font-medium m-0",
                    "Select a chat to start messaging"
                }
            }
        }
    }
}

#[component]
pub fn AttachComponent(
    on_select_media: EventHandler<Vec<FileData>>,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "fixed inset-0 z-40", onclick: move |_| on_close.call(()),
            div {
                class: "absolute bottom-24 right-24 bg-bg-panel rounded-xl shadow-lg border border-border p-2 min-w-48 animate-[slideIn_0.2s_ease]",
                onclick: move |e| e.stop_propagation(),
                div { class: "flex flex-row gap-1",
                    label { class: "flex items-center gap-3 px-4 py-3 rounded-lg cursor-pointer transition-all duration-200 hover:bg-bg-hover active:bg-bg-active group",
                        input {
                            class: "hidden",
                            r#type: "file",
                            multiple: true,
                            accept: "image/*,video/*",
                            onchange: move |e| on_select_media.call(e.files()),
                        }
                        p { class: "text-text-primary font-medium", "Photo/Video" }
                    }
                    button { class: "flex items-center gap-3 px-4 py-3 rounded-lg cursor-pointer transition-all duration-200 hover:bg-bg-hover active:bg-bg-active text-left",
                        p { class: "text-text-primary font-medium", "Files" }
                    }
                    button { class: "flex items-center gap-3 px-4 py-3 rounded-lg cursor-pointer transition-all duration-200 hover:bg-bg-hover active:bg-bg-active text-left",
                        p { class: "text-text-primary font-medium", "Audio" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ChatMessageComponent(
    message: Message,
    app_state: Signal<AppState>,
    show_image_details: Signal<Option<String>>,
) -> Element {
    let state = app_state();
    match message {
        Message::Chat(message) => {
            let timestamp_str = format_message_timestamp(message.timestamp);
            let sender_display = get_sender_display_name(&state, &message.sender_id);
            rsx! {
                div { class: if message.is_sent { "message-bubble-sent" } else { "message-bubble-received" },
                    p {
                        class: "message-sender-id m-0 mb-1 text-[clamp(11px,1.6vw,12px)] font-medium opacity-80 text-text-secondary whitespace-nowrap overflow-hidden text-ellipsis",
                        title: "{message.sender_id}",
                        "{sender_display}"
                    }
                    p { class: "m-0 max-w-96 text-[clamp(14px,2vw,15px)] leading-snug wrap-break-word",
                        "{message.content}"
                    }
                    p { class: "m-0 text-[clamp(10px,1.5vw,11px)] opacity-70 self-end",
                        "{timestamp_str}"
                    }
                }
            }
        }
        Message::Leave(message) => {
            let timestamp_str = format_message_timestamp(message.timestamp);
            let sender_display = get_sender_display_name(&state, &message.sender_id);
            rsx! {
                div { class: "max-w-full self-center bg-transparent text-text-muted py-2 px-3 text-[clamp(12px,1.8vw,13px)] italic text-center",
                    p { class: "m-0 text-[clamp(12px,1.8vw,13px)] opacity-85 text-text-muted",
                        "{sender_display} has left the topic."
                    }
                    p { class: "mt-1 mb-0 text-[clamp(10px,1.5vw,11px)] opacity-60 text-text-muted",
                        "{timestamp_str}"
                    }
                }
            }
        }
        Message::Join(message) => {
            let timestamp_str = format_message_timestamp(message.timestamp);
            let sender_display = get_sender_display_name(&state, &message.sender_id);
            let text = if message.me {
                format!("{sender_display} joined the topic.")
            } else {
                format!("{sender_display} has joined the topic.")
            };

            rsx! {
                div { class: "max-w-full self-center bg-transparent text-text-muted py-2 px-3 text-[clamp(12px,1.8vw,13px)] italic text-center",
                    p { class: "m-0 text-[clamp(12px,1.8vw,13px)] opacity-85 text-text-muted",
                        "{text}"
                    }
                    p { class: "mt-1 mb-0 text-[clamp(10px,1.5vw,11px)] opacity-60 text-text-muted",
                        "{timestamp_str}"
                    }
                }
            }
        }
        Message::Disconnect(message) => {
            let timestamp_str = format_message_timestamp(message.timestamp);
            let sender_display = get_sender_display_name(&state, &message.sender_id);
            rsx! {
                div { class: "max-w-full self-center bg-transparent text-text-muted py-2 px-3 text-[clamp(12px,1.8vw,13px)] italic text-center",
                    p { class: "m-0 text-[clamp(12px,1.8vw,13px)] opacity-85 text-text-muted",
                        "{sender_display} has disconnected."
                    }
                    p { class: "mt-1 mb-0 text-[clamp(10px,1.5vw,11px)] opacity-60 text-text-muted",
                        "{timestamp_str}"
                    }
                }
            }
        }
        Message::Image(message) => {
            let url = Rc::new(format!("data:image/webp;base64,{}", message.image_url));
            let sender_display = get_sender_display_name(&state, &message.sender_id);
            let alignment = if message.is_sent {
                "self-end"
            } else {
                "self-start"
            };
            let url_clone = url.clone();

            rsx! {
                div {
                    class: "max-w-[50%] flex flex-col gap-1 {alignment}",
                    onclick: move |_| show_image_details.set(Some(url_clone.to_string())),
                    if !message.is_sent {
                        p {
                            class: "m-0 text-[clamp(11px,1.6vw,12px)] font-medium opacity-80 text-text-secondary whitespace-nowrap overflow-hidden text-ellipsis",
                            title: "{message.sender_id}",
                            "{sender_display}"
                        }
                    }
                    img {
                        class: "max-w-96 rounded-xl shadow-md",
                        src: "{url}",
                        alt: "Image message",
                    }
                    p { class: "m-0 text-[clamp(10px,1.5vw,11px)] opacity-70 text-text-secondary self-end",
                        "{format_message_timestamp(message.timestamp)}"
                    }
                }
            }
        }
    }
}
