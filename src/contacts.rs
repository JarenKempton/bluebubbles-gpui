use std::{collections::HashMap, sync::Arc};

use base64::Engine as _;
use gpui_kit::Image;

use crate::{api::Contact, media::decode_image};

#[derive(Default)]
pub struct ContactBook {
    names: HashMap<String, String>,
    avatars: HashMap<String, Arc<Image>>,
}

impl ContactBook {
    pub fn from_contacts(contacts: Vec<Contact>) -> Self {
        let mut book = Self::default();
        for contact in contacts {
            let Some(name) = contact.name() else { continue };
            let avatar = contact
                .avatar
                .as_deref()
                .filter(|data| !data.is_empty())
                .and_then(|data| base64::engine::general_purpose::STANDARD.decode(data).ok())
                .and_then(decode_image);
            for address in contact.phone_numbers.iter().chain(&contact.emails) {
                let key = key(&address.address);
                if key.is_empty() {
                    continue;
                }
                book.names.entry(key.clone()).or_insert_with(|| name.clone());
                if let Some(avatar) = &avatar {
                    book.avatars.entry(key).or_insert_with(|| avatar.clone());
                }
            }
        }
        book
    }

    pub fn name(&self, address: &str) -> Option<&str> {
        self.names.get(&key(address)).map(String::as_str)
    }

    pub fn avatar(&self, address: &str) -> Option<Arc<Image>> {
        self.avatars.get(&key(address)).cloned()
    }

    pub fn display(&self, address: &str) -> String {
        self.name(address)
            .map(str::to_string)
            .unwrap_or_else(|| format_address(address))
    }
}

fn key(address: &str) -> String {
    if address.contains('@') {
        return address.trim().to_lowercase();
    }
    let digits: String = address.chars().filter(char::is_ascii_digit).collect();
    let start = digits.len().saturating_sub(10);
    digits[start..].to_string()
}

fn format_address(address: &str) -> String {
    let digits: String = address.chars().filter(char::is_ascii_digit).collect();
    match (address.starts_with('+'), digits.len()) {
        (true, 11) if digits.starts_with('1') => {
            format!("({}) {}-{}", &digits[1..4], &digits[4..7], &digits[7..])
        }
        (false, 10) => format!("({}) {}-{}", &digits[..3], &digits[3..6], &digits[6..]),
        _ => address.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_numbers_regardless_of_formatting() {
        assert_eq!(key("+1 (480) 555-0100"), key("4805550100"));
        assert_eq!(key("Me@Example.com "), "me@example.com");
    }

    #[test]
    fn formats_us_numbers() {
        assert_eq!(format_address("+14805550100"), "(480) 555-0100");
        assert_eq!(format_address("701107"), "701107");
    }
}
