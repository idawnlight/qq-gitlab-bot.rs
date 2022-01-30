use std::str::FromStr;
use std::string::ToString;
use reqwest::{Error, Response};

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::{SETTINGS, Webhook};

#[derive(Display, EnumString)]
pub enum SimpleMessageTarget {
    #[strum(serialize = "group")]
    Group,
    #[strum(serialize = "private")]
    Private,
}

pub struct SimpleMessage {
    pub to: i64,
    pub target: SimpleMessageTarget,
    pub content: String,
}

impl SimpleMessage {
    pub fn init(webhook: &Webhook) -> SimpleMessage {
        let target = SimpleMessageTarget::from_str(webhook.target.as_str())
            .unwrap_or_else(|_| {
                warn!("Invalid target: {}, assuming private", webhook.target);
                SimpleMessageTarget::Private
            });

        SimpleMessage {
            to: webhook.to,
            target,
            content: "".to_string(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct SendPrivateMessage {
    pub user_id: i64,
    pub group_id: Option<i64>,
    pub message: String,
    pub auto_escape: bool,
}

#[allow(dead_code)]
impl SendPrivateMessage {
    pub fn simple(user_id: i64, message: String) -> Self {
        SendPrivateMessage {
            user_id,
            group_id: None,
            message,
            auto_escape: false,
        }
    }
}

impl From<SimpleMessage> for SendPrivateMessage {
    fn from(msg: SimpleMessage) -> Self {
        SendPrivateMessage {
            user_id: msg.to,
            group_id: None,
            message: msg.content,
            auto_escape: false,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct SendGroupMessage {
    pub group_id: i64,
    pub message: String,
    pub auto_escape: bool,
}

#[allow(dead_code)]
impl SendGroupMessage {
    pub fn simple(group_id: i64, message: String) -> Self {
        SendGroupMessage {
            group_id,
            message,
            auto_escape: false,
        }
    }
}

impl From<SimpleMessage> for SendGroupMessage {
    fn from(msg: SimpleMessage) -> Self {
        SendGroupMessage {
            group_id: msg.to,
            message: msg.content,
            auto_escape: false,
        }
    }
}

#[derive(Deserialize)]
pub struct SendMessageResponse {
    pub data: SendMessageResponseData,
    pub retcode: i32,
    pub status: String,
}

#[derive(Deserialize)]
pub struct SendMessageResponseData {
    pub message_id: i32,
}

#[derive(Deserialize)]
pub struct OnebotAboutResponse {
    pub data: OnebotAbout,
    pub retcode: i32,
    pub status: String,
}

#[derive(Deserialize)]
pub struct OnebotAbout {
    pub app_name: String,
    pub app_version: String,
    pub protocol: i32,
}

pub fn get_api(path: &str) -> String {
    format!("{}{}",
            SETTINGS.read().unwrap().get_str("api.http").unwrap(),
            path)
}

pub async fn test_api() -> Option<OnebotAbout> {
    let resp = reqwest::get(get_api("/get_version_info")).await;
    match resp {
        Ok(r) => {
            Some(r.json::<OnebotAboutResponse>().await.unwrap().data)
        }
        Err(_) => {
            None
        }
    }
}

pub async fn send_message(data: SimpleMessage) -> Option<SendMessageResponse> {
    let info = format!("{} {}", data.target, data.to);

    let result = match data.target {
        SimpleMessageTarget::Private => send_private_message(data.into()).await,
        SimpleMessageTarget::Group => send_group_message(data.into()).await
    };

    match result {
        Some(r) => {
            info!("Message sent to {} successfully", info);
            Some(r)
        }
        None => {
            error!("Failed to send message to {}", info);
            None
        }
    }
}

pub async fn send_private_message(data: SendPrivateMessage) -> Option<SendMessageResponse> {
    let client = reqwest::Client::new();
    let resp = client.post(get_api("/send_private_msg"))
        .json(&data)
        .send()
        .await;
    send_message_result(resp).await
}

pub async fn send_group_message(data: SendGroupMessage) -> Option<SendMessageResponse> {
    let client = reqwest::Client::new();
    let resp = client.post(get_api("/send_group_msg"))
        .json(&data)
        .send()
        .await;
    send_message_result(resp).await
}

async fn send_message_result(response: Result<Response, Error>) -> Option<SendMessageResponse> {
    match response {
        Ok(r) => {
            Some(r.json::<SendMessageResponse>().await.unwrap())
        }
        Err(e) => {
            warn!("{}", e);
            None
        }
    }
}
