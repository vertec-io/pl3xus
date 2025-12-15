use leptos::prelude::*;
use leptos::ev::SubmitEvent;
use leptos_use::{
    core::ConnectionReadyState, use_websocket_with_options, UseWebSocketOptions,
    UseWebSocketReturn, DummyEncoder,
};
// Import reactive traits for .get(), .with(), .update() methods
use reactive_graph::traits::{Get, With, Update};

// Include the shared types from the examples directory
// This ensures type names match between client and server
#[path = "../../examples/shared_types.rs"]
mod shared_types;

// Use the official codec from pl3xus_common instead of a custom implementation
use pl3xus_common::codec::Pl3xusBincodeSingleMsgCodec;
use shared_types::{NewChatMessage, UserChatMessage};

fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);
    
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let chat_messages = RwSignal::new(Vec::<ChatMessage>::new());
    let (input_value, set_input_value) = signal(String::new());
    let (is_connected, set_is_connected) = signal(false);

    // WebSocket connection using Pl3xusBincodeCodec
    // Note: URL must be a static string, not read from a signal
    let UseWebSocketReturn {
        ready_state,
        message,
        send,
        open,
        close,
        ..
    } = use_websocket_with_options::<UserChatMessage, NewChatMessage, Pl3xusBincodeSingleMsgCodec, (), DummyEncoder>(
        "ws://127.0.0.1:8081",
        UseWebSocketOptions::default()
            .immediate(false)
            .on_open(move |_| {
                log::info!("WebSocket connected!");
                set_is_connected.set(true);
            })
            .on_close(move |_| {
                log::info!("WebSocket disconnected!");
                set_is_connected.set(false);
            })
            .on_error(move |e| {
                log::error!("WebSocket error: {:?}", e);
            }),
    );

    // Watch for connection state changes and update chat messages
    // This runs in a reactive context, so signal updates work properly
    Effect::new(move |prev_state: Option<ConnectionReadyState>| {
        let current_state = ready_state.get();

        // Only add messages when state actually changes
        if let Some(prev) = prev_state {
            if prev != current_state {
                match current_state {
                    ConnectionReadyState::Open => {
                        chat_messages.update(|msgs| {
                            msgs.push(ChatMessage::system("Connected to server!"));
                        });
                    }
                    ConnectionReadyState::Closed => {
                        chat_messages.update(|msgs| {
                            msgs.push(ChatMessage::system("Disconnected from server!"));
                        });
                    }
                    _ => {}
                }
            }
        }

        current_state
    });

    // Watch for incoming messages
    // The on_message callback runs in a non-reactive zone, so we need to use Effect
    // to properly trigger reactivity when messages arrive
    Effect::new(move |_| {
        message.with(|msg| {
            if let Some(msg) = msg {
                log::info!("Received message: {:?}", msg);
                chat_messages.update(|msgs| {
                    msgs.push(ChatMessage::user(&msg.name, &msg.message));
                });
            }
        });
    });

    let connect_disconnect = move |_| {
        if ready_state.get() == ConnectionReadyState::Open {
            close();
        } else {
            open();
        }
    };

    let send_message = move |ev: SubmitEvent| {
        ev.prevent_default();

        let msg = input_value.get();
        if msg.trim().is_empty() {
            return;
        }

        let current_state = ready_state.get();
        log::info!("Current ready_state: {:?}", current_state);

        if current_state == ConnectionReadyState::Open {
            let user_msg = UserChatMessage {
                message: msg.clone(),
            };
            log::info!("Sending message: {:?}", user_msg);
            send(&user_msg);
            log::info!("Message sent via send() function!");

            // Add to local chat optimistically
            chat_messages.update(|msgs| {
                msgs.push(ChatMessage::user("You", &msg));
            });

            set_input_value.set(String::new());
        } else {
            log::warn!("Attempted to send message while not connected! State: {:?}", current_state);
            chat_messages.update(|msgs| {
                msgs.push(ChatMessage::system(&format!("Not connected to server! State: {:?}", current_state)));
            });
        }
    };

    let connection_status = move || match ready_state.get() {
        ConnectionReadyState::Connecting => "Connecting...",
        ConnectionReadyState::Open => "Connected",
        ConnectionReadyState::Closing => "Closing...",
        ConnectionReadyState::Closed => "Disconnected",
    };

    let connection_button_text = move || {
        if ready_state.get() == ConnectionReadyState::Open {
            "Disconnect"
        } else {
            "Connect"
        }
    };

    view! {
        <div class="app-container">
            <h1>"Leptos WebSocket Chat Client"</h1>
            <p class="subtitle">"Using Pl3xusBincodeCodec with pl3xus server"</p>
            <p class="subtitle">"Connected to: ws://127.0.0.1:8081"</p>

            <div class="connection-panel">
                <div class="connection-row">
                    <button on:click=connect_disconnect>
                        {connection_button_text}
                    </button>
                    <span class="status-label">"Status:"</span>
                    <span class="status-value" class:connected=is_connected>
                        {connection_status}
                    </span>
                </div>
            </div>

            <div class="chat-container">
                <div class="chat-messages">
                    {move || {
                        chat_messages.with(|msgs| {
                            msgs.iter().map(|msg| {
                                let is_system = msg.is_system;
                                let author = msg.author.clone();
                                let text = msg.text.clone();
                                view! {
                                    <div class="chat-message" class:system-message=is_system>
                                        {if is_system {
                                            view! { <span class="message-text">{text}</span> }.into_any()
                                        } else {
                                            view! {
                                                <span class="message-author">{author + ":"}</span>
                                                <span class="message-text">{text}</span>
                                            }.into_any()
                                        }}
                                    </div>
                                }
                            }).collect_view()
                        })
                    }}
                </div>

                <form class="chat-input-form" on:submit=send_message>
                    <input
                        type="text"
                        placeholder="Type a message..."
                        prop:value=input_value
                        on:input=move |ev| set_input_value.set(event_target_value(&ev))
                        disabled=move || ready_state.get() != ConnectionReadyState::Open
                    />
                    <button
                        type="submit"
                        disabled=move || ready_state.get() != ConnectionReadyState::Open
                    >
                        "Send"
                    </button>
                </form>
            </div>
        </div>
    }
}

#[derive(Clone, Debug)]
struct ChatMessage {
    author: String,
    text: String,
    is_system: bool,
}

impl ChatMessage {
    fn user(author: &str, text: &str) -> Self {
        Self {
            author: author.to_string(),
            text: text.to_string(),
            is_system: false,
        }
    }

    fn system(text: &str) -> Self {
        Self {
            author: String::new(),
            text: text.to_string(),
            is_system: true,
        }
    }
}

