use std::time::Duration;

use futures::{FutureExt as _, channel::mpsc};
use rust_socketio::{Event, Payload, asynchronous::ClientBuilder};
use serde_json::Value;

use crate::{
    api::{Client, Message},
    runtime,
};

pub enum LiveEvent {
    Connected,
    Disconnected,
    NewMessage(Message),
    UpdatedMessage(Message),
}

pub fn subscribe(client: &Client) -> mpsc::UnboundedReceiver<LiveEvent> {
    let (tx, rx) = mpsc::unbounded();
    let url = socket_url(client);
    runtime::spawn(async move {
        loop {
            let builder = ClientBuilder::new(url.clone())
                .reconnect(true)
                .reconnect_on_disconnect(true)
                .reconnect_delay(1_000, 10_000)
                .max_reconnect_attempts(u8::MAX)
                .on(Event::Connect, forward(&tx, |_| Some(LiveEvent::Connected)))
                .on(
                    Event::Close,
                    forward(&tx, |_| Some(LiveEvent::Disconnected)),
                )
                .on(
                    "new-message",
                    forward(&tx, |v| parse(v).map(LiveEvent::NewMessage)),
                )
                .on(
                    "updated-message",
                    forward(&tx, |v| parse(v).map(LiveEvent::UpdatedMessage)),
                );
            match builder.connect().await {
                Ok(socket) => {
                    while !tx.is_closed() {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    let _ = socket.disconnect().await;
                    return;
                }
                Err(_) => {
                    if tx.unbounded_send(LiveEvent::Disconnected).is_err() {
                        return;
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });
    rx
}

type Callback = Box<
    dyn FnMut(
            Payload,
            rust_socketio::asynchronous::Client,
        ) -> futures::future::BoxFuture<'static, ()>
        + Send
        + Sync,
>;

fn forward(
    tx: &mpsc::UnboundedSender<LiveEvent>,
    map: impl Fn(Option<&Value>) -> Option<LiveEvent> + Send + Sync + 'static,
) -> Callback {
    let tx = tx.clone();
    Box::new(move |payload, _| {
        let event = match &payload {
            Payload::Text(values) => map(values.first()),
            _ => map(None),
        };
        if let Some(event) = event {
            let _ = tx.unbounded_send(event);
        }
        async {}.boxed()
    })
}

fn parse(value: Option<&Value>) -> Option<Message> {
    serde_json::from_value(value?.clone()).ok()
}

fn socket_url(client: &Client) -> String {
    let mut url = reqwest::Url::parse(client.root()).expect("server url was normalized");
    url.query_pairs_mut()
        .append_pair("password", client.password());
    url.to_string()
}
