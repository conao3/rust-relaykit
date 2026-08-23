use crate::{backend::BackendClient, protocol::RelayReply, telegram::TelegramClient};
use anyhow::Result;
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

pub async fn run(telegram: TelegramClient, backend: BackendClient) -> Result<()> {
    let mut offset = None;
    info!("relaykit gateway started");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("shutdown signal received");
                return Ok(());
            }
            result = poll_once(&telegram, &backend, offset) => {
                match result {
                    Ok(next_offset) => offset = next_offset,
                    Err(error) => {
                        error!(%error, "poll iteration failed");
                        sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }
    }
}

async fn poll_once(
    telegram: &TelegramClient,
    backend: &BackendClient,
    offset: Option<i64>,
) -> Result<Option<i64>> {
    let updates = telegram.get_updates(offset).await?;
    let mut next_offset = offset;

    for update in updates {
        next_offset = Some(update.update_id + 1);
        let Some(event) = update.into_relay_event() else {
            continue;
        };

        if !telegram.is_allowed_chat(&event.chat.id) {
            warn!(chat_id = %event.chat.id, "dropping event from disallowed chat");
            continue;
        }

        let replies = backend.dispatch(&event).await?;
        for reply in replies.messages {
            send_reply(
                telegram,
                &event.chat.id,
                event.chat.thread_id.as_deref(),
                reply,
            )
            .await?;
        }
    }

    Ok(next_offset)
}

async fn send_reply(
    telegram: &TelegramClient,
    default_chat_id: &str,
    default_thread_id: Option<&str>,
    reply: RelayReply,
) -> Result<()> {
    let chat_id = reply.chat_id.as_deref().unwrap_or(default_chat_id);
    let thread_id = reply.thread_id.as_deref().or(default_thread_id);
    telegram
        .send_message(
            chat_id,
            thread_id,
            &reply.text,
            reply.reply_to_message_id.as_deref(),
        )
        .await
}
