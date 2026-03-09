use axum::response::sse::{Event, Sse};
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::models::like::LikeEvent;

pub async fn sse_stream(
    event_rx: broadcast::Receiver<LikeEvent>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(event_rx)
        .filter_map(|result| {
            match result {
                Ok(event) => {
                    let data = serde_json::to_string(&event).ok()?;
                    Some(Ok(Event::default()
                        .event(match event.event_type {
                            crate::models::like::LikeEventType::Liked => "liked",
                            crate::models::like::LikeEventType::Unliked => "unliked",
                        })
                        .data(data)))
                }
                Err(_) => None,
            }
        });

    let heartbeat = tokio_stream::StreamExt::map(
        tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(Duration::from_secs(15))
        ),
        |_| Ok(Event::default().comment("heartbeat")),
    );

    let merged = tokio_stream::StreamExt::merge(stream, heartbeat);

    Sse::new(merged).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    )
}
