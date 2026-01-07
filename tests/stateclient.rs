#![cfg(feature = "mock")]

use chrono::Utc;
use oshatori::{
    client::{ConnectionStatus, StateClient},
    connection::{
        ChannelEvent, ChatEvent, ConnectionEvent, MockConnection, StatusEvent, UserEvent,
    },
    Channel, ChannelType, Connection, Message, MessageFragment, MessageStatus, MessageType,
    Profile,
};

#[tokio::test]
async fn stateclient_basic() {
    let client = StateClient::new();
    let conn_id = client.track("mock").await;

    assert!(client.get_connection(&conn_id).await.is_some());
    assert_eq!(client.list_connections().await.len(), 1);

    client.untrack(&conn_id).await;
    assert!(client.get_connection(&conn_id).await.is_none());
}

#[tokio::test]
async fn stateclient_status_events() {
    let client = StateClient::new();
    let conn_id = client.track("mock").await;

    client
        .process(
            &conn_id,
            ConnectionEvent::Status {
                event: StatusEvent::Connected { artifact: None },
            },
        )
        .await;

    let state = client.get_connection(&conn_id).await.unwrap();
    assert_eq!(state.status, ConnectionStatus::Connected);

    client
        .process(
            &conn_id,
            ConnectionEvent::Status {
                event: StatusEvent::Disconnected { artifact: None },
            },
        )
        .await;

    let state = client.get_connection(&conn_id).await.unwrap();
    assert_eq!(state.status, ConnectionStatus::Disconnected);
}

#[tokio::test]
async fn stateclient_channel_events() {
    let client = StateClient::new();
    let conn_id = client.track("mock").await;

    client
        .process(
            &conn_id,
            ConnectionEvent::Channel {
                event: ChannelEvent::New {
                    channel: Channel {
                        id: "general".to_string(),
                        name: Some("General".to_string()),
                        channel_type: ChannelType::Group,
                    },
                },
            },
        )
        .await;

    let channel = client.get_channel(&conn_id, "general").await;
    assert!(channel.is_some());
    assert_eq!(channel.unwrap().channel.name, Some("General".to_string()));

    client
        .process(
            &conn_id,
            ConnectionEvent::Channel {
                event: ChannelEvent::Switch {
                    channel_id: "general".to_string(),
                },
            },
        )
        .await;

    let state = client.get_connection(&conn_id).await.unwrap();
    assert_eq!(state.current_channel, Some("general".to_string()));
}

#[tokio::test]
async fn stateclient_user_events() {
    let client = StateClient::new();
    let conn_id = client.track("mock").await;

    client
        .process(
            &conn_id,
            ConnectionEvent::Channel {
                event: ChannelEvent::New {
                    channel: Channel {
                        id: "general".to_string(),
                        name: None,
                        channel_type: ChannelType::Group,
                    },
                },
            },
        )
        .await;

    client
        .process(
            &conn_id,
            ConnectionEvent::User {
                event: UserEvent::New {
                    channel_id: Some("general".to_string()),
                    user: Profile {
                        id: Some("user1".to_string()),
                        username: Some("testuser".to_string()),
                        display_name: None,
                        color: None,
                        picture: None,
                    },
                },
            },
        )
        .await;

    let user = client.get_user(&conn_id, "user1").await;
    assert!(user.is_some());
    assert_eq!(user.unwrap().username, Some("testuser".to_string()));

    let channel = client.get_channel(&conn_id, "general").await.unwrap();
    assert_eq!(channel.users.len(), 1);
}

#[tokio::test]
async fn stateclient_chat_events() {
    let client = StateClient::new();
    let conn_id = client.track("mock").await;

    client
        .process(
            &conn_id,
            ConnectionEvent::Channel {
                event: ChannelEvent::New {
                    channel: Channel {
                        id: "general".to_string(),
                        name: None,
                        channel_type: ChannelType::Group,
                    },
                },
            },
        )
        .await;

    let message = Message {
        id: Some("msg1".to_string()),
        sender_id: Some("user1".to_string()),
        content: vec![MessageFragment::Text("test".to_string())],
        timestamp: Utc::now(),
        message_type: MessageType::Normal,
        status: MessageStatus::Sent,
    };

    client
        .process(
            &conn_id,
            ConnectionEvent::Chat {
                event: ChatEvent::New {
                    channel_id: Some("general".to_string()),
                    message: message,
                },
            },
        )
        .await;

    let messages = client.get_messages(&conn_id, "general").await;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, Some("msg1".to_string()));

    client
        .process(
            &conn_id,
            ConnectionEvent::Chat {
                event: ChatEvent::Remove {
                    channel_id: Some("general".to_string()),
                    message_id: "msg1".to_string(),
                },
            },
        )
        .await;

    let messages = client.get_messages(&conn_id, "general").await;
    assert_eq!(messages.len(), 0);
}

#[tokio::test]
async fn stateclient_with_mock_connection() {
    let client = StateClient::new();
    let mut conn = MockConnection::new();
    let rx = conn.subscribe();

    let conn_id = client.track("mock").await;
    let handle = client.spawn_processor(conn_id.clone(), rx);

    conn.send(ConnectionEvent::Status {
        event: StatusEvent::Connected { artifact: None },
    })
    .await
    .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let state = client.get_connection(&conn_id).await.unwrap();
    assert_eq!(state.status, ConnectionStatus::Connected);

    handle.abort();
}
