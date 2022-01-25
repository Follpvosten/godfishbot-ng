use std::{
    collections::{BTreeMap, HashMap},
    mem,
    path::{Path, PathBuf},
    sync::Arc,
};

use color_eyre::eyre::{Result, WrapErr};
use rand::{prelude::SliceRandom, thread_rng};
use tbot::{
    contexts::methods::Message,
    types::{
        file,
        input_file::{Photo, Voice},
        message::Kind,
        Message as Msg,
    },
    EventLoop,
};
use tokio::{fs, sync::Mutex};
use tracing::{error, info};

use crate::ResultExt;

struct SoundDef {
    descr: Option<String>,
    files: Vec<&'static Path>,
}
struct RandTextDef {
    file: &'static Path,
    single_file: Option<&'static Path>,
    info: CmdDef,
}
struct RandImgDef {
    descr: String,
    folder: &'static Path,
}
#[derive(Default)]
pub struct GodfishBotBuilder {
    txt_cmds: BTreeMap<&'static str, RandTextDef>,
    sounds: BTreeMap<&'static str, SoundDef>,
    images: BTreeMap<&'static str, &'static Path>,
    img_cmds: BTreeMap<&'static str, RandImgDef>,
    other_cmds: BTreeMap<&'static str, CmdDef>,
}
#[derive(Debug, Clone)]
struct CmdDef {
    usage: Option<String>,
    descr: String,
}
impl CmdDef {
    fn new(descr: impl Into<String>) -> Self {
        CmdDef {
            usage: None,
            descr: descr.into(),
        }
    }
}

impl GodfishBotBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn audio(
        mut self,
        cmd: &'static str,
        files: &[&'static str],
        descr: Option<&'static str>,
    ) -> Self {
        self.sounds.insert(
            cmd,
            SoundDef {
                files: files.iter().map(|f| Path::new(*f)).collect(),
                descr: descr.map(Into::into),
            },
        );
        self
    }
    pub fn image(mut self, cmd: &'static str, file: &'static str) -> Self {
        self.images.insert(cmd, Path::new(file));
        self
    }
    pub fn rand_text(
        mut self,
        cmd: &'static str,
        usage: impl Into<String>,
        descr: impl Into<String>,
        opts_file: &'static str,
        single_file: Option<&'static str>,
    ) -> Self {
        self.txt_cmds.insert(
            cmd,
            RandTextDef {
                file: Path::new(opts_file),
                single_file: single_file.map(Path::new),
                info: CmdDef {
                    usage: Some(usage.into()),
                    descr: descr.into(),
                },
            },
        );
        self
    }
    pub fn rand_img(
        mut self,
        cmd: &'static str,
        descr: impl Into<String>,
        folder: &'static str,
    ) -> Self {
        self.img_cmds.insert(
            cmd,
            RandImgDef {
                descr: descr.into(),
                folder: Path::new(folder),
            },
        );
        self
    }
    /// Used to document other commands in the help message.
    pub fn other(
        mut self,
        cmd: &'static str,
        descr: impl Into<String>,
        usage: Option<&'static str>,
    ) -> Self {
        self.other_cmds.insert(
            cmd,
            CmdDef {
                usage: usage.map(Into::into),
                descr: descr.into(),
            },
        );
        self
    }
    fn build_help(&self) -> (String, HashMap<String, CmdDef>) {
        let mut help = format!(
            "GodfishBot v{}\nAvailable commands:\n\n",
            env!("CARGO_PKG_VERSION")
        );
        let mut cmd_helps = HashMap::new();
        help.push_str("/help - Get this help message\n");
        cmd_helps.insert(
            "help".to_string(),
            CmdDef {
                usage: Some("/help [command]".into()),
                descr: "Offers help for commands (or get a list of commands)".into(),
            },
        );
        help.push_str("\nRandom text commands:\n");
        for (&cmd, def) in self.txt_cmds.iter() {
            let usage = def
                .info
                .usage
                .clone()
                .unwrap_or_else(|| "/".to_string() + cmd);
            help.push_str(&(usage + " - " + &def.info.descr + "\n"));
            cmd_helps.insert(cmd.to_string(), def.info.clone());
        }
        help.push_str("\nSound commands:\n");
        for (&cmd, def) in self.sounds.iter() {
            let help_line = if let Some(descr) = &def.descr {
                "/".to_string() + cmd + " - " + descr + "\n"
            } else {
                "/".to_string() + cmd + "\n"
            };
            help.push_str(&help_line);
            let descr = def.descr.as_deref().unwrap_or("Get a sound effect");
            cmd_helps.insert(cmd.to_string(), CmdDef::new(descr));
        }
        help.push_str("\nImage commands:\n");
        for (&cmd, _) in self.images.iter() {
            help.push_str(&("/".to_string() + cmd + "\n"));
            cmd_helps.insert(cmd.to_string(), CmdDef::new("Get a specific image"));
        }
        help.push_str("\nRandom image commands:\n");
        for (&cmd, def) in self.img_cmds.iter() {
            help.push_str(&("/".to_string() + cmd + " - " + &def.descr + "\n"));
            cmd_helps.insert(cmd.to_string(), CmdDef::new(&def.descr));
        }
        help.push_str("\nOther commands:\n");
        for (&cmd, def) in self.other_cmds.iter() {
            let usage = def.usage.clone().unwrap_or_else(|| "/".to_string() + cmd);
            help.push_str(&(usage.to_string() + " - " + &def.descr + "\n"));
            cmd_helps.insert(cmd.to_string(), def.clone());
        }
        (help, cmd_helps)
    }
    pub async fn build(mut self, bot: tbot::Bot) -> Result<EventLoop> {
        // 1. build help messsage
        info!("Generating help message...");
        let (help_msg, cmd_helps) = self.build_help();
        let help_msg = Arc::new(help_msg);
        // 2. make basic event loop, fetch username, register help command
        let mut bot = bot.event_loop();
        info!("Fetching username...");
        bot.fetch_username()
            .await
            .wrap_err("Error fetching username")?;
        info!("Registering help handler...");
        bot.help(move |ctx| {
            let help = help_msg.clone();
            let cmd_help = cmd_helps.get(&ctx.text.value).cloned();
            async move {
                if ctx.text.value.is_empty() {
                    ctx.send_message_in_reply(help.as_str())
                        .is_web_page_preview_disabled(true)
                        .call()
                        .await
                        .log_err_msg("error sending help message");
                } else {
                    let result = if let Some(cmd_help) = cmd_help {
                        let usage = cmd_help.usage.as_ref().unwrap_or(&ctx.text.value);
                        format!("Usage: {}\n\n{}", usage, cmd_help.descr)
                    } else {
                        "Command not found!".into()
                    };
                    ctx.send_message_in_reply(result)
                        .is_web_page_preview_disabled(true)
                        .call()
                        .await
                        .log_err_msg("error sending help message");
                }
            }
        });
        // 3. register commands which don't need any state
        info!("Registering random text commands...");
        let base = Path::new("res/txt/");
        for (cmd, def) in mem::take(&mut self.txt_cmds) {
            let usage = Arc::new(def.info.usage.clone().unwrap_or_else(|| cmd.to_string()));
            let options = Arc::new(load_file_lines([base, def.file].iter().collect()).await?);
            let options_single = Arc::new(if let Some(file) = def.single_file {
                Some(load_file_lines([base, file].iter().collect()).await?)
            } else {
                None
            });
            bot.command(cmd, move |ctx| {
                let usage = usage.clone();
                let options = options.clone();
                let options_single = options_single.clone();
                async move {
                    let sender = ctx
                        .from
                        .as_ref()
                        .and_then(|from| from.clone().user())
                        .map(|user| user.first_name)
                        .unwrap_or_else(|| "Deine Mudda".into());
                    let result = if !ctx.text.value.is_empty() {
                        random_sentence_at(&options, &sender, &ctx.text.value)
                    } else if options_single.is_some() {
                        random_sentence(&options, &options_single, &sender)
                    } else {
                        String::clone(&usage)
                    };
                    ctx.send_message(result)
                        .call()
                        .await
                        .log_err_msg("error sending message");
                }
            });
        }
        // 4. image commands
        info!("Registering simple image commands...");
        let mut bot = bot.into_stateful(Mutex::new(HashMap::<PathBuf, file::Id>::new()));
        let base = Path::new("res/images/");
        for (cmd, img) in mem::take(&mut self.images) {
            let path: Arc<PathBuf> = Arc::new([base, img].iter().collect());
            bot.command(cmd, move |ctx, state| {
                let path = path.clone();
                let reply_to_id = if let Some(Msg { id, .. }) = &ctx.reply_to {
                    *id
                } else {
                    ctx.message_id
                };
                async move {
                    if let Some(id) = state.lock().await.get(&*path).cloned() {
                        ctx.send_photo(Photo::with_id(id))
                            .in_reply_to(reply_to_id)
                            .call()
                            .await
                            .log_err_msg("error sending image");
                        return;
                    }
                    let bytes = match fs::read(&*path).await {
                        Ok(x) => x,
                        Err(error) => {
                            error!(?error, ?path, "error loading file");
                            return;
                        }
                    };
                    match ctx
                        .send_photo(Photo::with_bytes(bytes))
                        .in_reply_to(reply_to_id)
                        .call()
                        .await
                    {
                        Ok(Msg {
                            kind: Kind::Photo { photo, .. },
                            ..
                        }) => {
                            if let Some(photo) = photo.into_iter().next() {
                                state
                                    .lock()
                                    .await
                                    .insert(PathBuf::clone(&*path), photo.file_id);
                            } else {
                                error!(?path, "Mysteriously didn't get a file id");
                            }
                        }
                        Err(error) => error!(?error, "error sending file"),
                        _ => unreachable!("non-photo from SendPhoto"),
                    };
                }
            });
        }
        info!("Registering random image commands...");
        let base = Path::new("res/");
        for (cmd, def) in mem::take(&mut self.img_cmds) {
            let folder: PathBuf = [base, def.folder].iter().collect();
            let mut stream = fs::read_dir(&folder).await?;
            let mut paths = Vec::new();
            while let Some(entry) = stream.next_entry().await? {
                paths.push(entry.path());
            }
            if paths.is_empty() {
                error!(
                    command = ?cmd,
                    ?folder,
                    "ignoring random image command: no images found"
                );
                continue;
            }
            paths.shrink_to_fit();
            let paths = paths;
            bot.command(cmd, move |ctx, state| {
                let path = paths.choose(&mut thread_rng()).cloned().unwrap();
                async move {
                    if let Some(id) = state.lock().await.get(&path).cloned() {
                        ctx.send_photo(Photo::with_id(id))
                            .call()
                            .await
                            .log_err_msg("error sending image");
                        return;
                    }
                    let bytes = match fs::read(&path).await {
                        Ok(x) => x,
                        Err(error) => {
                            error!(?error, "error loading file");
                            return;
                        }
                    };
                    match ctx.send_photo(Photo::with_bytes(bytes)).call().await {
                        Ok(Msg {
                            kind: Kind::Photo { photo, .. },
                            ..
                        }) => {
                            if let Some(photo) = photo.into_iter().next() {
                                state.lock().await.insert(path, photo.file_id);
                            } else {
                                error!("Mysteriously didn't get a file id");
                            }
                        }
                        Err(error) => error!(?error, "error sending file"),
                        _ => unreachable!("non-photo from SendPhoto"),
                    };
                }
            });
        }
        // 5. Sound commands
        info!("Registering sound commands...");
        let base = Path::new("res/sound/");
        for (cmd, SoundDef { files, .. }) in mem::take(&mut self.sounds) {
            if files.is_empty() {
                error!(command = ?cmd, "skipping sound command: no files specified");
                continue;
            }
            let paths = files
                .into_iter()
                .map(|file| [base, file].iter().collect::<PathBuf>())
                .collect::<Vec<_>>();
            let paths = paths;
            bot.command(cmd, move |ctx, state| {
                let path = paths.choose(&mut thread_rng()).cloned().unwrap();
                async move {
                    if let Some(id) = state.lock().await.get(&path).cloned() {
                        ctx.send_voice(Voice::with_id(id))
                            .call()
                            .await
                            .log_err_msg("error sending image");
                        return;
                    }
                    let bytes = match fs::read(&path).await {
                        Ok(x) => x,
                        Err(error) => {
                            error!(?error, "error loading file");
                            return;
                        }
                    };
                    match ctx.send_voice(Voice::with_bytes(bytes)).call().await {
                        Ok(Msg {
                            kind: Kind::Voice { voice, .. },
                            ..
                        }) => {
                            state.lock().await.insert(path, voice.file_id);
                        }
                        Err(error) => error!(?error, "error sending file"),
                        _ => unreachable!("non-voice from SendVoice"),
                    };
                }
            });
        }
        Ok(bot.into_stateless())
    }
}

#[tracing::instrument]
async fn load_file_lines(path: PathBuf) -> Result<Vec<String>> {
    Ok(fs::read_to_string(path)
        .await?
        .lines()
        .map(str::to_string)
        .collect())
}

pub fn random_sentence(
    options: &[String],
    options_single: &Option<Vec<String>>,
    username: &str,
) -> String {
    if let Some(options) = options_single {
        options
            .choose(&mut thread_rng())
            .cloned()
            .unwrap_or_else(|| "FIXME".into())
            .replace("{0}", username)
    } else {
        random_sentence_at(options, username, "Baumhardt")
    }
}
pub fn random_sentence_at(options: &[String], username: &str, target: &str) -> String {
    options
        .choose(&mut thread_rng())
        .cloned()
        .unwrap_or_else(|| "FIXME".into())
        .replace("{0}", username)
        .replace("{1}", target)
}
