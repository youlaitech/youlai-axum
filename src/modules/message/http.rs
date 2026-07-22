use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::{self, Stream, StreamExt};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::modules::auth::extractor::AuthUser;
use crate::modules::message::hub::{SseEvent, SseGuard};
use crate::modules::message::topics::ONLINE_COUNT;
use crate::state::AppState;

/// 包装一层使 guard 随 stream 存活，stream 丢弃时 guard drop 自动清理
struct GuardedStream<S> {
    inner: S,
    _guard: SseGuard,
}

impl<S: Stream + Unpin> Stream for GuardedStream<S> {
    type Item = S::Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner).poll_next(cx)
    }
}

/// 建立 SSE 长连接
pub async fn sse_connect(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (rx, guard) = state.message_hub.connect(user.user_id).await;

    // 连接建立即推送当前在线人数，并通知所有人人数变化
    let count = state.message_hub.online_count();
    state
        .message_hub
        .broadcast(ONLINE_COUNT, serde_json::json!(count))
        .await;

    // 首条 online-count
    let init = stream::once(async move {
        Ok::<_, Infallible>(Event::default().event(ONLINE_COUNT).data(count.to_string()))
    });

    // 后续事件流：SseEvent -> axum Event
    let evt_stream = UnboundedReceiverStream::new(rx).map(|SseEvent { event, data }| {
        Ok::<_, Infallible>(
            Event::default()
                .event(&event)
                .json_data(&data)
                .unwrap_or_else(|_| Event::default().event(&event).data("{}")),
        )
    });

    // guard 随事件流存活，流结束时（客户端断开）自动 drop
    let guarded = GuardedStream {
        inner: evt_stream,
        _guard: guard,
    };

    Sse::new(init.chain(guarded)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("ping"),
    )
}
