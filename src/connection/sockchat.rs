use crate::utils::bbcode::parse_bbcode;
use std::str::FromStr;

use crate::{
    AuthField, Channel, ChannelType, Connection, FieldValue, Message,
    MessageStatus, MessageType, Profile, Protocol,
};
use async_trait::async_trait;
use chrono::DateTime;
use futures_util::{SinkExt, StreamExt};
use kanii_lib::packets::{
    client::ClientPacket,
    server::{
        ChannelEventPacket, ChannelSwitchingPacket, ContextInformationPacket, JoinAuthPacket,
        ServerPacket,
    },
    types::Sockchatable,
};
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};
use url::Url;

use super::{ChannelEvent, ChatEvent, ConnectionEvent, StatusEvent, UserEvent};

#[derive(Clone, Debug)]
pub struct SockchatConnection {
    sender: Option<tokio::sync::mpsc::Sender<WsMessage>>,
    receiver: Option<broadcast::Sender<ConnectionEvent>>,
}

impl SockchatConnection {
    pub fn new() -> Self {
        SockchatConnection {
            sender: None,
            receiver: None,
        }
    }
}

unsafe impl Send for SockchatConnection {}
unsafe impl Sync for SockchatConnection {}

#[async_trait]
impl Connection for SockchatConnection {
    async fn connect(&mut self, auth: Vec<AuthField>) -> Result<(), String> {
        let mut url = None;
        let mut token = None;
        let mut uid = None;
        let mut pfp_url = None;

        for field in auth {
            match field.name.as_str() {
                "sockchat_url" => {
                    if let FieldValue::Text(Some(value)) = field.value {
                        url = Some(value);
                    }
                }
                "token" => {
                    if let FieldValue::Password(Some(value)) = field.value {
                        token = Some(value);
                    }
                }
                "uid" => {
                    if let FieldValue::Text(Some(value)) = field.value {
                        uid = Some(value);
                    }
                }
                "pfp_url" => {
                    if let FieldValue::Text(Some(value)) = field.value {
                        pfp_url = Some(value);
                    }
                }
                _ => {}
            }
        }

        let url = url.ok_or("Missing URL field")?;
        let token = token.ok_or("Missing Token field")?;
        let uid = uid.ok_or("Missing UID field")?;

        let url = Url::parse(&url).map_err(|e| e.to_string())?;
        let (ws_stream, _) = connect_async(url.to_string())
            .await
            .map_err(|e| e.to_string())?;
        let (mut write, mut read) = ws_stream.split();

        let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(127);
        let (event_tx, _) = broadcast::channel(127);

        self.sender = Some(tx);
        self.receiver = Some(event_tx.clone());

        let auth_packet = ClientPacket::Authentication(
            kanii_lib::packets::client::authentication::AuthenticationPacket {
                method: "Misuzu".to_string(),
                authkey: token,
            },
        );

        let _ = write.send(auth_packet.to_sockstr().into()).await;

        tokio::spawn(async move {
            let mut current_channel: Option<String> = None;
            while let Some(msg) = read.next().await {
                if let Ok(msg) = msg {
                    if let Ok(sockpacket) = ServerPacket::from_str(msg.to_string().as_str()) {
                        match sockpacket {
                            ServerPacket::Pong(packet) => {
                                let event = ConnectionEvent::Status {
                                    event: StatusEvent::Ping {
                                        artifact: Some(packet.text),
                                    },
                                };
                                let _ = event_tx.send(event);
                            }

                            ServerPacket::JoinAuth(packet) => match packet {
                                JoinAuthPacket::GoodAuth { .. } => {
                                    let event = ConnectionEvent::Status {
                                        event: StatusEvent::Connected { artifact: None },
                                    };
                                    let _ = event_tx.send(event);
                                }
                                JoinAuthPacket::BadAuth { reason, timestamp } => {
                                    let event = ConnectionEvent::Status {
                                        event: StatusEvent::Disconnected {
                                            artifact: Some(format!("{}: {}", timestamp, reason)),
                                        },
                                    };
                                    let _ = event_tx.send(event);
                                }
                                JoinAuthPacket::Join {
                                    timestamp: _,
                                    user_id,
                                    username,
                                    color: _,
                                    user_permissions: _,
                                    sequence_id: _,
                                } => {
                                    let mut pic = None;
                                    if let Some(pfp_format) = pfp_url.clone() {
                                        pic = Some(pfp_format.replace("{uid}", user_id.as_str()));
                                    }
                                    let event = ConnectionEvent::User {
                                        event: UserEvent::New {
                                            channel_id: current_channel.to_owned(),
                                            user: crate::Profile {
                                                id: Some(user_id),
                                                username: Some(username),
                                                display_name: None,
                                                color: None,
                                                picture: pic,
                                            },
                                        },
                                    };
                                    let _ = event_tx.send(event);
                                }
                            },

                            ServerPacket::ChatMessage(packet) => {
                                let event = ConnectionEvent::Chat {
                                    event: ChatEvent::New {
                                        channel_id: current_channel.clone(),
                                        message: Message {
                                            id: Some(packet.sequence_id),
                                            sender_id: Some(packet.user_id),
                                            content: parse_bbcode(packet.message.as_str()),
                                            timestamp: DateTime::from_timestamp_nanos(
                                                packet.timestamp,
                                            ),
                                            message_type: MessageType::Normal,
                                            status: MessageStatus::Delivered,
                                        },
                                    },
                                };
                                let _ = event_tx.send(event);
                            }

                            ServerPacket::UserDisconnect(packet) => {
                                let event = ConnectionEvent::User {
                                    event: UserEvent::Remove {
                                        user_id: packet.user_id,
                                    },
                                };
                                let _ = event_tx.send(event);
                            }

                            ServerPacket::ChannelEvent(packet) => match packet {
                                ChannelEventPacket::Creation {
                                    channel_name,
                                    is_protected: _,
                                    is_temporary: _,
                                } => {
                                    let event = ConnectionEvent::Channel {
                                        event: ChannelEvent::New {
                                            channel: Channel {
                                                id: channel_name,
                                                name: None,
                                                channel_type: ChannelType::Group,
                                            },
                                        },
                                    };
                                    let _ = event_tx.send(event);
                                }
                                ChannelEventPacket::Update {
                                    channel_name,
                                    new_name,
                                    is_protected: _,
                                    is_temporary: _,
                                } => {
                                    let event = ConnectionEvent::Channel {
                                        event: ChannelEvent::Update {
                                            channel_id: channel_name,
                                            new_channel: Channel {
                                                id: new_name,
                                                name: None,
                                                channel_type: ChannelType::Group,
                                            },
                                        },
                                    };
                                    let _ = event_tx.send(event);
                                }
                                ChannelEventPacket::Deletion { channel_name } => {
                                    let event = ConnectionEvent::Channel {
                                        event: ChannelEvent::Remove {
                                            channel_id: channel_name,
                                        },
                                    };
                                    let _ = event_tx.send(event);
                                }
                            },

                            ServerPacket::ChannelSwitching(packet) => match packet {
                                ChannelSwitchingPacket::Join {
                                    user_id,
                                    username,
                                    color: _,
                                    user_permissions: _,
                                    sequence_id: _,
                                } => {
                                    let event = ConnectionEvent::User {
                                        event: UserEvent::New {
                                            channel_id: current_channel.to_owned(),
                                            user: crate::Profile {
                                                id: Some(user_id),
                                                username: Some(username),
                                                display_name: None,
                                                color: None,
                                                picture: None,
                                            },
                                        },
                                    };
                                    let _ = event_tx.send(event);
                                }
                                ChannelSwitchingPacket::Departure {
                                    user_id,
                                    sequence_id: _,
                                } => {
                                    let event = ConnectionEvent::User {
                                        event: UserEvent::Remove { user_id },
                                    };
                                    let _ = event_tx.send(event);
                                }
                                ChannelSwitchingPacket::ForcedSwitch { channel_name } => {
                                    current_channel.replace(channel_name.to_owned());
                                    let event = ConnectionEvent::Channel {
                                        event: ChannelEvent::Switch {
                                            channel_id: channel_name,
                                        },
                                    };
                                    let _ = event_tx.send(event);
                                }
                            },

                            ServerPacket::MessageDeletion(packet) => {
                                let event = ConnectionEvent::Chat {
                                    event: ChatEvent::Remove {
                                        channel_id: current_channel.clone(),
                                        message_id: packet.sequence_id,
                                    },
                                };
                                let _ = event_tx.send(event);
                            }

                            ServerPacket::ContextInformation(packet) => match packet {
                                ContextInformationPacket::ExistingUsers { count: _, contexts } => {
                                    for context in contexts {
                                        let mut pic = None;
                                        if let Some(pfp_format) = pfp_url.clone() {
                                            pic = Some(
                                                pfp_format
                                                    .replace("{uid}", &context.user_id.as_str()),
                                            );
                                        }
                                        let event = ConnectionEvent::User {
                                            event: UserEvent::New {
                                                channel_id: current_channel.to_owned(),
                                                user: crate::Profile {
                                                    id: Some(context.user_id),
                                                    username: Some(context.username),
                                                    display_name: None,
                                                    color: None,
                                                    picture: pic,
                                                },
                                            },
                                        };
                                        let _ = event_tx.send(event);
                                    }
                                }
                                ContextInformationPacket::ExistingMessage {
                                    timestamp,
                                    user_id,
                                    username: _,
                                    color: _,
                                    user_permissions: _,
                                    message,
                                    sequence_id,
                                    notify: _,
                                    message_flags: _,
                                } => {
                                    let event = ConnectionEvent::Chat {
                                        event: ChatEvent::New {
                                            channel_id: current_channel.clone(),
                                            message: Message {
                                                id: Some(sequence_id),
                                                sender_id: Some(user_id),
                                                content: parse_bbcode(message.as_str()),
                                                timestamp: DateTime::from_timestamp_nanos(
                                                    timestamp,
                                                ),
                                                message_type: MessageType::Normal,
                                                status: MessageStatus::Delivered,
                                            },
                                        },
                                    };
                                    let _ = event_tx.send(event);
                                }
                                ContextInformationPacket::Channels { count: _, contexts } => {
                                    for context in contexts {
                                        let event = ConnectionEvent::Channel {
                                            event: ChannelEvent::New {
                                                channel: Channel {
                                                    id: context.channel_name,
                                                    name: None,
                                                    channel_type: ChannelType::Group,
                                                },
                                            },
                                        };
                                        let _ = event_tx.send(event);
                                    }
                                }
                            },

                            ServerPacket::ContextClearing(packet) => {
                                if packet.message_history {
                                    let event = ConnectionEvent::Channel {
                                        event: ChannelEvent::Wipe {
                                            channel_id: current_channel.clone(),
                                        },
                                    };
                                    let _ = event_tx.send(event);
                                }
                                if packet.user_list {
                                    let event = ConnectionEvent::User {
                                        event: UserEvent::ClearList {
                                            channel_id: current_channel.to_owned(),
                                        },
                                    };
                                    let _ = event_tx.send(event);
                                }
                                if packet.channel_list {
                                    let event = ConnectionEvent::Channel {
                                        event: ChannelEvent::ClearList,
                                    };
                                    let _ = event_tx.send(event);
                                }
                            }

                            ServerPacket::ForcedDisconnect(packet) => {
                                let event = ConnectionEvent::Channel {
                                    event: ChannelEvent::Kick {
                                        channel_id: current_channel.clone(),
                                        reason: None,
                                        ban: packet.ban,
                                    },
                                };
                                let _ = event_tx.send(event);
                            }

                            ServerPacket::UserUpdate(packet) => {
                                let mut pic = None;
                                if let Some(pfp_format) = pfp_url.clone() {
                                    pic =
                                        Some(pfp_format.replace("{uid}", &packet.user_id.as_str()));
                                }
                                let event = ConnectionEvent::User {
                                    event: UserEvent::Update {
                                        user_id: packet.user_id.to_owned(),
                                        new_user: Profile {
                                            id: Some(packet.user_id),
                                            username: Some(packet.username),
                                            display_name: None,
                                            color: None,
                                            picture: pic,
                                        },
                                    },
                                };
                                let _ = event_tx.send(event);
                            }
                        }
                    }
                }
            }
        });

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let packet =
                    ClientPacket::Message(kanii_lib::packets::client::message::MessagePacket {
                        user_id: uid.to_owned(),
                        message: msg.to_string(),
                    })
                    .to_sockstr();
                let _ = write.send(packet.into()).await;
            }
        });

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), String> {
        if let Some(sender) = &self.sender {
            let _ = sender.send(WsMessage::Close(None)).await;
        }
        self.sender = None;
        self.receiver = None;
        Ok(())
    }

    async fn send(&mut self, event: ConnectionEvent) -> Result<(), String> {
        if let Some(sender) = &self.sender {
            match event {
                ConnectionEvent::Chat {
                    event:
                        ChatEvent::New {
                            channel_id: _,
                            message,
                        },
                } => {
                    let text = if let Some(crate::MessageFragment::Text(content)) =
                        message.content.first()
                    {
                        content.clone()
                    } else {
                        return Err("Unsupported message format".to_string());
                    };

                    sender
                        .send(WsMessage::Text(text.into()))
                        .await
                        .map_err(|e| e.to_string())?
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<ConnectionEvent> {
        self.receiver.as_ref().unwrap().subscribe()
    }

    fn protocol_spec() -> Protocol {
        Protocol {
            name: "sockchat".to_string(),
            auth: Some(vec![
                AuthField {
                    name: "sockchat_url".to_string(),
                    display: Some("Sockchat URL".to_string()),
                    value: crate::FieldValue::Text(None),
                    required: true,
                },
                AuthField {
                    name: "token".to_string(),
                    display: Some("User token".to_string()),
                    value: crate::FieldValue::Password(None),
                    required: true,
                },
                AuthField {
                    name: "uid".to_string(),
                    display: Some("UID".to_string()),
                    value: crate::FieldValue::Text(None),
                    required: true,
                },
                AuthField {
                    name: "pfp_url".to_string(),
                    display: Some(
                        "Profile picture URL using {uid} to specify the user".to_string(),
                    ),
                    value: crate::FieldValue::Text(None),
                    required: false,
                },
            ]),
        }
    }
}
