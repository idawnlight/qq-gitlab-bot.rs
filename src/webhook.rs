use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use actix_web::web::Bytes;
use gitlab::webhooks::PushHook;
use crate::AppState;
use serde_json;
use crate::bot::{send_message, SimpleMessage};

#[derive(Debug, Deserialize)]
pub struct Webhook {
    pub target: String,
    pub to: i64,
    secret: String
}

#[post("/{identifier}")]
pub async fn handle(
    body: Bytes,
    request: web::HttpRequest,
    path: web::Path<String>,
    data: web::Data<AppState>
) -> impl Responder {
    // dbg!(&data.webhooks);
    // dbg!(identifier);
    let identifier = path.into_inner();
    match data.webhooks.get(&identifier) {
        Some(webhook) => {
            if !webhook.secret.is_empty() {
                let token = if let Some(token) = request.head().headers().get("X-Gitlab-Token") {
                    token.to_str().unwrap()
                } else {
                    error!("Webhook {} is called without token", identifier);
                    return HttpResponse::Unauthorized().body("unauthorized (empty token)");
                };
                if !webhook.secret.eq(token) {
                    error!("Webhook {} is called with incorrect token {}", identifier, token);
                    return HttpResponse::Unauthorized().body("unauthorized (incorrect token)");
                }
            }

            let mut message = SimpleMessage::init(webhook);

            if let Some(event) = request.headers().get("X-Gitlab-Event") {
                match event.to_str().unwrap_or("") {
                    "Push Hook" => {
                        // dbg!(&body);
                        let push_hook: PushHook = serde_json::from_slice(&body).unwrap();
                        // dbg!(&push_hook);
                        message.content = format!("Recent commit to {} by {}", push_hook.project.path_with_namespace, push_hook.user_username);
                        for commit in &push_hook.commits {
                            message.content = format!("{}\n{} {}", message.content,
                                                  commit.id.value()[0..7].to_string(),
                                                  commit.message.lines().next().unwrap_or(commit.message.as_str())
                            );

                            let mut modification = "".to_string();
                            if commit.added.as_ref().unwrap().len() > 0 {
                                modification.push_str(format!("{}+", commit.added.as_ref().unwrap().len()).as_str());
                            }
                            if commit.modified.as_ref().unwrap().len() > 0 {
                                modification.push_str(format!("{}M", commit.modified.as_ref().unwrap().len()).as_str());
                            }
                            if commit.removed.as_ref().unwrap().len() > 0 {
                                modification.push_str(format!("{}-", commit.removed.as_ref().unwrap().len()).as_str());
                            }

                            message.content = format!("{} ({})", message.content, modification.as_str());
                        }
                        message.content = format!("{}\n\n{}", message.content,
                                                  push_hook.commits.first().unwrap().url
                        );
                        send_message(message).await;
                    },
                    // "Tag Push Hook" => (),
                    // "Merge Request Hook" => (),
                    // "Note Hook" => (),
                    // "Pipeline Hook" => (),
                    // "Wiki Page Hook" => (),
                    // "Issue Hook" => (),
                    // "Confidential Issue Hook" => (),
                    // "Snippet Hook" => (),
                    // "Job Hook" => (),
                    // "Build Hook" => (),
                    // "Repository Hook" => (),
                    _ => return HttpResponse::BadRequest().body("unknown event")
                }
            } else {
                return HttpResponse::BadRequest().body("unknown event");
            };

            HttpResponse::Ok().body("success")
        }
        None => {
            error!("No webhook found for identifier {}", identifier);
            HttpResponse::NotFound().body("webhook not found")
        }
    }
}