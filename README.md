<h1 align="center"> 🐦 oshatori</h1>

> oshatori is a multi-protocol simultaneous chat library for Rust

> [!WARNING]
> This library is in its early stages and the core structure
> will evolve as more protocols are implemented. If you're
> still considering contributing at this point in time, make
> sure to look out for a more stable release of oshatori
> before sending a pull request.

## Introduction

This project aims to generalize a broad range of text chat and social interactions. To start off let's go over the types provided by the library:

| Name                | Kind     | Fields / Variants                                                                                                                                                                                        | Description                                                                                                                           |
|---------------------|----------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------|
| **Account**         | `struct` | **auth:** `Vec<AuthField>`<br>**protocol\_name:** `String`<br>**private\_profile:** `Option<Profile>`                                                                                                    | Represents a user’s account on a protocol, with auth fields and an optional private profile.                                          |
| **Profile**         | `struct` | **id:** `Option<String>`<br>**username:** `Option<String>`<br>**display\_name:** `Option<String>`<br>**color:** `Option<[u8;4]>`<br>**picture:** `Option<String>`                                        | Holds display info for a user (defaults all to `None`).                                                                               |
| **Message**         | `struct` | **id:** `Option<String>`<br>**sender\_id:** `Option<String>`<br>**content:** `Vec<MessageFragment>`<br>**timestamp:** `DateTime<Utc>`<br>**message\_type:** `MessageType`<br>**status:** `MessageStatus` | Encapsulates a single chat message with fragments, timestamp, type, and delivery status.                                              |
| **MessageStatus**   | `enum`   | `Sent`<br>`Delivered`<br>`Edited`<br>`Deleted`<br>`Failed`                                                                                                                                               | Tracks the state of a message.                                                                                                        |
| **MessageType**     | `enum`   | `CurrentUser`<br>`Normal`<br>`Server`<br>`Meta`                                                                                                                                                          | Categorizes if a message was sent by the current user, another user, the server, or internally by the protocol implementation itself. |
| **MessageFragment** | `enum`   | `Text(String)`<br>`Image { url: String, mime: String }`<br>`Video { url: String, mime: String }`<br>`Audio { url: String, mime: String }`<br>`Url(String)`                                               | A piece of a message: plaintext, media embed, or URL.                                                                                 |
| **Channel**         | `struct` | **id:** `String`<br>**name:** `Option<String>`<br>**channel\_type:** `ChannelType`                                                                                                                       | Represents a chat channel (group, direct, or broadcast).                                                                              |
| **ChannelType**     | `enum`   | `Group`<br>`Direct`<br>`Broadcast`                                                                                                                                                                       | Defines the type of channel (multi-user, peer-to-peer, or broadcast-only).                                                            |
| **Protocol**        | `struct` | **name:** `String`<br>**auth:** `Option<Vec<AuthField>>`                                                                                                                                                 | Describes a messaging protocol with its auth fields (or `None` if no authentication is needed.                                        |
| **AuthField**       | `struct` | **name:** `String`<br>**display:** `Option<String>`<br>**value:** `FieldValue`<br>**required:** `bool`                                                                                                   | One input field needed for authentication (e.g. username, password).                                                                  |
| **FieldValue**      | `enum`   | `Text(Option<String>)`<br>`Password(Option<String>)`<br>`Group(Vec<AuthField>)`                                                                                                                          | The type and current value of an `AuthField`: plain text, password, or nested group of fields.                                        |


Common interface trait called `Connection`:

```Rust
pub trait Connection: Send + Sync {
    async fn connect(&mut self, auth: Vec<AuthField>) -> Result<(), String>;
    async fn disconnect(&mut self) -> Result<(), String>;
    async fn send(&mut self, event: ConnectionEvent) -> Result<(), String>;
    fn subscribe(&self) -> broadcast::Receiver<ConnectionEvent>;
    fn protocol_spec() -> Protocol;
}
```

Every implemented protocol can be interacted with using this interface, and the same set of events:

| Type                              | Variant        | Fields                                                                     |
|-----------------------------------|----------------|----------------------------------------------------------------------------|
| **ChatEvent**                     | `New`          | `channel_id: Option<String>`, `message: Message`                           |
|                                   | `Update`       | `channel_id: Option<String>`, `message_id: String`, `new_message: Message` |
|                                   | `Remove`       | `channel_id: Option<String>`, `message_id: String`                         |
| **ChannelEvent**                  | `New`          | `channel: Channel`                                                         |
|                                   | `Update`       | `channel_id: String`, `new_channel: Channel`                               |
|                                   | `Remove`       | `channel_id: String`                                                       |
|                                   | `Join`         | `channel_id: String`                                                       |
|                                   | `Leave`        | `channel_id: String`                                                       |
|                                   | `Switch`       | `channel_id: String`                                                       |
|                                   | `Kick`         | `channel_id: Option<String>`, `reason: Option<String>`, `ban: bool`        |
|                                   | `Wipe`         | `channel_id: Option<String>`                                               |
|                                   | `ClearList`    | *(no fields)*                                                              |
| **UserEvent**                     | `New`          | `channel_id: Option<String>`, `user: Profile`                              |
|                                   | `Update`       | `channel_id: Option<String>`, `user_id: String`, `new_user: Profile`       |
|                                   | `Remove`       | `channel_id: Option<String>`, `user_id: String`                            |
|                                   | `ClearList`    | `channel_id: Option<String>`                                               |
| **StatusEvent**                   | `Ping`         | `artifact: Option<String>`                                                 |
|                                   | `Connected`    | `artifact: Option<String>`                                                 |
|                                   | `Disconnected` | `artifact: Option<String>`                                                 |
| **AssetEvent** (soon to be added) | `New`          | `channel_id: Option<String>`, `user: Profile`                              |
|                                   | `Update`       | `channel_id: Option<String>`, `user_id: String`, `new_user: Profile`       |
|                                   | `Remove`       | `channel_id: Option<String>`, `user_id: String`                            |
|                                   | `ClearList`    | `channel_id: Option<String>`                                               |
| **ConnectionEvent**               | `Chat`         | `event: ChatEvent`                                                         |
|                                   | `User`         | `event: UserEvent`                                                         |
|                                   | `Channel`      | `event: ChannelEvent`                                                      |
|                                   | `Status`       | `event: StatusEvent`                                                       |
|                                   | `Asset`        | `event: AssetEvent`                                                        |

## Example

Here is an example straight from the `mock_connection.rs` test:

```Rust
use chrono::Utc;
use oshatori::{
    connection::{ChatEvent, ConnectionEvent, MockConnection},
    Connection, Message, MessageFragment, MessageStatus, MessageType,
};

#[tokio::test]
async fn test_mock_connection_integration() {
    let mut conn = MockConnection::new();
    let mut rx = conn.subscribe();

    let test_message = Message {
        id: None,
        sender_id: None,
        content: vec![MessageFragment::Text("some text".to_string())],
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

    let received = rx.recv().await.expect("failed to receive");

    if let ConnectionEvent::Chat { event } = received {
        if let ChatEvent::New {
            channel_id,
            message,
        } = event
        {
            assert_eq!(channel_id, None);
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
```

## Coverage

Currently these protocols are implemented:

* sockchat - using [kanii-lib](https://github.com/saikuru0/kanii-lib)
* mock - a mock protocol for testing

## Styleguide

The folder structure is used as follows:

* `src`
  * `connection` - protocol implementations
    * `mod.rs` - Connection trait definition
    * `sockchat.rs`
    * `mock.rs`
  * `utils` - helper functions used by multiple protocols
    * `bbcode.rs` - bbcode parser
    * `color.rs` - kanii_to_rgba
    * `html.rs` - replacing `&lt;`, `&gt;`, and `\s<br/>\s` with <, >, and \n
    * `mod.rs`
  * `lib.rs` - type definitions
* `tests` - tests for each protocol
  * `mock_connection.rs`
  * `sockchat_connection`
