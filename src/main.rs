use std::{collections::HashMap, sync::Arc};

use color_eyre::{
    eyre::{eyre, WrapErr},
    Report, Result,
};
use reqwest::Client;
use serde_json::Value;
use tbot::{
    contexts::{methods::Message, Command},
    types::file,
};
use tokio::sync::Mutex;
use tracing::{error, info};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

use bot::GodfishBotBuilder;

mod bot;
mod doggo;
mod flausch;
mod love_test;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    color_eyre::install()?;
    dotenv::dotenv().map(drop).unwrap_or_else(|error| {
        info!(?error, "Failed to load .env; continuing with defaults");
    });

    info!("Starting up bot");
    info!("Setting up commands...");
    let mut bot = GodfishBotBuilder::new()
        .rand_text(
            "explode",
            "/explode [target]",
            "Explode [at someone]",
            "explode.txt",
            Some("explode_single.txt"),
        )
        .rand_text("kiss", "/kiss <target>", "Kiss someone", "kiss.txt", None)
        .rand_text("hug", "/hug <target>", "Hug someone", "hugs.txt", None)
        .rand_img("star", "Get a star", "stars/")
        .audio("bitchwhere", &["bitchwhere.mp3"], None)
        .audio("boahey", &["boahey.ogg"], None)
        .audio("eeyup", &["eeyup.opus"], None)
        .audio("eghugh", &["eghughehhhh.mp3"], None)
        .audio("arsam", &["failure.mp3"], Some("YOU FUCKING FAILURE!"))
        .audio("gasp", &["gasp.opus"], None)
        .audio("heuldoch", &["heuldoch.ogg"], None)
        .audio("okay", &["okay.mp3"], None)
        .audio("truthahn", &["truthahn.ogg"], None)
        .audio("ululu", &["ululu.opus"], None)
        .audio("property", &["property.mp3", "property2.mp3"], None)
        .audio("sixpack", &["sixpack.mp3"], None)
        .audio("sexy", &["sexy.mp3"], None)
        .audio("ayaya", &["ayaya1.mp3", "ayaya2.mp3"], None)
        .audio("nigerundayo", &["nigerundayo.mp3"], None)
        .audio("wow", &["wow.mp3", "wow2.mp3", "wow3.mp3"], None)
        .audio("nneville", &["nneville.mp3"], None)
        .audio("saido", &["saidochesto.mp3"], None)
        .audio("ohyeah", &["ohyeah1.mp3", "ohyeah2.mp3"], None)
        .audio("damedame", &["damedame.mp3"], None)
        .audio("yeah", &["yeah.mp3"], None)
        .audio("dingdong", &["dingdong.mp3"], None)
        .audio("horn", &["horn.mp3"], None)
        .audio("nani", &["nani.mp3"], None)
        .audio("explosion", &["explosion1.mp3", "explosion2.mp3"], None)
        .audio("french", &["french.mp3"], None)
        .audio("chinese", &["chinese.mp3"], None)
        .audio("friendship", &["friendship.mp3"], None)
        .audio("selfie", &["selfie.mp3"], None)
        .audio("baum", &["baum.mp3"], None)
        .audio("dundundun", &["dundundun.mp3"], None)
        .audio("sasgay", &["sasuke.mp3"], None)
        .audio("naruto", &["naruto.mp3"], None)
        .audio("alpakistan", &["oreimo.mp3"], None)
        .audio("pling", &["pling.mp3"], None)
        .audio("laugh", &["laugh.mp3"], None)
        .audio("power", &["woahohohah.mp3"], None)
        .audio("zawarudo", &["zawarudo.mp3"], None)
        .audio("wah", &["wah.mp3"], None)
        .audio("checkmate", &["checkometo.mp3"], None)
        .audio("nintendo", &["daisy.mp3"], None)
        .audio("heal", &["heal.mp3"], None)
        .audio("mammamia", &["mammamia.mp3"], None)
        .audio("morioh", &["morioh.mp3"], None)
        .audio("youready", &["youready.mp3"], None)
        .audio("herewego", &["herewego.mp3"], None)
        .audio("again", &["again.mp3"], None)
        .audio("uuuh", &["uuuh.mp3"], None)
        .audio("fbi", &["fbi.mp3"], None)
        .audio("rivalun", &["rivalun.mp3"], None)
        .audio("confusion", &["iamconfusion.mp3"], None)
        .audio("like", &["leonard.mp3"], None)
        .audio("hiii", &["HIIII.wav"], None)
        .audio("yay", &["YAY.wav"], None)
        .audio("piedro", &["piedro.mp3"], None)
        .audio("lvlup", &["lvlup.mp3"], None)
        .image("bully", "bully.jpg")
        .image("bully2", "bully2.jpg")
        .image("spicken", "spicken.jpg")
        .image("frenz", "frenz.jpg")
        .image("teacher", "teacher.jpg")
        .image("bullyback", "bullyback.jpg")
        .image("tease", "tease.jpg")
        .image("flashbacks", "flashback.jpg")
        .other(
            "cn",
            "Get a fact about Chuck Norris. (Powered by http://www.icndb.com)",
            None,
        )
        .other(
            "trump",
            "Get a Donald Trump quote. Powered by https://whatdoestrumpthink.com",
            None,
        )
        .other(
            "dadjoke",
            "Get a random dad joke from https://icanhazdadjoke.com/api",
            None,
        )
        .other(
            "catfact",
            "Get a random cat fact from https://cat-fact.herokuapp.com",
            None,
        )
        .other(
            "funfact",
            "Get a useless fact from https://uselessfacts.jsph.pl",
            None,
        )
        .other(
            "doggo",
            "Get a random doggo from teh interwebs (may be filtered by breed)",
            Some("/doggo [breed]"),
        )
        .other(
            "testlove",
            "Test compatibility based on names. Totally scientifically correct!",
            Some(love_test::USAGE),
        )
        .other("flausch", "Get a fluffy bunny gif", None)
        .build(tbot::Bot::from_env("BOT_TOKEN"))
        .await?;
    info!("Registering custom commands...");
    bot.command("testlove", love_test::handler);
    let client = Client::new();
    let mut bot = bot.into_stateful(client.clone());
    bot.commands(["cn", "chucknorris"], |ctx, client| async move {
        simple_api(
            ctx,
            &client,
            "http://api.icndb.com/jokes/random?escape=javascript",
            "joke",
            Some("value"),
        )
        .await;
    });
    bot.command("trump", |ctx, client| async move {
        simple_api(
            ctx,
            &client,
            "https://api.whatdoestrumpthink.com/api/v1/quotes/random",
            "message",
            None,
        )
        .await;
    });
    bot.command("catfact", |ctx, client| async move {
        simple_api(
            ctx,
            &client,
            "https://cat-fact.herokuapp.com/facts/random",
            "text",
            None,
        )
        .await;
    });
    bot.command("funfact", |ctx, client| async move {
        simple_api(
            ctx,
            &client,
            "https://uselessfacts.jsph.pl/random.json?language=en",
            "text",
            None,
        )
        .await;
    });
    bot.command("dadjoke", |ctx, client| async move {
        let msg = match fetch_dad_joke(&client).await {
            Ok(msg) => msg,
            Err(error) => {
                error!(?error, "error during API request");
                error.to_string()
            }
        };
        ctx.send_message(msg).call().await.log_err();
    });
    // Doggo command
    let breeds = doggo::fetch_breeds(&client).await.unwrap_or_else(|error| {
        error!(?error, "Error loading doggo breeds");
        error!("Using empty breed list");
        Default::default()
    });
    let img_id_map = Mutex::new(HashMap::<String, file::Id>::new());
    let mut bot = bot.with_other_state((client.clone(), breeds, img_id_map));
    bot.command("doggo", doggo::doggo_handler);
    bot.command("breeds", doggo::breeds_handler);
    // Flausch command
    let img_id_map = Mutex::new(HashMap::<String, file::Id>::new());
    let mut bot = bot.with_other_state((client, img_id_map));
    bot.command("flausch", flausch::handler);
    info!("Starting event loop...");
    tokio::select! {
        res = bot.polling().start() => { res.unwrap(); }
        _ = tokio::signal::ctrl_c() => { info!("Ctrl-C received"); }
    };
    Ok(())
}

trait ResultExt {
    fn log_err(self);
    fn log_err_msg(self, msg: &'static str);
}
impl<T, E> ResultExt for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn log_err(self) {
        if let Err(error) = self {
            let error: Report = error.into();
            error!(?error, "error occured");
        }
    }
    fn log_err_msg(self, msg: &'static str) {
        if let Err(error) = self.wrap_err(msg) {
            error!(?error, "error occured");
        }
    }
}

async fn simple_api(
    ctx: Arc<Command>,
    client: &Client,
    url: &str,
    field: &str,
    outer_field: Option<&str>,
) {
    let res = fetch_json_extract_field(client, url, field, outer_field).await;
    let msg = match res {
        Ok(msg) => msg,
        Err(error) => {
            error!(?error, "error during API request");
            error.to_string()
        }
    };
    ctx.send_message(msg).call().await.log_err();
}
async fn fetch_json_extract_field(
    client: &Client,
    url: &str,
    field: &str,
    outer_field: Option<&str>,
) -> Result<String> {
    let mut json = client.get(url).send().await?.json::<Value>().await?;
    if let Some(outer_field) = outer_field {
        json = json
            .get(outer_field)
            .ok_or_else(|| {
                eyre!(
                    "outer_field not found: {:?} (json: {:?})",
                    outer_field,
                    json
                )
            })?
            .to_owned();
    }
    let val = json
        .get(field)
        .ok_or_else(|| eyre!("json_field not found: {:?} (json: {:?})", field, json))?;
    let val = val
        .as_str()
        .ok_or_else(|| eyre!("type error: expected string, found value: {:?}", val))?
        .to_string();
    Ok(val)
}

async fn fetch_dad_joke(client: &Client) -> Result<String> {
    let json = client
        .get("https://icanhazdadjoke.com/")
        .header("Accept", "application/json")
        .header(
            "User-Agent",
            "godfishbot-ng (https://github.com/Follpvosten/godfishbot-ng)",
        )
        .send()
        .await?
        .json::<Value>()
        .await?;
    json.get("joke")
        .ok_or_else(|| eyre!("joke field missing: {:?}", json))?
        .as_str()
        .ok_or_else(|| eyre!("joke field not a string, wtf"))
        .map(str::to_string)
}
