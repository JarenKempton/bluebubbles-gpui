use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use gpui_kit::*;

use crate::{api::Client, runtime};

pub enum Media {
    Loading,
    Ready(Arc<Image>, Option<Size<u32>>),
    NotOnMac,
    Failed,
}

#[derive(Clone)]
enum Source {
    Attachment(String),
    GroupIcon(String),
}

pub struct MediaCache {
    client: Client,
    images: HashMap<String, Media>,
}

impl MediaCache {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            images: HashMap::new(),
        }
    }

    pub fn image(&mut self, guid: &str, cx: &mut Context<Self>) -> &Media {
        self.get(guid.to_string(), Source::Attachment(guid.to_string()), cx)
    }

    pub fn insert_local(&mut self, guid: &str, image: Arc<Image>, dims: Option<Size<u32>>) {
        self.images
            .insert(guid.to_string(), Media::Ready(image, dims));
    }

    pub fn group_icon(&mut self, chat_guid: &str, cx: &mut Context<Self>) -> Option<Arc<Image>> {
        match self.get(
            format!("icon:{chat_guid}"),
            Source::GroupIcon(chat_guid.to_string()),
            cx,
        ) {
            Media::Ready(image, _) => Some(image.clone()),
            _ => None,
        }
    }

    fn get(&mut self, key: String, source: Source, cx: &mut Context<Self>) -> &Media {
        if !self.images.contains_key(&key) {
            self.images.insert(key.clone(), Media::Loading);
            self.fetch(key.clone(), source, cx);
        }
        &self.images[&key]
    }

    fn fetch(&self, key: String, source: Source, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = runtime::run(async move {
                let bytes = match source {
                    Source::Attachment(guid) => client.attachment(&guid, Some(800)).await?,
                    Source::GroupIcon(chat) => client.group_icon(&chat).await?,
                };
                let dims = imagesize::blob_size(&bytes)
                    .ok()
                    .map(|d| size(d.width as u32, d.height as u32));
                let image =
                    decode_image(bytes).ok_or_else(|| anyhow!("unsupported image format"))?;
                anyhow::Ok((image, dims))
            })
            .await;
            this.update(cx, |this, cx| {
                let media = match result {
                    Ok((image, dims)) => Media::Ready(image, dims),
                    Err(err) if err.to_string().contains("does not exist") => Media::NotOnMac,
                    Err(_) => Media::Failed,
                };
                this.images.insert(key, media);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

pub fn decode_image(bytes: Vec<u8>) -> Option<Arc<Image>> {
    let format = image_format(&bytes)?;
    Some(Arc::new(Image::from_bytes(format, bytes)))
}

fn image_format(bytes: &[u8]) -> Option<ImageFormat> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some(ImageFormat::Png),
        [0xFF, 0xD8, ..] => Some(ImageFormat::Jpeg),
        [b'G', b'I', b'F', b'8', ..] => Some(ImageFormat::Gif),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => Some(ImageFormat::Webp),
        [b'B', b'M', ..] => Some(ImageFormat::Bmp),
        [b'I', b'I', 0x2A, 0x00, ..] | [b'M', b'M', 0x00, 0x2A, ..] => Some(ImageFormat::Tiff),
        _ => None,
    }
}

pub fn open_attachment(client: Client, attachment: crate::api::Attachment) {
    runtime::spawn(async move {
        let Ok(bytes) = client.attachment(&attachment.guid, None).await else {
            return;
        };
        let dir = dirs::download_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("BlueBubbles");
        let name = attachment
            .transfer_name
            .as_deref()
            .and_then(|n| std::path::Path::new(n).file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| attachment.guid.clone());
        let path = dir.join(name);
        if tokio::fs::create_dir_all(&dir).await.is_ok()
            && tokio::fs::write(&path, bytes).await.is_ok()
        {
            let _ = open::that_detached(&path);
        }
    });
}
