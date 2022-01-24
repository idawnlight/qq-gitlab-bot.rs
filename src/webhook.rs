use actix_web::{HttpResponse, post, Responder, web};
use actix_web::web::Bytes;
use gitlab::NoteType;
use gitlab::webhooks::{IssueAction, MergeRequestAction, WebHook};
use serde::Deserialize;
use serde_json;

use crate::AppState;
use crate::bot::{send_message, SimpleMessage};

#[derive(Debug, Deserialize)]
pub struct Webhook {
    pub target: String,
    pub to: i64,
    secret: String,
}

#[post("/{identifier}")]
pub async fn handle(
    body: Bytes,
    request: web::HttpRequest,
    path: web::Path<String>,
    data: web::Data<AppState>,
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

            let hook = serde_json::from_slice::<WebHook>(&body);
            // dbg!(&hook);
            // return HttpResponse::Ok().body("success");

            match hook {
                Ok(hook) => match hook {
                    WebHook::Push(push_hook) => {
                        if push_hook.ref_.starts_with("refs/heads/") {
                            let branch = push_hook.ref_[11..].to_string();
                            message.content = format!("Recent commit to {}:{} by {}", push_hook.project.path_with_namespace, branch, push_hook.user_username);

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

                            if push_hook.commits.len() > 0 {
                                message.content = format!("{}\n\n{}", message.content,
                                                          push_hook.commits.first().unwrap().url
                                );
                            }
                        } else if push_hook.ref_.starts_with("refs/tags/") {
                            let tag = push_hook.ref_[10..].to_string();
                            message.content = format!("New tag {} on {} by {}", tag.as_str(), push_hook.project.path_with_namespace, push_hook.user_username);
                            message.content = format!("{}\n\n{}", message.content,
                                                      format!("{}{}{}", push_hook.project.web_url, "/-/tags/", tag.as_str())
                            );
                        } else {
                            message.content = format!("New {} on {} by {}", push_hook.ref_, push_hook.project.path_with_namespace, push_hook.user_username);
                        }

                        send_message(message).await;
                    }
                    WebHook::Issue(issue_hook) => {
                        let keyword = match issue_hook.object_attributes.action {
                            None => return HttpResponse::BadRequest().body("unknown issue action"),
                            Some(action) => match action {
                                IssueAction::Update => "updated",
                                IssueAction::Open => "opened",
                                IssueAction::Close => "closed",
                                IssueAction::Reopen => "reopened"
                            }
                        };
                        message.content = format!("{} {} issue {}#{}", issue_hook.user.username, keyword, issue_hook.project.path_with_namespace, issue_hook.object_attributes.iid);
                        message.content = format!("{}\n{}\n{}", message.content, issue_hook.object_attributes.title, issue_hook.object_attributes.description.unwrap_or("".to_string()));
                        message.content = format!("{}\n\n{}", message.content, issue_hook.object_attributes.url.unwrap_or("Fail to fetch issue url, a bug of GitLab?".to_string()));
                        send_message(message).await;
                    }
                    WebHook::Note(note_hook) => {
                        match note_hook.object_attributes.noteable_type {
                            NoteType::Commit => {
                                message.content = format!("{} commented on {}@{}", note_hook.user.username, note_hook.project.path_with_namespace, note_hook.object_attributes.commit_id.unwrap().value()[0..7].to_string());
                                message.content = format!("{}\n{}", message.content, note_hook.object_attributes.note);
                                message.content = format!("{}\n\n{}", message.content, note_hook.object_attributes.url);
                                send_message(message).await;
                            }
                            NoteType::Issue => {
                                message.content = format!("{} commented on {}#{}", note_hook.user.username, note_hook.project.path_with_namespace, note_hook.issue.unwrap().iid);
                                message.content = format!("{}\n{}", message.content, note_hook.object_attributes.note);
                                message.content = format!("{}\n\n{}", message.content, note_hook.object_attributes.url);
                                send_message(message).await;
                            }
                            NoteType::MergeRequest => {
                                message.content = format!("{} commented on {}#{}", note_hook.user.username, note_hook.project.path_with_namespace, note_hook.merge_request.unwrap().iid);
                                message.content = format!("{}\n{}", message.content, note_hook.object_attributes.note);
                                message.content = format!("{}\n\n{}", message.content, note_hook.object_attributes.url);
                                send_message(message).await;
                            }
                            NoteType::Snippet => {
                                message.content = format!("{} commented on snippet {}", note_hook.user.username, note_hook.snippet.unwrap().title);
                                message.content = format!("{}\n{}", message.content, note_hook.object_attributes.note);
                                message.content = format!("{}\n\n{}", message.content, note_hook.object_attributes.url);
                                send_message(message).await;
                            }
                        }
                    }
                    WebHook::MergeRequest(mr_hook) => {
                        let keyword = match mr_hook.object_attributes.action {
                            None => return HttpResponse::BadRequest().body("unknown mr action"),
                            Some(action) => match action {
                                MergeRequestAction::Update => "updated",
                                MergeRequestAction::Open => "opened",
                                MergeRequestAction::Close => "closed",
                                MergeRequestAction::Reopen => "reopened",
                                MergeRequestAction::Approved => "approved",
                                MergeRequestAction::Unapproved => "unapproved",
                                MergeRequestAction::Merge => "merged"
                            }
                        };
                        message.content = format!("{} {} mr {}#{}", mr_hook.user.username, keyword, mr_hook.project.path_with_namespace, mr_hook.object_attributes.iid);
                        message.content = format!("{}\n\n{}", message.content, mr_hook.object_attributes.url.unwrap_or("Fail to fetch merge request url, a bug of GitLab?".to_string()));
                        send_message(message).await;
                    }
                    WebHook::Build(_) => {
                        message.content = format!("Unsupported action build");
                        send_message(message).await;
                    }
                    WebHook::Pipeline(_) => {
                        message.content = format!("Unsupported action pipeline");
                        send_message(message).await;
                    }
                    WebHook::WikiPage(_) => {
                        message.content = format!("Unsupported action wiki page");
                        send_message(message).await;
                    }
                }
                Err(e) => {
                    message.content = e.to_string();
                    send_message(message).await;
                    return HttpResponse::BadRequest().body("unknown event");
                }
            };

            HttpResponse::Ok().body("success")
        }
        None => {
            error!("No webhook found for identifier {}", identifier);
            HttpResponse::NotFound().body("webhook not found")
        }
    }
}