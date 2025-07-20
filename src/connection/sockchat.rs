use std::str::FromStr;

use crate::{
    connection::{AssetEvent, ChannelEvent, ChatEvent, ConnectionEvent, StatusEvent, UserEvent},
    utils::{assets::parse_assets, bbcode::parse_bbcode, color::kanii_to_rgba, html::parse_html},
    Asset, AssetSource, AuthField, Channel, ChannelType, Connection, FieldValue, Message,
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
use tokio::sync::broadcast::{self, error::RecvError};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};
use url::Url;

#[derive(Clone, Debug)]
pub struct SockchatConnection {
    auth: Vec<AuthField>,
    ws_tx: broadcast::Sender<WsMessage>,
    event_tx: broadcast::Sender<ConnectionEvent>,
    assets: Vec<Asset>,
}

impl SockchatConnection {
    pub fn new() -> Self {
        let (ws_tx, _) = broadcast::channel::<WsMessage>(127);
        let (event_tx, _) = broadcast::channel(127);
        SockchatConnection {
            auth: vec![],
            ws_tx: ws_tx.clone(),
            event_tx,
            assets: Vec::new(),
        }
    }
}

unsafe impl Send for SockchatConnection {}
unsafe impl Sync for SockchatConnection {}

#[async_trait]
impl Connection for SockchatConnection {
    fn set_auth(&mut self, auth: Vec<AuthField>) -> Result<(), String> {
        self.auth = auth;
        Ok(())
    }

    async fn connect(&mut self) -> Result<(), String> {
        let mut url = None;
        let mut token = None;
        let mut uid = None;
        let mut pfp_url = None;
        let mut asset_api = None;

        for field in &self.auth {
            match field.name.as_str() {
                "sockchat_url" => {
                    if let FieldValue::Text(Some(value)) = field.value.clone() {
                        url = Some(value);
                    }
                }
                "token" => {
                    if let FieldValue::Password(Some(value)) = field.value.clone() {
                        token = Some(value);
                    }
                }
                "uid" => {
                    if let FieldValue::Text(Some(value)) = field.value.clone() {
                        uid = Some(value);
                    }
                }
                "pfp_url" => {
                    if let FieldValue::Text(Some(value)) = field.value.clone() {
                        pfp_url = Some(value);
                    }
                }
                "asset_api" => {
                    if let FieldValue::Text(Some(value)) = field.value.clone() {
                        asset_api = Some(value);
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

        let tx = self.ws_tx.clone();
        let mut rx = tx.subscribe();
        let event_tx = self.event_tx.clone();

        if let Some(mut api) = asset_api {
            if api.ends_with('/') {
                api.pop();
            }
            match reqwest::Client::new()
                .get(format!("{}/{}", api, "emotes"))
                .query(&[("fields", "uri,strings,min_rank")])
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.text().await {
                            Ok(text) => {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if let Some(emotes) = json.as_array() {
                                        for emote in emotes {
                                            if let (Some(uri), Some(strings)) =
                                                (emote.get("uri"), emote.get("strings"))
                                            {
                                                if let (Some(uri_str), Some(strings_array)) =
                                                    (uri.as_str(), strings.as_array())
                                                {
                                                    let keys: Vec<String> = strings_array
                                                        .iter()
                                                        .filter_map(|s| {
                                                            s.as_str().map(|s| s.to_string())
                                                        })
                                                        .collect();

                                                    if !keys.is_empty() {
                                                        let escaped_keys: Vec<String> = keys
                                                            .iter()
                                                            .map(|k| regex::escape(k))
                                                            .collect();
                                                        let pattern = format!(
                                                            r":(?:{}):",
                                                            escaped_keys.join("|")
                                                        );

                                                        let id = keys.first().cloned();

                                                        let asset = Asset::Emote {
                                                            id,
                                                            pattern,
                                                            src: uri_str.to_string(),
                                                            source: AssetSource::Server,
                                                        };

                                                        self.assets.push(asset.clone());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                dbg!(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    dbg!(e);
                }
            }
        }

        let auth_packet = ClientPacket::Authentication(
            kanii_lib::packets::client::authentication::AuthenticationPacket {
                method: "Misuzu".to_string(),
                authkey: token,
            },
        );

        let channel_assets = self.assets.clone();
        tokio::spawn(async move {
            let mut current_channel: Option<String> = None;
            let mut assets_sent = false;
            while let Some(msg) = read.next().await {
                if let Ok(msg) = msg {
                    if let Ok(sockpacket) =
                        ServerPacket::from_str(parse_html(msg.to_string()).as_str())
                    {
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
                                JoinAuthPacket::GoodAuth {
                                    user_id,
                                    username,
                                    color,
                                    channel_name,
                                    ..
                                } => {
                                    current_channel.replace(channel_name.clone());

                                    let event = ConnectionEvent::Status {
                                        event: StatusEvent::Connected { artifact: None },
                                    };
                                    let _ = event_tx.send(event);

                                    let event = ConnectionEvent::Channel {
                                        event: ChannelEvent::New {
                                            channel: Channel {
                                                id: current_channel.clone().unwrap(),
                                                name: current_channel.clone(),
                                                channel_type: ChannelType::Group,
                                            },
                                        },
                                    };
                                    let _ = event_tx.send(event);

                                    let event = ConnectionEvent::Channel {
                                        event: ChannelEvent::Join {
                                            channel_id: current_channel.clone().unwrap(),
                                        },
                                    };
                                    let _ = event_tx.send(event);

                                    let event = ConnectionEvent::Channel {
                                        event: ChannelEvent::Switch {
                                            channel_id: current_channel.clone().unwrap(),
                                        },
                                    };
                                    let _ = event_tx.send(event);

                                    let mut pic = None;
                                    if let Some(pfp_format) = pfp_url.clone() {
                                        pic = Some(pfp_format.replace("{uid}", user_id.as_str()));
                                    }

                                    let event = ConnectionEvent::User {
                                        event: UserEvent::New {
                                            channel_id: current_channel.clone(),
                                            user: Profile {
                                                id: Some(user_id),
                                                username: Some(username),
                                                display_name: None,
                                                color: kanii_to_rgba(color),
                                                picture: pic,
                                            },
                                        },
                                    };
                                    let _ = event_tx.send(event);

                                    if !assets_sent && !channel_assets.is_empty() {
                                        for asset in &channel_assets {
                                            let asset_event = AssetEvent::New {
                                                channel_id: current_channel.clone(),
                                                asset: asset.clone(),
                                            };
                                            let connection_event =
                                                ConnectionEvent::Asset { event: asset_event };
                                            let _ = event_tx.send(connection_event);
                                        }
                                        assets_sent = true;
                                    }
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
                                    color,
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
                                                color: kanii_to_rgba(color),
                                                picture: pic,
                                            },
                                        },
                                    };
                                    let _ = event_tx.send(event);
                                }
                            },

                            ServerPacket::ChatMessage(packet) => {
                                let content = parse_bbcode(packet.message.as_str());

                                let mut parsed_content = Vec::new();
                                for fragment in content {
                                    match fragment {
                                        crate::MessageFragment::Text(text) => {
                                            let asset_parsed = parse_assets(&text, &channel_assets);
                                            parsed_content.extend(asset_parsed);
                                        }
                                        other => parsed_content.push(other),
                                    }
                                }

                                let event = ConnectionEvent::Chat {
                                    event: ChatEvent::New {
                                        channel_id: current_channel.clone(),
                                        message: Message {
                                            id: Some(packet.sequence_id),
                                            sender_id: Some(packet.user_id),
                                            content: parsed_content,
                                            timestamp: DateTime::from_timestamp_nanos(
                                                packet.timestamp * 1_000_000_000,
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
                                        channel_id: current_channel.to_owned(),
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
                                    color,
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
                                                color: kanii_to_rgba(color),
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
                                        event: UserEvent::Remove {
                                            user_id,
                                            channel_id: current_channel.to_owned(),
                                        },
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
                                                    color: kanii_to_rgba(context.color),
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
                                            message: {
                                                let content = parse_bbcode(message.as_str());

                                                let mut parsed_content = Vec::new();
                                                for fragment in content {
                                                    match fragment {
                                                        crate::MessageFragment::Text(text) => {
                                                            let asset_parsed = parse_assets(
                                                                &text,
                                                                &channel_assets,
                                                            );
                                                            parsed_content.extend(asset_parsed);
                                                        }
                                                        other => parsed_content.push(other),
                                                    }
                                                }

                                                Message {
                                                    id: Some(sequence_id),
                                                    sender_id: Some(user_id),
                                                    content: parsed_content,
                                                    timestamp: DateTime::from_timestamp_nanos(
                                                        timestamp,
                                                    ),
                                                    message_type: MessageType::Normal,
                                                    status: MessageStatus::Delivered,
                                                }
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
                                        channel_id: current_channel.to_owned(),
                                        user_id: packet.user_id.to_owned(),
                                        new_user: Profile {
                                            id: Some(packet.user_id),
                                            username: Some(packet.username),
                                            display_name: None,
                                            color: kanii_to_rgba(packet.color),
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

        let _ = write.send(auth_packet.to_sockstr().into()).await;

        tokio::spawn(async move {
            loop {
                let resp = rx.recv().await;
                match resp {
                    Ok(msg) => {
                        let packet = ClientPacket::Message(
                            kanii_lib::packets::client::message::MessagePacket {
                                user_id: uid.to_owned(),
                                message: msg.to_string(),
                            },
                        )
                        .to_sockstr();
                        let _ = write.send(packet.into()).await;
                    }
                    Err(e) => match e {
                        RecvError::Lagged(skipped) => {
                            eprintln!("skipped {}x WsMessage", skipped);
                        }
                        _ => {
                            break;
                        }
                    },
                }
            }
        });

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), String> {
        if let Err(e) = self.ws_tx.send(WsMessage::Close(None)) {
            return Err(e.to_string());
        }
        Ok(())
    }

    async fn send(&mut self, event: ConnectionEvent) -> Result<(), String> {
        match event {
            ConnectionEvent::Chat {
                event:
                    ChatEvent::New {
                        channel_id: _,
                        message,
                    },
            } => {
                let text =
                    if let Some(crate::MessageFragment::Text(content)) = message.content.first() {
                        content.clone()
                    } else {
                        return Err("Unsupported message format".to_string());
                    };

                if let Err(e) = self.ws_tx.send(WsMessage::Text(text.into())) {
                    return Err(e.to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<ConnectionEvent> {
        self.event_tx.subscribe()
    }

    fn protocol_spec(&self) -> Protocol {
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
                AuthField {
                    name: "asset_api".to_string(),
                    display: Some("URL of the Mami-compatible asset API".to_string()),
                    value: crate::FieldValue::Text(None),
                    required: false,
                },
            ]),
        }
    }
}
