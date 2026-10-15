use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Handle {
    pub address: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRef {
    pub guid: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    pub guid: String,
    #[serde(default)]
    pub chat_identifier: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub style: Option<i64>,
    #[serde(default)]
    pub participants: Vec<Handle>,
    #[serde(default)]
    pub last_message: Option<Message>,
}

impl Chat {
    pub fn is_group(&self) -> bool {
        self.style == Some(43) || self.participants.len() > 1
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub guid: String,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub transfer_name: Option<String>,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub total_bytes: Option<i64>,
    #[serde(default)]
    pub hide_attachment: bool,
}

impl Attachment {
    pub fn is_image(&self) -> bool {
        self.mime_type
            .as_deref()
            .is_some_and(|mime| mime.starts_with("image/"))
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub guid: String,
    #[serde(default)]
    pub temp_guid: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub handle: Option<Handle>,
    #[serde(default)]
    pub is_from_me: bool,
    #[serde(default)]
    pub date_created: i64,
    #[serde(default)]
    pub date_read: Option<i64>,
    #[serde(default)]
    pub date_delivered: Option<i64>,
    #[serde(default)]
    pub is_delivered: bool,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub associated_message_guid: Option<String>,
    #[serde(default)]
    pub associated_message_type: Option<Value>,
    #[serde(default)]
    pub item_type: i64,
    #[serde(default)]
    pub error: i64,
    #[serde(default)]
    pub chats: Vec<ChatRef>,
}

impl Message {
    pub fn body(&self) -> &str {
        self.text
            .as_deref()
            .map(|text| text.trim_matches(['\u{fffc}', '\u{fffd}']).trim())
            .unwrap_or_default()
    }

    pub fn is_reaction(&self) -> bool {
        self.associated_message_type
            .as_ref()
            .is_some_and(|value| !value.is_null())
    }

    pub fn reaction_target(&self) -> Option<&str> {
        let guid = self.associated_message_guid.as_deref()?;
        Some(guid.rsplit('/').next().unwrap_or(guid))
    }

    pub fn reaction_kind(&self) -> Option<String> {
        match self.associated_message_type.as_ref()? {
            Value::String(kind) => Some(kind.clone()),
            Value::Number(code) => Some(code.to_string()),
            _ => None,
        }
    }

    pub fn visible_attachments(&self) -> impl Iterator<Item = &Attachment> {
        self.attachments.iter().filter(|a| !a.hide_attachment)
    }

    pub fn chat_guid(&self) -> Option<&str> {
        self.chats.first().map(|chat| chat.guid.as_str())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactAddress {
    pub address: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub phone_numbers: Vec<ContactAddress>,
    #[serde(default)]
    pub emails: Vec<ContactAddress>,
    #[serde(default)]
    pub avatar: Option<String>,
}

impl Contact {
    pub fn name(&self) -> Option<String> {
        if let Some(name) = self
            .display_name
            .as_deref()
            .filter(|n| !n.trim().is_empty())
        {
            return Some(name.trim().to_string());
        }
        let joined = [self.first_name.as_deref(), self.last_name.as_deref()]
            .into_iter()
            .flatten()
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        (!joined.is_empty()).then_some(joined)
    }
}
