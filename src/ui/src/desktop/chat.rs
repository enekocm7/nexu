use super::desktop_web_components::{CLIP_ICON, DEFAULT_AVATAR};
use super::models::{AppState, BlobType, Controller, Message};
use super::utils::{format_message_timestamp, get_sender_display_name, is_video_file};
use crate::components::toast::ToastProvider;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use dioxus::desktop::{AssetRequest, RequestAsyncResponder, use_asset_handler};
use dioxus::html::FileData;
use dioxus::prelude::*;
use dioxus_primitives::toast::{ToastOptions, use_toast};
use image::ImageFormat::WebP;
use image::ImageReader;
use std::io::Cursor;

#[component]
pub fn Chat<C: Controller + 'static>(
    app_state: Signal<AppState>,
    topic_id: Option<String>,
    controller: Signal<C>,
    show_image_details: Signal<Option<(String, String)>>,
    show_video_details: Signal<Option<(String, String)>>,
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
            let controller = controller;
            move |_| {
                let content = message_input().trim().to_string();
                if !content.is_empty() {
                    if is_dm {
                        controller.read().send_message_to_user(id.clone(), content);
                    } else {
                        controller.read().send_message_to_topic(id.clone(), content);
                    }
                    message_input.set(String::new());
                }
            }
        });

        let handle_media_submit = {
            let controller = controller;
            move |files: Vec<FileData>| {
                let chat_id = chat_id.clone();
                let mut show_attachment = show_attachment;
                spawn(async move {
                    for file in files {
                        let name = file.name().to_owned();
                        let blob_type = if is_video_file(&name) {
                            BlobType::Video
                        } else {
                            BlobType::Image
                        };
                        controller.read().send_blob_to_topic(
                            chat_id.clone(),
                            file,
                            name,
                            blob_type,
                        );
                    }
                    show_attachment.set(false)
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
                        ToastProvider {
                            ChatMessageComponent {
                                message: message.clone(),
                                app_state,
                                show_image_details,
                                show_video_details,
                                controller,
                            }
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
pub fn ChatMessageComponent<C: Controller + 'static>(
    message: Message,
    app_state: Signal<AppState>,
    show_image_details: Signal<Option<(String, String)>>,
    show_video_details: Signal<Option<(String, String)>>,
    controller: Signal<C>,
) -> Element {
    let state = app_state();
    let toast = use_toast();
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
            let sender_display = get_sender_display_name(&state, &message.sender_id);
            let alignment = if message.is_sent {
                "self-end"
            } else {
                "self-start"
            };

            let img_path = controller.read().get_or_download(
                &message.blob_hash,
                &message.sender_id,
                &message.blob_name,
            );

            let img_path = match img_path {
                Ok(path) => path,
                Err(_) => {
                    toast.error(
                        "Failed to load image blob.".to_string(),
                        ToastOptions::default(),
                    );
                    return rsx! {};
                }
            };

            let mut thumbnail_data = use_signal(|| String::from(""));
            let mut is_too_large = use_signal(|| false);

            if message.blob_size > 10_000_000 {
                is_too_large.set(true);
            }

            let path = img_path.clone();
            let external_path = img_path.clone();
            use_effect(move || {
                if is_too_large() {
                    return;
                }
                let path = path.clone();
                spawn(async move {
                    let reader = match ImageReader::open(&path) {
                        Ok(r) => r,
                        Err(e) => {
                            toast.error(
                                format!("Failed to open image: {}", e),
                                ToastOptions::default(),
                            );
                            return;
                        }
                    };

                    let reader = match reader.with_guessed_format() {
                        Ok(r) => r,
                        Err(e) => {
                            toast.error(
                                format!("Failed to detect image format: {}", e),
                                ToastOptions::default(),
                            );
                            return;
                        }
                    };

                    let img = match reader.decode() {
                        Ok(i) => i,
                        Err(e) => {
                            toast.error(
                                format!("Failed to decode image: {}", e),
                                ToastOptions::default(),
                            );
                            return;
                        }
                    };

                    let (width, height) = (img.width(), img.height());
                    if width > 4096 || height > 4096 {
                        is_too_large.set(true);
                        return;
                    }

                    let thumbnail = img.thumbnail(1024, 1024);
                    let mut buf = Cursor::new(Vec::new());

                    if let Err(e) = thumbnail.write_to(&mut buf, WebP) {
                        toast.error(
                            format!("Failed to create thumbnail: {}", e),
                            ToastOptions::default(),
                        );
                        return;
                    }

                    let b64 = BASE64_STANDARD.encode(buf.get_ref());
                    thumbnail_data.set(format!("data:image/webp;base64,{}", b64));
                });
            });

            rsx! {
                div { class: "max-w-[50%] flex flex-col gap-1 {alignment}",
                    if !message.is_sent {
                        p {
                            class: "m-0 text-[clamp(11px,1.6vw,12px)] font-medium opacity-80 text-text-secondary whitespace-nowrap overflow-hidden text-ellipsis",
                            title: "{message.sender_id}",
                            "{sender_display}"
                        }
                    }
                    if is_too_large() {
                        div { class: "bg-bg-panel rounded-xl p-4 border border-border flex flex-col gap-2 items-center",
                            p { class: "m-0 text-text-secondary text-sm", "{message.blob_name}" }
                            p { class: "m-0 text-text-secondary text-sm",
                                "{message.blob_size / 1_000_000} MB"
                            }
                            button {
                                class: "btn-primary py-2 px-4 text-sm",
                                onclick: move |_| {
                                    let path = external_path.as_os_str();
                                    if open::that(path).is_err() {
                                        toast
                                            .error(
                                                "Failed to open external viewer.".to_string(),
                                                ToastOptions::default(),
                                            );
                                    }
                                },
                                "Open in External Viewer"
                            }
                        }
                    } else {
                        img {
                            class: "max-w-96 rounded-xl shadow-md cursor-pointer",
                            src: "{thumbnail_data.read()}",
                            onclick: move |_| {
                                show_image_details.set(Some((thumbnail_data(), message.blob_name.clone())))
                            },
                        }
                    }
                    p { class: "m-0 text-[clamp(10px,1.5vw,11px)] opacity-70 text-text-secondary self-end",
                        "{format_message_timestamp(message.timestamp)}"
                    }
                }
            }
        }
        Message::Video(message) => {
            let sender_display = get_sender_display_name(&state, &message.sender_id);
            let alignment = if message.is_sent {
                "self-end"
            } else {
                "self-start"
            };

            let video_path = controller.read().get_or_download(
                &message.blob_hash,
                &message.sender_id,
                &message.blob_name,
            );

            let video_path = match video_path {
                Ok(path) => path,
                Err(_) => {
                    toast.error(
                        "Failed to load video blob.".to_string(),
                        ToastOptions::default(),
                    );
                    return rsx! {};
                }
            };

            let video_path_clone = video_path.clone();

            use_asset_handler(
                "video",
                move |request: AssetRequest, responder: RequestAsyncResponder| {
                    let request = request.clone();
                    let video_path = video_path_clone.clone();
                    spawn(async move {
                        let mut file = tokio::fs::File::open(&video_path).await.unwrap();

                        match C::get_stream_response(&mut file, &request).await {
                            Ok(response) => responder.respond(response),
                            Err(err) => eprintln!("Error: {}", err),
                        }
                    });
                },
            );

            let video_path_str = format!("/video/{}", message.blob_name.clone());
            let video_path_for_click = video_path_str.clone();
            let blob_name = message.blob_name.clone();

            rsx! {
                div { class: "max-w-[50%] flex flex-col gap-1 {alignment}",
                    if !message.is_sent {
                        p {
                            class: "m-0 text-[clamp(11px,1.6vw,12px)] font-medium opacity-80 text-text-secondary whitespace-nowrap overflow-hidden text-ellipsis",
                            title: "{message.sender_id}",
                            "{sender_display}"
                        }
                    }
                    div {
                        class: "relative cursor-pointer group",
                        onclick: move |_| {
                            show_video_details.set(Some((video_path_for_click.clone(), blob_name.clone())));
                        },
                        video {
                            class: "max-w-96 rounded-xl shadow-md",
                            src: "{video_path_str}",
                            preload: "metadata",
                        }
                        div { class: "absolute inset-0 flex items-center justify-center bg-black/30 rounded-xl group-hover:bg-black/40 transition-all duration-200",
                            div { class: "w-16 h-16 rounded-full bg-white/90 flex items-center justify-center shadow-lg group-hover:scale-110 transition-transform duration-200",
                                div { class: "w-0 h-0 border-t-8 border-t-transparent border-b-8 border-b-transparent border-l-12 border-l-gray-800 ml-1" }
                            }
                        }
                    }
                    p { class: "m-0 text-[clamp(10px,1.5vw,11px)] opacity-70 text-text-secondary self-end",
                        "{format_message_timestamp(message.timestamp)}"
                    }
                }
            }
        }
    }
}
