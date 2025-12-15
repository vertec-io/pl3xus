use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;
use pl3xus::{
    AppNetworkMessage, Pl3xusPlugin, Pl3xusRuntime, Network,
    ConnectionId, SubscriptionMessage,
    tcp::{TcpProvider, NetworkSettings},
};
use pl3xus_common::SubscribeById;
use serde::{Deserialize, Serialize};

// Test message type
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct TestMessage {
    content: String,
}

// Another test message type
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct AnotherMessage {
    content: String,
}

// Helper function to create a test app with minimal setup
fn create_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(Pl3xusPlugin::<TcpProvider, bevy::tasks::TaskPool>::default());
    app.insert_resource(Pl3xusRuntime(TaskPoolBuilder::new().num_threads(2).build()));
    app.insert_resource(NetworkSettings::default());
    app
}

#[test]
fn test_register_message() {
    let mut app = create_test_app();

    // Register message using Pl3xusMessage
    app.register_network_message::<TestMessage, TcpProvider>();

    // Verify registration with auto-generated name
    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();
    let names = net.registered_message_names();
    assert!(names.iter().any(|name| name.contains("TestMessage")));
}

#[test]
fn test_register_another_message() {
    let mut app = create_test_app();

    // Register another message using Pl3xusMessage
    app.register_network_message::<AnotherMessage, TcpProvider>();

    // Verify registration with auto-generated name
    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();
    let names = net.registered_message_names();
    assert!(names.iter().any(|name| name.contains("AnotherMessage")));
}

#[test]
fn test_is_message_registered() {
    let mut app = create_test_app();

    app.register_network_message::<TestMessage, TcpProvider>();

    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();
    let names = net.registered_message_names();

    // Find the auto-generated name
    let auto_name = names.iter().find(|name| name.contains("TestMessage")).unwrap();

    // Verify is_message_registered works with auto-generated name
    assert!(net.is_message_registered(auto_name));
}

#[test]
fn test_multiple_registration() {
    let mut app = create_test_app();

    // Register multiple messages
    app.register_network_message::<TestMessage, TcpProvider>();
    app.register_network_message::<AnotherMessage, TcpProvider>();

    // Both should be registered
    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();
    let names = net.registered_message_names();
    assert!(names.iter().any(|name| name.contains("TestMessage")));
    assert!(names.iter().any(|name| name.contains("AnotherMessage")));
}

#[test]
#[should_panic(expected = "Duplicate registration")]
fn test_duplicate_registration_panics() {
    let mut app = create_test_app();

    app.register_network_message::<TestMessage, TcpProvider>();
    app.register_network_message::<TestMessage, TcpProvider>(); // Should panic
}

#[test]
fn test_send_message() {
    let mut app = create_test_app();

    app.register_network_message::<TestMessage, TcpProvider>();

    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();

    // Test that send method exists and compiles
    // (We can't actually send without a connection, but we can verify the API works)
    let msg = TestMessage { content: "test".to_string() };
    let result = net.send(ConnectionId { id: 999 }, msg);

    // Should fail because connection doesn't exist, but that's expected
    assert!(result.is_err());
}

#[test]
fn test_broadcast_message() {
    let mut app = create_test_app();

    app.register_network_message::<TestMessage, TcpProvider>();

    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();

    // Test that broadcast method works
    let msg = TestMessage { content: "test".to_string() };
    net.broadcast(msg);

    // No connections, so nothing happens, but API works
}

#[test]
fn test_external_type_registration() {
    // Test that we can register types from external crates
    // (simulated by using a type without NetworkMessage impl)

    #[derive(Serialize, Deserialize, Clone)]
    struct ExternalType {
        data: Vec<u8>,
    }

    let mut app = create_test_app();

    // This works because Pl3xusMessage has a blanket impl
    app.register_network_message::<ExternalType, TcpProvider>();
    
    // Verify registration
    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();
    let names = net.registered_message_names();
    let has_external = names.iter().any(|name| name.contains("ExternalType"));
    assert!(has_external, "ExternalType should be registered");
}

#[test]
fn test_generic_type_registration() {
    #[derive(Serialize, Deserialize, Clone)]
    struct GenericMessage<T> {
        value: T,
    }

    let mut app = create_test_app();

    // Register different instantiations of the generic type
    app.register_network_message::<GenericMessage<i32>, TcpProvider>();
    app.register_network_message::<GenericMessage<String>, TcpProvider>();
    
    // Both should be registered with different names
    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();
    let names = net.registered_message_names();
    let registrations: Vec<_> = names.iter()
        .filter(|name| name.contains("GenericMessage"))
        .collect();

    assert_eq!(registrations.len(), 2, "Both generic instantiations should be registered");
}

// Subscription message using Pl3xusMessage
#[derive(SubscribeById, Serialize, Deserialize, Clone, Debug)]
struct SubscriptionMessage1 {
    data: String,
}

#[test]
fn test_subscription_registration() {
    let mut app = create_test_app();

    // Register subscription using Pl3xusMessage
    app.register_subscription::<SubscriptionMessage1, TcpProvider>();

    // Verify all three message types are registered
    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();
    let names = net.registered_message_names();

    // All three messages now use auto-generated names (Pl3xusMessage)
    let has_base = names.iter().any(|name| name.contains("SubscriptionMessage1") && !name.contains("Subscribe") && !name.contains("Unsubscribe"));
    assert!(has_base, "Base subscription message should be registered with auto-generated name");

    // Subscribe/Unsubscribe messages also use auto-generated names now
    let has_subscribe = names.iter().any(|name| name.contains("Subscribe") && name.contains("SubscriptionMessage1"));
    assert!(has_subscribe, "Subscribe message should be registered with auto-generated name");

    let has_unsubscribe = names.iter().any(|name| name.contains("Unsubscribe") && name.contains("SubscriptionMessage1"));
    assert!(has_unsubscribe, "Unsubscribe message should be registered with auto-generated name");
}

#[test]
fn test_subscription_no_duplicate_registration() {
    let mut app = create_test_app();

    // Register subscription twice - should not panic because we check for duplicates
    app.register_subscription::<SubscriptionMessage1, TcpProvider>();
    app.register_subscription::<SubscriptionMessage1, TcpProvider>();

    // Verify registration still works
    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();
    let names = net.registered_message_names();
    let has_subscribe = names.iter().any(|name| name.contains("Subscribe") && name.contains("SubscriptionMessage1"));
    assert!(has_subscribe, "Subscribe message should be registered");
}

#[test]
fn test_targeted_message() {
    let mut app = create_test_app();

    // Register targeted message using Pl3xusMessage
    app.register_targeted_message::<TestMessage, TcpProvider>();

    // Verify targeted message is registered
    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();
    let names = net.registered_message_names();
    let has_targeted = names.iter().any(|name| name.contains("Targeted") && name.contains("TestMessage"));
    assert!(has_targeted, "Targeted message should be registered");
}

#[test]
fn test_targeted_message_no_duplicate_registration() {
    let mut app = create_test_app();

    // Register targeted message twice - should not panic
    app.register_targeted_message::<AnotherMessage, TcpProvider>();
    app.register_targeted_message::<AnotherMessage, TcpProvider>();

    // Verify registration still works
    let net = app.world().get_resource::<Network<TcpProvider>>().unwrap();
    let names = net.registered_message_names();
    let has_targeted = names.iter().any(|name| name.contains("Targeted") && name.contains("AnotherMessage"));
    assert!(has_targeted);
}

