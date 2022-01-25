use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use color_eyre::{eyre::eyre, Result};
use itertools::Itertools;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tbot::{
    contexts::{methods::Message, Command},
    types::{file, message::Kind, Message as Msg},
};
use tokio::sync::Mutex;
use tracing::error;

use crate::ResultExt;

type State = Arc<(Client, BTreeSet<String>, Mutex<HashMap<String, file::Id>>)>;

pub async fn doggo_handler(ctx: Arc<Command>, state: State) {
    let (client, all_breeds, id_map) = &*state;
    let queried_breed = (!ctx.text.value.is_empty()).then(|| ctx.text.value.clone());
    match query_api(client, all_breeds, queried_breed).await {
        Ok(QueryResult::Doggo { url }) => {
            use tbot::types::input_file::Photo;
            if let Some(id) = id_map.lock().await.get(&url).cloned() {
                ctx.send_photo(Photo::with_id(id))
                    .call()
                    .await
                    .log_err_msg("error sending image");
                return;
            }
            match ctx.send_photo(Photo::with_url(&url)).call().await {
                Ok(Msg {
                    kind: Kind::Photo { photo, .. },
                    ..
                }) => {
                    if let Some(photo) = photo.into_iter().next() {
                        id_map.lock().await.insert(url, photo.file_id);
                    } else {
                        error!(?url, "Mysteriously didn't get a file id");
                    }
                }
                Err(error) => error!(?error, "error sending doggo"),
                _ => unreachable!("non-photo from SendPhoto"),
            };
        }
        Ok(QueryResult::Error { msg }) => {
            ctx.send_message_in_reply(msg).call().await.log_err();
        }
        Err(error) => {
            error!(?error, "error fetching doggo");
        }
    };
}
pub async fn breeds_handler(ctx: Arc<Command>, state: State) {
    let all_breeds = state.1.iter().join("\n");
    let text = format!("Available doggo breeds:\n\n{}", all_breeds);
    ctx.send_message_in_reply(text).call().await.log_err();
}

pub async fn fetch_breeds(client: &Client) -> Result<BTreeSet<String>> {
    #[derive(Debug, serde::Deserialize)]
    struct BreedResponse {
        status: String,
        message: Value,
    }
    let resp: BreedResponse = client
        .get("https://dog.ceo/api/breeds/list/all")
        .send()
        .await?
        .json()
        .await?;
    if resp.status != "success" {
        let msg = if let Some(error) = resp.message.as_str() {
            eyre!("Error fetching breed list: {:?}", error)
        } else {
            eyre!("Unknown error fetching breed list: {:?}", resp)
        };
        return Err(msg);
    }
    let breeds: BTreeMap<String, BTreeSet<String>> = serde_json::from_value(resp.message)?;
    Ok(breeds
        .into_iter()
        .flat_map(|(breed, sub_breeds)| {
            if !sub_breeds.is_empty() {
                sub_breeds
                    .into_iter()
                    .map(|sb| sb + " " + &breed)
                    .collect::<Vec<_>>()
                    .into_iter()
            } else {
                vec![breed].into_iter()
            }
        })
        .collect())
}

enum QueryResult {
    Doggo { url: String },
    Error { msg: String },
}
async fn query_api(
    client: &Client,
    all_breeds: &BTreeSet<String>,
    queried_breed: Option<String>,
) -> Result<QueryResult> {
    let url = if let Some(breed) = &queried_breed {
        use std::borrow::Cow;
        let breed = if breed.contains(' ') {
            Cow::Owned(breed.rsplit(' ').collect::<Vec<_>>().join("/"))
        } else {
            Cow::Borrowed(breed)
        };
        format!(
            "https://dog.ceo/api/breed/{}/images/random",
            breed.to_ascii_lowercase()
        )
    } else {
        "https://dog.ceo/api/breeds/image/random".into()
    };
    #[derive(Debug, serde::Deserialize)]
    struct DoggoQueryResult {
        message: String,
        status: String,
    }
    let resp = client.get(&url).send().await?;
    let status = resp.status();
    let resp: DoggoQueryResult = resp.json().await?;
    match resp.status.as_str() {
        "success" => Ok(QueryResult::Doggo { url: resp.message }),
        _ => {
            error!(?resp, ?status, "got non-success response from doggo API");
            let msg = if let (Some(breed), StatusCode::NOT_FOUND) = (queried_breed, status) {
                let breed_results: Vec<_> = all_breeds
                    .iter()
                    .filter(|b| b.contains(&breed.to_ascii_lowercase()))
                    .collect();
                if !breed_results.is_empty() {
                    let result_texts = breed_results
                        .into_iter()
                        .map(|v| v.to_owned())
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!("Did you mean any of these:\n{}", result_texts)
                } else {
                    "Breed not found!".into()
                }
            } else {
                resp.message
            };
            Ok(QueryResult::Error { msg })
        }
    }
}
