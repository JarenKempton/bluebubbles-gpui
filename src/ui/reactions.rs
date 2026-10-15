use std::collections::HashMap;

use crate::api::Message;

struct Mark {
    sender: String,
    emoji: Option<String>,
    at: i64,
}

#[derive(Default)]
pub struct Reactions {
    by_target: HashMap<String, Vec<Mark>>,
}

impl Reactions {
    pub fn apply(&mut self, msg: &Message) -> Option<String> {
        let target = msg.reaction_target()?.to_string();
        let emoji = emoji_for(msg)?;
        let sender = if msg.is_from_me {
            "me".to_string()
        } else {
            msg.handle
                .as_ref()
                .map(|h| h.address.clone())
                .unwrap_or_default()
        };
        let marks = self.by_target.entry(target.clone()).or_default();
        match marks.iter_mut().find(|mark| mark.sender == sender) {
            Some(mark) if mark.at > msg.date_created => return None,
            Some(mark) => {
                mark.emoji = emoji;
                mark.at = msg.date_created;
            }
            None => marks.push(Mark {
                sender,
                emoji,
                at: msg.date_created,
            }),
        }
        Some(target)
    }

    pub fn for_message(&self, guid: &str) -> Vec<&str> {
        self.by_target
            .get(guid)
            .map(|marks| marks.iter().filter_map(|m| m.emoji.as_deref()).collect())
            .unwrap_or_default()
    }
}

fn emoji_for(msg: &Message) -> Option<Option<String>> {
    let kind = msg.reaction_kind()?;
    if kind.starts_with('-') || (kind.len() == 4 && kind.starts_with('3')) {
        return Some(None);
    }
    let emoji = match kind.as_str() {
        "love" | "2000" => "❤️",
        "like" | "2001" => "👍",
        "dislike" | "2002" => "👎",
        "laugh" | "2003" => "😂",
        "emphasize" | "2004" => "‼️",
        "question" | "2005" => "❓",
        "emoji" | "2006" => return Some(Some(custom_emoji(msg.body())?)),
        _ => return None,
    };
    Some(Some(emoji.to_string()))
}

fn custom_emoji(text: &str) -> Option<String> {
    let emoji = text.strip_prefix("Reacted ")?.split(" to ").next()?.trim();
    (!emoji.is_empty()).then(|| emoji.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::api::Handle;

    fn reaction(kind: serde_json::Value, from: &str, at: i64, text: &str) -> Message {
        Message {
            guid: format!("r-{at}"),
            text: Some(text.to_string()),
            handle: Some(Handle {
                address: from.to_string(),
            }),
            date_created: at,
            associated_message_guid: Some("p:0/TARGET".to_string()),
            associated_message_type: Some(kind),
            ..Default::default()
        }
    }

    #[test]
    fn newer_reaction_replaces_and_removal_clears() {
        let mut reactions = Reactions::default();
        reactions.apply(&reaction(json!("like"), "+1", 1, ""));
        reactions.apply(&reaction(json!("love"), "+1", 2, ""));
        reactions.apply(&reaction(json!(2006), "+2", 3, "Reacted 🔥 to “hi”"));
        assert_eq!(reactions.for_message("TARGET"), vec!["❤️", "🔥"]);
        reactions.apply(&reaction(json!("-love"), "+1", 4, ""));
        assert_eq!(reactions.for_message("TARGET"), vec!["🔥"]);
    }

    #[test]
    fn older_history_does_not_override_newer_state() {
        let mut reactions = Reactions::default();
        reactions.apply(&reaction(json!("-like"), "+1", 5, ""));
        reactions.apply(&reaction(json!("like"), "+1", 1, ""));
        assert!(reactions.for_message("TARGET").is_empty());
    }
}
