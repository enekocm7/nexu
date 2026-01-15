pub mod chat;
pub mod columns;
pub mod details;
pub mod dialogs;
pub mod models;
pub mod utils;

#[cfg(feature = "desktop-web")]
pub mod desktop_web_components {
    use crate::desktop::dialogs::TopicDialog;

    pub use super::chat::Chat;
    pub use super::columns::{ContactColumn, TopicColumn};
    pub use super::details::{ProfileDetails, TopicDetails};
    pub use super::dialogs::{ConfirmationDialog, ContactDialog};
    pub use super::models::{
        AppState, ColumnState, Profile, RemovalType, Topic, TopicCreationMode,
    };

    use dioxus::prelude::*;
    use dioxus_primitives::toast::ToastProvider;

    pub static DEFAULT_AVATAR: Asset = asset!("/assets/default_avatar.png");
    pub static CLOSE_ICON: Asset = asset!("/assets/close_icon.svg");
    pub static CLIP_ICON: Asset = asset!("/assets/clip_icon.svg");
    pub static COMPONENTS_CSS: Asset = asset!("/assets/dx-components-theme.css");

    #[component]
    pub fn Desktop(
        app_state: Signal<AppState>,
        on_create_topic: EventHandler<String>,
        on_join_topic: EventHandler<String>,
        on_leave_topic: EventHandler<String>,
        on_remove_contact: EventHandler<String>,
        on_send_message: EventHandler<(String, String)>,
        on_modify_topic: EventHandler<Topic>,
        on_modify_profile: EventHandler<Profile>,
        on_send_message_dm: EventHandler<(String, String)>,
        on_connect_peer: EventHandler<String>,
        on_add_contact: EventHandler<String>,
        on_image_send: EventHandler<(String, Vec<u8>)>,
    ) -> Element {
        let mut show_topic_dialog = use_signal(|| false);
        let mut show_contact_dialog = use_signal(|| false);
        let mut selected_topic_id = use_signal::<Option<String>>(|| None);
        let show_topic_details = use_signal::<Option<Topic>>(|| None);
        let mut show_profile_details = use_signal::<Option<Profile>>(|| None);
        let mut search_query = use_signal(String::new);
        let mut show_leave_confirmation =
            use_signal::<Option<(String, String, RemovalType)>>(|| None);
        let mut selected_column = use_signal::<ColumnState>(|| ColumnState::Contact);

        let profile_data: Profile = {
            let state = app_state();
            state.get_profile()
        };

        let avatar_url = if let Some(url) = &profile_data.avatar
            && !url.is_empty()
        {
            url.clone()
        } else {
            DEFAULT_AVATAR.to_string()
        };

        let profile_for_click = profile_data.clone();

        rsx! {
            link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
            link { rel: "stylesheet", href: COMPONENTS_CSS }
            div {
                class: "font-[Segoe_UI,Tahoma,Geneva,Verdana,sans-serif] m-0 bg-bg-dark text-text-primary flex flex-row h-screen overflow-hidden",
                oncontextmenu: move |e| {
                    e.prevent_default();
                },
                div { class: "flex flex-col h-full bg-bg-panel w-[clamp(280px,25%,400px)] transition-[width] duration-300 ease-in-out relative border-r border-border",
                    div { class: "bg-bg-panel py-5 px-3.75 shadow-md",
                        div { class: "flex justify-between items-center mb-5",
                            h2 { class: "text-text-primary pl-1.25 m-0 text-[clamp(1.2rem,4vw,1.6rem)] font-semibold",
                                "Messages"
                            }
                            button {
                                class: "w-10 h-10 text-3xl leading-none bg-bg-subtle text-text-primary border-none rounded-xl flex items-center justify-center cursor-pointer transition-all duration-300 ease-in-out shadow-sm hover:bg-bg-active hover:-translate-y-0.5 hover:shadow-md active:translate-y-0 active:shadow-sm",
                                title: if selected_column() == ColumnState::Topic { "New Topic" } else { "Add Contact" },
                                onclick: move |_| {
                                    if selected_column() == ColumnState::Topic {
                                        show_topic_dialog.set(true);
                                    } else {
                                        show_contact_dialog.set(true);
                                    }
                                },
                                "+"
                            }
                        }
                        input {
                            class: "input-field pl-11.25 bg-[url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='20' height='20' viewBox='0 0 24 24' fill='none' stroke='%23a1a1aa' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='11' cy='11' r='8'%3E%3C/circle%3E%3Cpath d='m21 21-4.35-4.35'%3E%3C/path%3E%3C/svg%3E\")] bg-no-repeat bg-position-[15px_center] bg-size-[18px] focus:-translate-y-px",
                            r#type: "text",
                            icon: "search",
                            placeholder: "Search...",
                            oninput: move |value| {
                                search_query.set(value.value());
                            },
                        }
                        div { class: "grid grid-cols-2 mt-4 p-1 bg-bg-subtle rounded-lg gap-1",
                            button {
                                class: if selected_column() == ColumnState::Contact { "selected-column-button" } else { "unselected-topic-button" },
                                title: "Contacts",
                                onclick: move |_| selected_column.set(ColumnState::Contact),
                                "Contacts"
                            }
                            button {
                                class: if selected_column() == ColumnState::Topic { "selected-column-button" } else { "unselected-topic-button" },
                                title: "Topics",
                                onclick: move |_| selected_column.set(ColumnState::Topic),
                                "Topics"
                            }
                        }
                    }
                    div { class: "flex-1 overflow-y-auto overflow-x-hidden min-h-0 scrollbar-custom",
                        if selected_column() == ColumnState::Topic {
                            TopicColumn {
                                search_query,
                                selected_topic_id,
                                show_topic_details,
                                show_leave_confirmation,
                                app_state,
                            }
                        } else {
                            ContactColumn {
                                search_query,
                                selected_topic_id,
                                show_profile_details,
                                show_leave_confirmation,
                                app_state,
                            }
                        }
                    }
                    div {
                        class: "bg-bg-panel py-3 px-3.75 border-t border-[#444444] flex items-center gap-3 cursor-pointer transition-colors duration-200 mt-auto shrink-0 hover:bg-bg-hover active:bg-bg-active group",
                        onclick: move |_| {
                            show_profile_details.set(Some(profile_for_click.clone()));
                        },
                        img {
                            class: "avatar w-11.25 h-11.25 transition-colors duration-200 group-hover:border-accent",
                            src: "{avatar_url}",
                            alt: "Profile Avatar",
                        }
                        div { class: "flex-1 flex flex-col gap-1 overflow-hidden",
                            h2 { class: "m-0 text-white text-base font-semibold whitespace-nowrap overflow-hidden text-ellipsis",
                                "{profile_data.name}"
                            }
                        }
                    }

                    if let Some(profile) = show_profile_details() {
                        ToastProvider {
                            ProfileDetails {
                                profile: profile.clone(),
                                toggle: show_profile_details,
                                on_modify_profile,
                                readonly: profile.id != profile_data.id,
                            }
                        }
                    }

                    if let Some(topic) = show_topic_details() {
                        ToastProvider {
                            TopicDetails {
                                topic: topic.clone(),
                                toggle: show_topic_details,
                                on_modify_topic,
                                view_profile: show_profile_details,
                                app_state,
                            }
                        }
                    }

                    if show_topic_dialog() {
                        TopicDialog {
                            toggle: show_topic_dialog,
                            on_create: on_create_topic,
                            on_join: on_join_topic,
                        }
                    }


                    if show_contact_dialog() {
                        ContactDialog {
                            toggle: show_contact_dialog,
                            on_add: on_add_contact,
                        }
                    }

                    if let Some((id, name, removal_type)) = show_leave_confirmation() {
                        {
                            let (title, message, confirm_text) = match removal_type {

                                RemovalType::Topic => {
                                    (
                                        "Leave Topic".to_string(),
                                        format!(
                                            "Are you sure you want to leave \"{}\"? You will no longer receive messages from this topic.",
                                            name,
                                        ),
                                        "Leave".to_string(),
                                    )
                                }
                                RemovalType::Contact => {
                                    (
                                        "Remove Contact".to_string(),
                                        format!(
                                            "Are you sure you want to remove \"{}\" from your contacts?",
                                            name,
                                        ),
                                        "Remove".to_string(),
                                    )
                                }
                            };
                            rsx! {
                                ConfirmationDialog {
                                    title,
                                    message,
                                    confirm_text,
                                    cancel_text: "Cancel".to_string(),
                                    is_danger: true,
                                    toggle: show_leave_confirmation,
                                    on_confirm: move |_| {
                                        match removal_type {
                                            RemovalType::Topic => on_leave_topic.call(id.clone()),
                                            RemovalType::Contact => on_remove_contact.call(id.clone()),
                                        }
                                        show_leave_confirmation.set(None);
                                        selected_topic_id.set(None);
                                    },
                                }
                            }
                        }
                    }
                }

                Chat {
                    app_state,
                    topic_id: selected_topic_id(),
                    on_send_message,
                    on_send_message_dm,
                    on_image_send,
                }
            }
        }
    }
}
