#![allow(clippy::type_complexity)]

use async_net::{Ipv4Addr, SocketAddr};
use bevy::{
    color::palettes,
    prelude::*,
    tasks::{TaskPool, TaskPoolBuilder},
    ui::Interaction,
};
use pl3xus::{ConnectionId, Pl3xusRuntime, Network, NetworkData, NetworkEvent};
use pl3xus_common::Pl3xusMessage;
use std::net::IpAddr;

use pl3xus::tcp::{NetworkSettings, TcpProvider};
use examples_common as shared;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);

    // You need to add the `ClientPlugin` first before you can register
    // `ClientMessage`s
    app.add_plugins(pl3xus::Pl3xusPlugin::<
        TcpProvider,
        bevy::tasks::TaskPool,
    >::default());

    // Make sure you insert the Pl3xusRuntime resource with your chosen Runtime
    app.insert_resource(Pl3xusRuntime(
        TaskPoolBuilder::new().num_threads(2).build(),
    ));

    // A good way to ensure that you are not forgetting to register
    // any messages is to register them where they are defined!
    shared::client_register_network_messages(&mut app);

    app.add_systems(Startup, setup_ui);

    app.add_systems(
        Update,
        (
            handle_connect_button,
            handle_message_button,
            handle_outbound_button,
            handle_incoming_messages,
            handle_network_events,
        ),
    );

    // We have to insert the TCP [`NetworkSettings`] with our chosen settings.
    app.insert_resource(NetworkSettings::default());

    app.init_resource::<GlobalChatSettings>();
    app.init_resource::<ServerConnection>();

    // Set clear color to help debug rendering
    app.insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.1)));

    app.add_systems(PostUpdate, handle_chat_area);

    app.run();
}

#[derive(Resource)]
#[allow(dead_code)]
struct NetworkTaskPool(TaskPool);

///////////////////////////////////////////////////////////////
////////////// Incoming Message Handler ///////////////////////
///////////////////////////////////////////////////////////////

fn handle_incoming_messages(
    mut messages: Query<&mut AppChatMessages>,
    mut new_messages: MessageReader<NetworkData<shared::NewChatMessage>>,
) {
    let Ok(mut messages) = messages.single_mut() else {
        return;
    };

    for new_message in new_messages.read() {
        messages.add(UserMessage::new(&new_message.name, &new_message.message));
    }
}

fn handle_network_events(
    mut new_network_events: MessageReader<NetworkEvent>,
    connect_query: Query<&Children, With<ConnectButton>>,
    mut text_query: Query<&mut Text>,
    mut messages: Query<&mut AppChatMessages>,
    mut server_connection: ResMut<ServerConnection>,
) {
    let Ok(connect_children) = connect_query.single() else {
        return;
    };
    let mut text = text_query.get_mut(connect_children[0]).unwrap();
    let Ok(mut messages) = messages.single_mut() else {
        return;
    };

    for event in new_network_events.read() {
        info!("Received event");
        match event {
            NetworkEvent::Connected(conn_id) => {
                server_connection.connection_id = Some(*conn_id);
                messages.add(SystemMessage::new(format!(
                    "Successfully connected to server! Connection ID: {}",
                    conn_id.id
                )));
                text.0 = String::from("Disconnect");
            }

            NetworkEvent::Disconnected(_) => {
                server_connection.connection_id = None;
                messages.add(SystemMessage::new("Disconnected from server!".to_string()));
                text.0 = String::from("Connect to server");
            }
            NetworkEvent::Error(err) => {
                messages.add(UserMessage::new(String::from("SYSTEM"), err.to_string()));
            }
        }
    }
}

///////////////////////////////////////////////////////////////
////////////// Data Definitions ///////////////////////////////
///////////////////////////////////////////////////////////////

#[derive(Resource, Default)]
struct ServerConnection {
    connection_id: Option<ConnectionId>,
}

#[derive(Resource)]
struct GlobalChatSettings {
    chat_style: (TextFont, TextColor),
    author_style: (TextFont, TextColor),
}

impl FromWorld for GlobalChatSettings {
    fn from_world(_world: &mut World) -> Self {
        GlobalChatSettings {
            chat_style: (
                TextFont::from_font_size(20.0),
                TextColor::from(Color::BLACK),
            ),
            author_style: (
                TextFont::from_font_size(20.0),
                TextColor::from(palettes::css::RED),
            ),
        }
    }
}

enum ChatMessage {
    SystemMessage(SystemMessage),
    UserMessage(UserMessage),
}

impl ChatMessage {
    fn get_author(&self) -> String {
        match self {
            ChatMessage::SystemMessage(_) => "SYSTEM".to_string(),
            ChatMessage::UserMessage(UserMessage { user, .. }) => user.clone(),
        }
    }

    fn get_text(&self) -> String {
        match self {
            ChatMessage::SystemMessage(SystemMessage(msg)) => msg.clone(),
            ChatMessage::UserMessage(UserMessage { message, .. }) => message.clone(),
        }
    }
}

impl From<SystemMessage> for ChatMessage {
    fn from(other: SystemMessage) -> ChatMessage {
        ChatMessage::SystemMessage(other)
    }
}

impl From<UserMessage> for ChatMessage {
    fn from(other: UserMessage) -> ChatMessage {
        ChatMessage::UserMessage(other)
    }
}

struct SystemMessage(String);

impl SystemMessage {
    fn new<T: Into<String>>(msg: T) -> SystemMessage {
        Self(msg.into())
    }
}

#[derive(Component)]
struct UserMessage {
    user: String,
    message: String,
}

impl UserMessage {
    fn new<U: Into<String>, M: Into<String>>(user: U, message: M) -> Self {
        UserMessage {
            user: user.into(),
            message: message.into(),
        }
    }
}

#[derive(Component)]
struct ChatMessages<T> {
    messages: Vec<T>,
}

impl<T> ChatMessages<T> {
    fn new() -> Self {
        ChatMessages { messages: vec![] }
    }

    fn add<K: Into<T>>(&mut self, msg: K) {
        let msg = msg.into();
        self.messages.push(msg);
    }
}

type AppChatMessages = ChatMessages<ChatMessage>;

///////////////////////////////////////////////////////////////
////////////// UI Definitions/Handlers ////////////////////////
///////////////////////////////////////////////////////////////

#[derive(Component)]
struct ConnectButton;

fn handle_connect_button(
    net: ResMut<Network<TcpProvider>>,
    settings: Res<NetworkSettings>,
    interaction_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<ConnectButton>),
    >,
    mut text_query: Query<&mut Text>,
    mut messages: Query<&mut AppChatMessages>,
    task_pool: Res<Pl3xusRuntime<TaskPool>>,
    mut server_connection: ResMut<ServerConnection>,
) {
    let Ok(mut messages) = messages.single_mut() else {
        return;
    };

    for (interaction, children) in interaction_query.iter() {
        let mut text = text_query.get_mut(children[0]).unwrap();
        if let Interaction::Pressed = interaction {
            if let Some(conn_id) = server_connection.connection_id {
                match net.disconnect(conn_id) {
                    Ok(()) => {
                        server_connection.connection_id = None;
                        messages.add(SystemMessage::new("Disconnecting...".to_string()));
                        text.0 = String::from("Connect to server");
                    }
                    Err(err) => {
                        messages.add(SystemMessage::new(format!(
                            "Couldn't disconnect: {}",
                            err
                        )));
                    }
                }
            } else {
                text.0 = String::from("Connecting...");
                messages.add(SystemMessage::new("Connecting to server..."));

                net.connect(
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3030),
                    &task_pool.0,
                    &settings,
                );
            }
        }
    }
}

#[derive(Component)]
struct MessageButton;

#[derive(Component)]
struct OutboundButton;

fn handle_message_button(
    net: Res<Network<TcpProvider>>,
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<MessageButton>)>,
    mut messages: Query<&mut AppChatMessages>,
    server_connection: Res<ServerConnection>,
) {
    let Ok(mut messages) = messages.single_mut() else {
        return;
    };

    for interaction in interaction_query.iter() {
        if let Interaction::Pressed = interaction {
            let Some(conn_id) = server_connection.connection_id else {
                messages.add(SystemMessage::new("Not connected to server!".to_string()));
                return;
            };

            match net.send(
                conn_id,
                shared::UserChatMessage {
                    message: String::from("Hello there!"),
                },
            ) {
                Ok(()) => {
                    messages.add(SystemMessage::new("Message sent via net.send()!".to_string()));
                }
                Err(err) => messages.add(SystemMessage::new(format!(
                    "Could not send message: {}",
                    err
                ))),
            }
        }
    }
}

fn handle_outbound_button(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<OutboundButton>)>,
    mut messages: Query<&mut AppChatMessages>,
    server_connection: Res<ServerConnection>,
    mut outbound_writer: MessageWriter<pl3xus::OutboundMessage<shared::OutboundTestMessage>>,
) {
    let Ok(mut messages) = messages.single_mut() else {
        return;
    };

    for interaction in interaction_query.iter() {
        if let Interaction::Pressed = interaction {
            let Some(conn_id) = server_connection.connection_id else {
                messages.add(SystemMessage::new("Not connected to server!".to_string()));
                return;
            };

            // Send message using OutboundMessage MessageWriter
            outbound_writer.write(
                pl3xus::OutboundMessage::new(
                    shared::OutboundTestMessage::type_name().to_string(),
                    shared::OutboundTestMessage {
                        content: String::from("Hello via OutboundMessage!"),
                    },
                )
                .for_client(conn_id),
            );

            messages.add(SystemMessage::new(
                "Message sent via OutboundMessage MessageWriter!".to_string(),
            ));
        }
    }
}

#[derive(Component)]
struct ChatArea;

fn handle_chat_area(
    chat_settings: Res<GlobalChatSettings>,
    messages: Query<&AppChatMessages, Changed<AppChatMessages>>,
    chat_area_query: Query<Entity, With<ChatArea>>,
    mut read_messages_index: Local<usize>,
    mut commands: Commands,
) {
    let Ok(messages) = messages.single() else {
        return;
    };
    let Ok(chat_area_entity) = chat_area_query.single() else {
        warn!("handle_chat_area: Could not find chat area entity!");
        return;
    };

    info!("handle_chat_area: Processing {} messages (read_index: {})",
          messages.messages.len(), *read_messages_index);

    for message_index in *read_messages_index..messages.messages.len() {
        let message = &messages.messages[message_index];
        let new_message = commands
            .spawn((
                Text::new(format!("{}:", message.get_author())),
                chat_settings.author_style.clone(),
            ))
            .with_child((
                TextSpan::new(format!(" {}", message.get_text())),
                chat_settings.chat_style.clone(),
            ))
            .id();
        commands.entity(chat_area_entity).add_children(&[new_message]);
        info!("Added chat message: {:?}", new_message);
    }

    *read_messages_index = messages.messages.len();
}

fn setup_ui(mut commands: Commands) {
    info!("=== SETUP_UI CALLED ===");

    commands.spawn(Camera2d);
    info!("Spawned Camera2d");

    commands.spawn((AppChatMessages::new(),));
    info!("Spawned AppChatMessages");

    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Into::<BackgroundColor>::into(Color::WHITE),
        ))
        .id();
    info!("Spawned root UI node: {:?}", root);

    commands.entity(root).with_children(|parent| {
        let chat_area = parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(90.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                ChatArea,
            ))
            .id();
        info!("Spawned chat area: {:?}", chat_area);

        parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(10.0),
                    ..default()
                },
                Into::<BackgroundColor>::into(palettes::css::GRAY),
            ))
            .with_children(|parent_button_bar| {
                info!("Creating buttons in button bar");
            let btn1 = parent_button_bar
                .spawn((
                    Button,
                    Node {
                        width: Val::Percent(33.33),
                        height: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.5, 0.8)),
                    MessageButton,
                ))
                .with_child((
                    Text::new("Send Message!"),
                    TextFont {
                        font_size: 30.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.0, 0.0, 0.0)),
                ))
                .id();
            info!("Spawned MessageButton: {:?}", btn1);

            let btn2 = parent_button_bar
                .spawn((
                    Button,
                    Node {
                        width: Val::Percent(33.33),
                        height: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.8, 0.5, 0.2)),
                    OutboundButton,
                ))
                .with_child((
                    Text::new("Send Outbound!"),
                    TextFont {
                        font_size: 30.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.0, 0.0, 0.0)),
                ))
                .id();
            info!("Spawned OutboundButton: {:?}", btn2);

            let btn3 = parent_button_bar
                .spawn((
                    Button,
                    Node {
                        width: Val::Percent(33.33),
                        height: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.8, 0.3)),
                    ConnectButton,
                ))
                .with_child((
                    Text::new("Connect to server"),
                    TextFont {
                        font_size: 30.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.0, 0.0, 0.0)),
                ))
                .id();
            info!("Spawned ConnectButton: {:?}", btn3);
        });
    });

    info!("=== SETUP_UI COMPLETE ===");
}
