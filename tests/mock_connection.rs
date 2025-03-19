use chrono::Utc;
use oshatori::{
    connection::{ChatEvent, ConnectionEvent},
    Connection, Message, MessageFragment, MessageStatus, MessageType, MockConnection,
};

#[tokio::test]
async fn test_mock_connection_integration() {
    let mut conn = MockConnection::new();
    let mut rx = conn.subscribe();

    let test_message = Message {
        id: "random".to_string(),
        sender: None,
        content: vec![MessageFragment::Text("some text".to_string())],
        timestamp: Utc::now(),
        message_type: MessageType::Normal,
        status: MessageStatus::Sent,
    };

    conn.send(ConnectionEvent::Chat {
        event: ChatEvent::New {
            channel_id: "random".to_string(),
            message: test_message.clone(),
        },
    })
    .await
    .expect("failed to send");

    let received = rx.recv().await.expect("failed to receive");

    if let ConnectionEvent::Chat { event } = received {
        if let ChatEvent::New {
            channel_id,
            message,
        } = event
        {
            assert_eq!(channel_id, "random");
            match message.content.get(0) {
                Some(fragment) => match fragment {
                    MessageFragment::Text(value) => {
                        assert_eq!(value.to_owned(), "some text".to_string())
                    }
                    _ => {}
                },
                None => {}
            }
        } else {
            panic!("unexpected chat event");
        }
    } else {
        panic!("unexpected connection event");
    }
}
