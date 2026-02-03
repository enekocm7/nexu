use super::desktop_web_components::CLOSE_ICON;
use super::models::{Controller, RemovalType};
use dioxus::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopicCreationMode {
    Create,
    Join,
}

#[component]
pub fn TopicDialog<C: Controller + 'static>(
    mut toggle: Signal<bool>,
    controller: Signal<C>,
) -> Element {
    let mut topic_name = use_signal(String::new);
    let mut selected_mode = use_signal(|| TopicCreationMode::Create);

    let handle_submit = move |_| {
        let mode = selected_mode();
        let name = topic_name().trim().to_string();
        let controller = controller;

        if !name.is_empty() {
            match mode {
                TopicCreationMode::Create => controller.read().create_topic(name),
                TopicCreationMode::Join => controller.read().join_topic(name),
            }
            toggle.set(false);
            topic_name.set(String::new());
        }
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/60 flex items-center justify-center z-1000 animate-[fadeIn_0.2s_ease]",
            onclick: move |_| {
                toggle.set(false);
                topic_name.set(String::new());
            },
            div {
                class: "card w-[90%] max-w-125 animate-[slideIn_0.3s_ease]",
                onclick: move |e| {
                    e.stop_propagation();
                },
                div { class: "flex justify-between items-center py-5 px-6 border-b border-border",
                    h3 { class: "m-0 text-xl font-semibold text-text-primary", "New Topic" }
                    button {
                        class: "btn-icon w-8 h-8 rounded-lg [&>img]:w-5 [&>img]:h-5 [&>img]:brightness-0 [&>img]:saturate-100 [&>img]:invert-73 [&>img]:sepia-0 [&>img]:hue-rotate-180 [&>img]:contrast-88 [&>img]:transition-[filter] [&>img]:duration-200 [&:hover>img]:invert-100 [&:hover>img]:sepia-0 [&:hover>img]:saturate-7500 [&:hover>img]:hue-rotate-324 [&:hover>img]:brightness-103 [&:hover>img]:contrast-103",
                        onclick: move |_| {
                            toggle.set(false);
                            topic_name.set(String::new());
                        },
                        img { src: CLOSE_ICON }
                    }
                }
                div { class: "p-6",
                    div { class: "flex gap-2 mb-6 bg-bg-input p-1 rounded-[10px]",
                        button {
                            class: if *selected_mode.read() == TopicCreationMode::Create { "flex-1 py-2.5 px-4 bg-bg-subtle border-none rounded-lg text-text-primary text-sm font-medium cursor-pointer transition-all duration-200 shadow-sm" } else { "flex-1 py-2.5 px-4 bg-transparent border-none rounded-lg text-text-secondary text-sm font-medium cursor-pointer transition-all duration-200 hover:bg-bg-hover hover:text-text-primary" },
                            onclick: move |_| selected_mode.set(TopicCreationMode::Create),
                            "Create Topic"
                        }
                        button {
                            class: if *selected_mode.read() == TopicCreationMode::Join { "flex-1 py-2.5 px-4 bg-bg-subtle border-none rounded-lg text-text-primary text-sm font-medium cursor-pointer transition-all duration-200 shadow-sm" } else { "flex-1 py-2.5 px-4 bg-transparent border-none rounded-lg text-text-secondary text-sm font-medium cursor-pointer transition-all duration-200 hover:bg-bg-hover hover:text-text-primary" },
                            onclick: move |_| selected_mode.set(TopicCreationMode::Join),
                            "Join Topic"
                        }
                    }
                    div { class: "mb-5",
                        label { class: "block text-text-secondary text-sm font-medium mb-2",
                            if *selected_mode.read() == TopicCreationMode::Create {
                                "Topic Name"
                            } else {
                                "Topic ID or Invite Link"
                            }
                        }
                        input {
                            class: "input-field border-2 border-border focus:border-accent focus:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                            r#type: "text",
                            value: "{topic_name}",
                            placeholder: if *selected_mode.read() == TopicCreationMode::Create { "Enter topic name..." } else { "Enter topic ID or paste invite link..." },
                            oninput: move |e| topic_name.set(e.value()),
                        }
                    }
                    p { class: "m-0 text-text-secondary text-[13px] leading-relaxed",
                        if *selected_mode.read() == TopicCreationMode::Create {
                            "Create a new topic to start chatting with others. You can share the topic ID with your friends."
                        } else {
                            "Join an existing topic by entering its ID or invite link shared by a friend."
                        }
                    }
                }
                div { class: "flex gap-3 justify-end py-5 px-6 border-t border-border bg-bg-input",
                    button {
                        class: "btn-secondary py-2.5 px-6",
                        onclick: move |_| {
                            toggle.set(false);
                            topic_name.set(String::new());
                        },
                        "Cancel"
                    }
                    button {
                        class: "btn-primary py-2.5 px-6 disabled:bg-bg-subtle disabled:text-text-muted disabled:cursor-not-allowed disabled:shadow-none",
                        disabled: topic_name().trim().is_empty(),
                        onclick: handle_submit,
                        if *selected_mode.read() == TopicCreationMode::Create {
                            "Create"
                        } else {
                            "Join"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ContactDialog<C: Controller + 'static>(
    mut toggle: Signal<bool>,
    controller: Signal<C>,
) -> Element {
    let mut address_str = use_signal(String::new);

    let handle_submit = move |_| {
        let addr = address_str().trim().to_string();
        let controller = controller;
        if !addr.is_empty() {
            controller.read().connect_to_user(addr);
            toggle.set(false);
            address_str.set(String::new());
        }
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/60 flex items-center justify-center z-1000 animate-[fadeIn_0.2s_ease]",
            onclick: move |_| {
                toggle.set(false);
                address_str.set(String::new());
            },
            div {
                class: "card w-[90%] max-w-125 animate-[slideIn_0.3s_ease]",
                onclick: move |e| {
                    e.stop_propagation();
                },
                div { class: "flex justify-between items-center py-5 px-6 border-b border-border",
                    h3 { class: "m-0 text-xl font-semibold text-text-primary", "Add Contact" }
                    button {
                        class: "btn-icon w-8 h-8 rounded-lg [&>img]:w-5 [&>img]:h-5 [&>img]:brightness-0 [&>img]:saturate-100 [&>img]:invert-73 [&>img]:sepia-0 [&>img]:hue-rotate-180 [&>img]:contrast-88 [&>img]:transition-[filter] [&>img]:duration-200 [&:hover>img]:invert-100 [&:hover>img]:sepia-0 [&:hover>img]:saturate-7500 [&:hover>img]:hue-rotate-324 [&:hover>img]:brightness-103 [&:hover>img]:contrast-103",
                        onclick: move |_| {
                            toggle.set(false);
                            address_str.set(String::new());
                        },
                        img { src: CLOSE_ICON }
                    }
                }
                div { class: "p-6",
                    div { class: "mb-5",
                        label { class: "block text-text-secondary text-sm font-medium mb-2",
                            "Contact Id"
                        }
                        input {
                            class: "input-field border-2 border-border focus:border-accent focus:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                            r#type: "text",
                            value: "{address_str}",
                            placeholder: "Enter contact id...",
                            oninput: move |e| address_str.set(e.value()),
                        }
                    }
                    p { class: "m-0 text-text-secondary text-[13px] leading-relaxed",
                        "Enter the id of the user you want to add to your contacts."
                    }
                }
                div { class: "flex gap-3 justify-end py-5 px-6 border-t border-border bg-bg-input",
                    button {
                        class: "btn-secondary py-2.5 px-6",
                        onclick: move |_| {
                            toggle.set(false);
                            address_str.set(String::new());
                        },
                        "Cancel"
                    }
                    button {
                        class: "btn-primary py-2.5 px-6 disabled:bg-bg-subtle disabled:text-text-muted disabled:cursor-not-allowed disabled:shadow-none",
                        disabled: address_str().trim().is_empty(),
                        onclick: handle_submit,
                        "Add Contact"
                    }
                }
            }
        }
    }
}

#[component]
pub fn ConfirmationDialog(
    title: String,
    message: String,
    confirm_text: String,
    cancel_text: String,
    is_danger: bool,
    mut toggle: Signal<Option<(String, String, RemovalType)>>,
    on_confirm: EventHandler<()>,
) -> Element {
    let button_class = if is_danger {
        "btn-danger py-2.5 px-6"
    } else {
        "btn-primary py-2.5 px-6"
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/70 flex items-center justify-center z-1001 animate-[fadeIn_0.2s_ease]",
            onclick: move |_| toggle.set(None),
            div {
                class: "card w-[90%] max-w-112.5 animate-[slideIn_0.3s_ease]",
                onclick: move |e| e.stop_propagation(),
                div { class: "flex justify-between items-center py-5 px-6 border-b border-border",
                    h3 { class: "m-0 text-xl font-semibold text-text-primary", "{title}" }
                    button {
                        class: "btn-icon w-8 h-8 rounded-lg [&>img]:w-5 [&>img]:h-5 [&>img]:brightness-0 [&>img]:saturate-100 [&>img]:invert-73 [&>img]:sepia-0 [&>img]:hue-rotate-180 [&>img]:contrast-88 [&>img]:transition-[filter] [&>img]:duration-200 [&:hover>img]:invert-100 [&:hover>img]:sepia-0 [&:hover>img]:saturate-7500 [&:hover>img]:hue-rotate-324 [&:hover>img]:brightness-103 [&:hover>img]:contrast-103",
                        onclick: move |_| toggle.set(None),
                        img { src: CLOSE_ICON }
                    }
                }
                div { class: "p-6",
                    p { class: "m-0 text-text-secondary text-[15px] leading-relaxed",
                        "{message}"
                    }
                }
                div { class: "flex gap-3 justify-end py-5 px-6 border-t border-border bg-bg-input",
                    button {
                        class: "btn-secondary py-2.5 px-6",
                        onclick: move |_| toggle.set(None),
                        "{cancel_text}"
                    }
                    button {
                        class: "{button_class}",
                        onclick: move |_| on_confirm.call(()),
                        "{confirm_text}"
                    }
                }
            }
        }
    }
}

#[component]
pub fn ProgressBar(title: String, progress: Signal<u64>) -> Element {
    rsx! {
        div { class: "fixed inset-0 bg-black/70 flex items-center justify-center z-1001 animate-[fadeIn_0.2s_ease]",
            div {
                class: "card w-[90%] max-w-112.5 animate-[slideIn_0.3s_ease]",
                onclick: move |e| e.stop_propagation(),
                div { class: "flex flex-col justify-between items-center py-5 px-6 border-b border-border",
                    h3 { class: "m-0 text-xl font-semibold text-text-primary pb-3",
                        "{title}"
                    }
                    progress {
                        class: "w-full h-2 bg-gray-200 rounded-full overflow-hidden",
                        //TODO Ver cual es el valor maximo
                        max: "100",
                        value: "{progress}",
                    }
                }
            }
        }
    }
}
