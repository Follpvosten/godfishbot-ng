use std::{collections::HashMap, sync::Arc};

use color_eyre::{eyre::bail, Result};
use reqwest::Client;
use serde::Deserialize;
use tbot::{
    contexts::{methods::Message, Command},
    types::{file, input_file::Animation, message::Kind},
};
use tokio::sync::Mutex;

use crate::ResultExt;

pub async fn handler(ctx: Arc<Command>, state: Arc<(Client, Mutex<HashMap<String, file::Id>>)>) {
    if let Err(error) = attempt_flausch(&ctx, &state.0, &state.1).await {
        ctx.send_message_in_reply(format!("Error attempting flausch: {}", error))
            .call()
            .await
            .log_err();
    }
}

#[derive(Debug, Deserialize)]
struct FlauschResponse {
    id: String,
    media: FlauschMedia,
}

#[derive(Debug, Deserialize)]
struct FlauschMedia {
    mp4: String,
}

const URL: &str = "https://api.bunnies.io/v2/loop/random/?media=mp4";
async fn attempt_flausch(
    ctx: &Arc<Command>,
    client: &Client,
    file_id_map: &Mutex<HashMap<String, file::Id>>,
) -> Result<()> {
    let resp: FlauschResponse = client.get(URL).send().await?.json().await?;
    if let Some(id) = file_id_map.lock().await.get(&resp.id).cloned() {
        ctx.send_animation(Animation::with_id(id)).call().await?;
        return Ok(());
    }
    match ctx
        .send_animation(Animation::with_url(&resp.media.mp4))
        .call()
        .await?
        .kind
    {
        Kind::Animation { animation, .. } => {
            file_id_map.lock().await.insert(resp.id, animation.file_id);
        }
        // We need to handle this because it will turn stuff with audio into a video
        Kind::Video { video, .. } => {
            file_id_map.lock().await.insert(resp.id, video.file_id);
        }
        _ => bail!("got non-animation from animation"),
    }
    Ok(())
}
