use oshatori::connection::{Connection, MockConnection};
use serde_json::json;

#[tokio::test]
async fn test_mock_connection_integration() {
    let mut conn = MockConnection::new();
    assert!(conn.connect(&json!({"username": "test"})).await.is_ok());
    dbg!(conn.receive("channel_id").await.unwrap());
    let channels = conn.list_channels().await.unwrap();
    dbg!(conn);
    assert_eq!(channels.len(), 1);
}
