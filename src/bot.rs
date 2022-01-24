use crate::{SETTINGS, Webhook};
use serde::{Deserialize, Serialize};

pub enum SimpleMessageTarget {
    Group,
    Private,
}

pub struct SimpleMessage {
    pub to: i64,
    pub target: SimpleMessageTarget,
    pub content: String
}

impl SimpleMessage {
    pub fn init(webhook: &Webhook) -> SimpleMessage {
        let target = match webhook.target.as_str() {
            "group" => SimpleMessageTarget::Group,
            "private" => SimpleMessageTarget::Private,
            _ => panic!("Unknown target type")
        };

        SimpleMessage {
            to: webhook.to,
            target,
            content: "".to_string()
        }
    }
}

#[derive(Serialize, Debug)]
pub struct SendPrivateMessage {
    pub user_id: i64,
    pub group_id: Option<i64>,
    pub message: String,
    pub auto_escape: bool
}

impl SendPrivateMessage {
    pub fn simple(user_id: i64, message: String) -> Self {
        SendPrivateMessage {
            user_id,
            group_id: None,
            message,
            auto_escape: false
        }
    }
}

impl Into<SendPrivateMessage> for SimpleMessage {
    fn into(self) -> SendPrivateMessage {
        SendPrivateMessage {
            user_id: self.to,
            group_id: None,
            message: self.content,
            auto_escape: false
        }
    }
}

#[derive(Serialize, Debug)]
pub struct SendGroupMessage {
    pub group_id: i64,
    pub message: String,
    pub auto_escape: bool
}

impl SendGroupMessage {
    pub fn simple(group_id: i64, message: String) -> Self {
        SendGroupMessage {
            group_id,
            message,
            auto_escape: false
        }
    }
}

impl Into<SendGroupMessage> for SimpleMessage {
    fn into(self) -> SendGroupMessage {
        SendGroupMessage {
            group_id: self.to,
            message: self.content,
            auto_escape: false
        }
    }
}

#[derive(Deserialize)]
pub struct SendMessageResponse {
    pub data: SendMessageResponseData,
    pub retcode: i32,
    pub status: String
}

#[derive(Deserialize)]
pub struct SendMessageResponseData {
    pub message_id: i32
}

#[derive(Deserialize)]
pub struct OnebotAboutResponse {
    pub data: OnebotAbout,
    pub retcode: i32,
    pub status: String
}

#[derive(Deserialize)]
pub struct OnebotAbout {
    pub app_name: String,
    pub app_version: String,
    pub protocol: i32
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
        },
        Err(e) => {
            None
        }
    }
}

pub async fn send_message(data: SimpleMessage) -> Option<SendMessageResponse> {
    match data.target {
        SimpleMessageTarget::Private => send_private_message(data.into()).await,
        SimpleMessageTarget::Group => send_group_message(data.into()).await
    }
}

pub async fn send_private_message(data: SendPrivateMessage) -> Option<SendMessageResponse> {
    let client = reqwest::Client::new();
    let resp = client.post(get_api("/send_private_msg"))
        .json(&data)
        .send()
        .await;
    match resp {
        Ok(r) => {
            dbg!(&r);
            Some(r.json::<SendMessageResponse>().await.unwrap())
        },
        Err(e) => {
            dbg!(e);
            None
        }
    }
}

pub async fn send_group_message(data: SendGroupMessage) -> Option<SendMessageResponse> {
    let client = reqwest::Client::new();
    let resp = client.post(get_api("/send_group_msg"))
        .json(&data)
        .send()
        .await;
    match resp {
        Ok(r) => {
            Some(r.json::<SendMessageResponse>().await.unwrap())
        },
        Err(e) => {
            None
        }
    }
}