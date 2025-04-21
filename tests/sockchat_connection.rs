#![cfg(feature = "sockchat")]

use chrono::Utc;
use oshatori::{
    connection::{ChatEvent, ConnectionEvent, SockchatConnection},
    Connection, Message, MessageFragment, MessageStatus, MessageType,
};
use std::env;
use tokio::{sync::broadcast::error::RecvError, time::Duration};

#[tokio::test]
async fn test_sockchat_connection_integration() {
    let _ = dotenvy::dotenv();

    let mut conn = SockchatConnection::new();

    conn.connect(vec![
        oshatori::AuthField {
            name: "sockchat_url".to_string(),
            display: None,
            value: oshatori::FieldValue::Text(env::var("SOCKCHAT_URL").ok()),
            required: true,
        },
        oshatori::AuthField {
            name: "token".to_string(),
            display: None,
            value: oshatori::FieldValue::Password(env::var("SOCKCHAT_TOKEN").ok()),
            required: true,
        },
        oshatori::AuthField {
            name: "uid".to_string(),
            display: None,
            value: oshatori::FieldValue::Text(env::var("SOCKCHAT_UID").ok()),
            required: true,
        },
    ])
    .await
    .expect("failed to connect");

    let mut rx = conn.subscribe();

    tokio::time::sleep(Duration::from_millis(1200)).await;

    let test_message = Message {
        id: None,
        sender_id: None,
        content: vec![MessageFragment::Text("test".to_string())],
        timestamp: Utc::now(),
        message_type: MessageType::Normal,
        status: MessageStatus::Sent,
    };

    conn.send(ConnectionEvent::Chat {
        event: ChatEvent::New {
            channel_id: None,
            message: test_message.clone(),
        },
    })
    .await
    .expect("failed to send");

    tokio::time::sleep(Duration::from_millis(100)).await;

    for _ in 0..24 {
        let mut received = rx.recv().await;
        while let Err(RecvError::Lagged(_)) = received {
            received = rx.recv().await;
        }

        dbg!(received.unwrap());
    }

    conn.disconnect().await.expect("failed to disconnect");
}
