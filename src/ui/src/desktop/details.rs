use super::desktop_web_components::{CLOSE_ICON, DEFAULT_AVATAR, DOWNLOAD_ICON};
use super::models::{AppState, ConnectionStatus, Controller, Profile, Topic};
use super::utils::{copy_to_clipboard, format_relative_time};
use arboard::Clipboard;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use dioxus::prelude::*;
use dioxus_primitives::toast::{ToastOptions, use_toast};
use std::rc::Rc;

#[component]
pub fn TopicDetails<C: Controller + 'static>(
    topic: Topic,
    mut toggle: Signal<Option<Topic>>,
    controller: Signal<C>,
    mut view_profile: Signal<Option<Profile>>,
    app_state: Signal<AppState>,
) -> Element {
    let toast = use_toast();
    let mut edited_title = use_signal(|| topic.name.clone());

    let handle_copy_topic_id = {
        let topic_id = topic.id.clone();
        move |_event: Event<MouseData>| match Clipboard::new() {
            Ok(clipboard) => copy_to_clipboard(clipboard, &topic_id, toast),
            Err(_) => {
                toast.error(
                    "Error accessing clipboard.".to_owned(),
                    ToastOptions::default(),
                );
            }
        }
    };

    let topic_clone = topic.clone();
    let handle_save = move |_event: Event<MouseData>| {
        let mut updated_topic = topic_clone.clone();
        updated_topic.name = edited_title().trim().to_string();
        let controller = controller;
        controller.read().modify_topic(updated_topic);
        toast.success(
            "Topic updated successfully".to_owned(),
            ToastOptions::default(),
        );
        toggle.set(None);
    };

    let topic_clone_for_image = topic.clone();
    let handle_image_change = move |event: Event<FormData>| {
        let files = event.files();
        if let Some(file) = files.first() {
            let file = file.clone();
            let topic_clone = topic_clone_for_image.clone();
            let controller = controller;
            spawn(async move {
                match file.read_bytes().await {
                    Ok(bytes) => {
                        const MAX_SIZE: usize = 512 * 1024 * 4 / 3; // 512 KB
                        if bytes.len() > MAX_SIZE {
                            toast.error(
                                "Image size must be less than 512 KB".to_owned(),
                                ToastOptions::default(),
                            );
                            return;
                        }

                        let base64 = BASE64_STANDARD.encode(&bytes);
                        let url = format!("data:image/jpeg;base64,{}", base64);

                        let mut updated_topic = topic_clone.clone();
                        updated_topic.avatar_url = Some(url);
                        controller.read().modify_topic(updated_topic);
                        toggle.set(None);

                        toast.success(
                            "Topic avatar updated successfully".to_owned(),
                            ToastOptions::default(),
                        );
                    }
                    Err(e) => {
                        toast.error(
                            format!("Failed to read file: {}", e),
                            ToastOptions::default(),
                        );
                    }
                }
            });
        } else {
            toast.error("No file selected.".to_owned(), ToastOptions::default());
        }
    };

    let avatar_url = if let Some(url) = &topic.avatar_url
        && !url.is_empty()
    {
        url.clone()
    } else {
        DEFAULT_AVATAR.to_string()
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/60 flex items-center justify-center z-1000 animate-[fadeIn_0.2s_ease]",
            onclick: move |_| toggle.set(None),
            div {
                class: "card w-[90%] max-w-125 p-6 animate-[slideIn_0.3s_ease]",
                onclick: move |e| e.stop_propagation(),
                div { class: "flex items-center gap-3 mb-5",
                    label { class: "cursor-pointer relative shrink-0 group",
                        img {
                            class: "avatar w-15 h-15 transition-all duration-200 group-hover:border-accent group-hover:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                            src: avatar_url,
                        }
                        input {
                            r#type: "file",
                            style: "display: none;",
                            onchange: handle_image_change,
                        }
                    }
                    input {
                        class: "input-field flex-1 m-0 text-2xl font-semibold border-2 border-border max-w-[90%] overflow-hidden text-ellipsis whitespace-nowrap focus:border-accent focus:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                        r#type: "text",
                        value: "{edited_title}",
                        oninput: move |e| edited_title.set(e.value()),
                    }
                    button {
                        class: "btn-primary py-3 px-6 whitespace-nowrap",
                        onclick: handle_save,
                        "Save"
                    }
                }
                hr { class: "border-none border-t border-border my-5" }
                p { class: "my-4 mb-2 text-sm font-medium text-text-secondary uppercase tracking-wider",
                    "Topic ID"
                }
                p {
                    class: "input-field m-0 border border-border font-mono text-sm break-all cursor-pointer hover:border-accent",
                    title: "Click to copy",
                    onclick: handle_copy_topic_id,
                    "{topic.id}"
                }
                div { class: "mb-4",
                    p { class: "my-4 mb-2 text-sm font-medium text-text-secondary uppercase tracking-wider",
                        "Members"
                    }
                    ul {
                        {
                            let members: Vec<Profile> = {
                                let state = app_state();
                                let own_profile = state.get_profile();
                                topic
                                    .members
                                    .iter()
                                    .map(|member_id| {
                                        if let Some(contact) = state.get_contact(member_id) {
                                            contact.clone()
                                        } else if member_id == &own_profile.id {
                                            own_profile.clone()
                                        } else {
                                            Profile::new_with_id(member_id)
                                        }
                                    })
                                    .collect()
                            };
                            members
                                .into_iter()
                                .map(|member| {
                                    let avatar = if let Some(url) = &member.avatar && !url.is_empty() {
                                        url.clone()
                                    } else {
                                        DEFAULT_AVATAR.to_string()
                                    };

                                    let last_seen = match member.last_connection {
                                        ConnectionStatus::Online => member.last_connection.to_string(),
                                        ConnectionStatus::Offline(time) => {
                                            format_relative_time((time / 1000) as i64)
                                        }
                                    };
                                    let member_clone = member.clone();
                                    rsx! {
                                        li {
                                            class: "flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors duration-200 hover:bg-bg-hover list-none",
                                            onclick: move |_| {
                                                view_profile.set(Some(member_clone.clone()));
                                            },
                                            img { class: "avatar w-10 h-10", src: "{avatar}" }
                                            div { class: "flex-1 flex flex-col gap-0.5 overflow-hidden",
                                                h3 { class: "m-0 text-sm font-medium text-text-primary whitespace-nowrap overflow-hidden text-ellipsis",
                                                    "{member.name}"
                                                }
                                                p { class: "m-0 text-xs text-text-secondary whitespace-nowrap overflow-hidden text-ellipsis",
                                                    "{last_seen}"
                                                }
                                            }
                                        }
                                    }
                                })
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ProfileDetails<C: Controller + 'static>(
    profile: Profile,
    mut toggle: Signal<Option<Profile>>,
    controller: Signal<C>,
    readonly: bool,
) -> Element {
    let toast = use_toast();
    let mut edited_name = use_signal(|| profile.name.clone());
    let mut edited_avatar = use_signal(|| profile.avatar.clone());

    let profile_id = profile.id.clone();
    let handle_copy_profile_id = {
        move |_event: Event<MouseData>| match Clipboard::new() {
            Ok(clipboard) => copy_to_clipboard(clipboard, &profile_id, toast),
            Err(_) => {
                toast.error(
                    "Error accessing clipboard.".to_owned(),
                    ToastOptions::default(),
                );
            }
        }
    };

    let profile_clone = profile.clone();
    let handle_save = move |_event: Event<MouseData>| {
        let mut updated_profile = profile_clone.clone();
        updated_profile.name = edited_name().trim().to_string();
        updated_profile.avatar = edited_avatar().clone();
        let controller = controller;
        controller.read().modify_profile(updated_profile);
        toast.success(
            "Profile updated successfully".to_owned(),
            ToastOptions::default(),
        );
        toggle.set(None);
    };

    let handle_image_change = move |event: Event<FormData>| {
        let files = event.files();
        if let Some(file) = files.first() {
            let file = file.clone();
            spawn(async move {
                match file.read_bytes().await {
                    Ok(bytes) => {
                        const MAX_SIZE: usize = 512 * 1024 * 4 / 3; // 512 KB
                        if bytes.len() > MAX_SIZE {
                            toast.error(
                                "Image size must be less than 512 KB".to_owned(),
                                ToastOptions::default(),
                            );
                            return;
                        }

                        let base64 = BASE64_STANDARD.encode(&bytes);
                        let url = format!("data:image/jpeg;base64,{}", base64);

                        edited_avatar.set(Some(url));

                        toast.success(
                            "Profile avatar updated successfully".to_owned(),
                            ToastOptions::default(),
                        );
                    }
                    Err(e) => {
                        toast.error(
                            format!("Failed to read file: {}", e),
                            ToastOptions::default(),
                        );
                    }
                }
            });
        } else {
            toast.error("No file selected.".to_owned(), ToastOptions::default());
        }
    };

    let avatar_url = if let Some(url) = edited_avatar()
        && !url.is_empty()
    {
        url.clone()
    } else {
        DEFAULT_AVATAR.to_string()
    };

    let last_connection_text = match profile.last_connection {
        ConnectionStatus::Online => profile.last_connection.to_string(),
        ConnectionStatus::Offline(time) => format_relative_time((time / 1000) as i64),
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/70 flex justify-center items-center z-2000 animate-[fadeIn_0.2s_ease]",
            onclick: move |_| toggle.set(None),
            div {
                class: "card w-[90%] max-w-125 p-6 animate-[slideIn_0.3s_ease]",
                onclick: move |e| e.stop_propagation(),
                div { class: "flex items-center gap-3 mb-5",
                    label { class: "cursor-pointer relative shrink-0 group",
                        img {
                            class: "avatar w-15 h-15 transition-all duration-200 group-hover:border-accent group-hover:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                            src: avatar_url,
                        }
                        if !readonly {
                            input {
                                r#type: "file",
                                accept: "image/*",
                                style: "display: none;",
                                onchange: handle_image_change,
                            }
                            if profile.avatar.is_some_and(|url| !url.is_empty()) {
                                button {
                                    class: "absolute -top-1 -right-1 w-5 h-5 rounded-full bg-danger hover:bg-danger-hover text-white text-xs font-bold flex items-center justify-center cursor-pointer transition-all duration-200 hover:scale-110 shadow-md pb-0.5 opacity-0 pointer-events-none group-hover:opacity-100 group-hover:pointer-events-auto",
                                    title: "Remove Avatar",
                                    onclick: move |e| {
                                        e.stop_propagation();
                                        edited_avatar.set(None);
                                    },
                                    "✕"
                                }
                            }
                        }
                    }
                    input {
                        class: "input-field flex-1 m-0 text-2xl font-semibold border-2 border-border min-w-0 focus:border-accent focus:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                        r#type: "text",
                        value: "{edited_name}",
                        placeholder: "Display Name",
                        readonly: "{readonly}",
                        oninput: move |e| edited_name.set(e.value()),
                    }
                    if !readonly {
                        button {
                            class: "btn-primary py-3 px-6 whitespace-nowrap",
                            onclick: handle_save,
                            "Save"
                        }
                    }
                }
                hr { class: "border-none border-t border-border my-5" }

                div { class: "mb-4",
                    p { class: "m-0 mb-2 text-sm font-medium text-text-secondary uppercase tracking-wider",
                        "Profile ID"
                    }
                    p {
                        class: "input-field m-0 border border-border font-mono text-sm break-all cursor-pointer hover:border-accent",
                        title: "Click to copy",
                        onclick: handle_copy_profile_id,
                        "{profile.id}"
                    }
                }

                div { class: "mb-0",
                    p { class: "m-0 mb-2 text-sm font-medium text-text-secondary uppercase tracking-wider",
                        "Last Active"
                    }
                    p { class: "input-field m-0 border border-border text-sm",
                        "{last_connection_text}"
                    }
                }
            }
        }
    }
}

#[component]
pub fn ImageDetails(image: String, on_close: EventHandler<()>) -> Element {
    let image_rc = Rc::new(image);
    let image_clone = image_rc.clone();
    rsx! {
        div {
            class: "fixed inset-0 bg-black/70 flex justify-center items-center z-2000 animate-[fadeIn_0.2s_ease]",
            onclick: move |_| on_close.call(()),
            button {
                class: "absolute top-5 right-16 w-10 h-10 rounded-full bg-white/10 backdrop-blur-sm text-white text-lg font-bold flex items-center justify-center cursor-pointer transition-all duration-200 hover:bg-white/20 hover:scale-110 hover:shadow-lg active:scale-95",
                title: "Download",
                onclick: move |e| {
                    e.stop_propagation();
                    let image_clone = image_clone.clone();
                    spawn(async move {
                        let dir = dirs::download_dir()

                            .unwrap_or_else(|| std::path::PathBuf::from("."));
                        let file = rfd::AsyncFileDialog::new()
                            .set_file_name("image.png")
                            .set_directory(dir)
                            .save_file()
                            .await;
                        let bytes = image_clone
                            .to_string()
                            .split(",")
                            .nth(1)
                            .and_then(|b64| BASE64_STANDARD.decode(b64).ok())
                            .unwrap_or_default();
                        if let Some(path) = file && let Err(err) = std::fs::write(path.path(), bytes)
                        {
                            println!("Error saving file: {}", err);
                        }
                    });
                },
                img { class: "w-6 h-6 drop-shadow-md", src: DOWNLOAD_ICON }
            }
            button {
                class: "absolute top-5 right-5 w-10 h-10 rounded-full bg-white/10 backdrop-blur-sm text-white text-lg font-bold flex items-center justify-center cursor-pointer transition-all duration-200 hover:bg-danger/80 hover:scale-110 hover:shadow-lg active:scale-95",
                title: "Close",
                onclick: move |e| {
                    e.stop_propagation();
                    on_close.call(());
                },
                img { class: "w-5 h-5", src: CLOSE_ICON }
            }
            div {
                class: "w-max max-w-300 p-6 animate-[slideIn_0.3s_ease]",
                onclick: move |e| e.stop_propagation(),
                img {
                    class: "max-w-full max-h-[90vh] mx-auto rounded-lg",
                    src: "{image_rc}",
                }
            }
        }
    }
}
