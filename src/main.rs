use std::fs;
use serde::Deserialize;
use serde::Serialize;
use poise::serenity_prelude as serenity;
use songbird::SerenityInit;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::input::YoutubeDl;
use reqwest::{Client as HttpClient, Url};
use serenity::all::CreateAttachment;
use serenity::async_trait;
use tokio::fs::File;
use tokio::process::Command;
use xxhash_rust::xxh3::xxh3_64;
use std::path::Path;

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    token: String,
}

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

struct HttpKey;
impl serenity::prelude::TypeMapKey for HttpKey {
    type Value = HttpClient;
}

fn read_token() -> Config {
    let config_data = fs::read_to_string("config.json").expect("Unable to read config file");
    serde_json::from_str(&config_data).expect("Invalid JSON in config file")
}

async fn get_yt_dlp_filename(url: &str) -> Result<String, Error> {
    let output = Command::new("yt-dlp")
        .arg("--print")
        .arg("filename")
        .arg("--no-warnings")
        .arg(url)
        .output()
        .await?;
    if !output.status.success() {
        return Err("yt-dlp failed to get filename".into());
    }

    let raw_filename = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let hash = xxh3_64(raw_filename.as_bytes());
    let ext = Path::new(&raw_filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let filename = if ext.is_empty() {
        format!("{:x}", hash)
    } else {
        format!("{:x}.{}", hash, ext)
    };
    Ok(filename)
}

#[poise::command(slash_command, prefix_command)]
async fn video(ctx: Context<'_>, #[description = "video"] url: String) -> Result<(), Error> {
    ctx.defer().await?;

    match Url::parse(url.as_str()) {
        Ok(url) => {
            println!("url is valid {}", url.as_str());
        }
        Err(e) => {
            eprintln!("url is invalid {}", e);
            ctx.say("url is invalid").await?;
            return Ok(());
        }
    }

    let filename: String = get_yt_dlp_filename(&url).await?;

    println!("filename is {}", &filename);

    let output = Command::new("yt-dlp")
        .arg("--output")
        .arg(&filename)
        .arg(url.as_str())
        .arg("--max-filesize")
        .arg("20M")
        .arg("--force-overwrite")
        .output()
        .await
        .expect("failed to execute");

    if !output.status.success() {
        eprintln!("yt-dlp failed with status: {}", output.status);
        ctx.say("file too large :'-(").await?;
        return Ok(());
    };

    let file = match File::open(&filename).await {
        Ok(file) => file,
        Err(e) => {
            eprintln!("failed to open file: {}", e);
            ctx.say("file too large :'-(").await?;
            return Ok(());
        }
    };

    // get file size
    let metadata = match file.metadata().await {
        Ok(metadata) => metadata,
        Err(e) => {
            eprintln!("failed to get file metadata: {}", e);
            return Ok(());
        }
    };

    let file_size = metadata.len();
    if file_size > 10 * 1024 * 1024 {
        ctx.say("file is too large :'-(").await?;
        return Ok(());
    }

    println!("file is good!");

    let attachment = CreateAttachment::file(&file, &filename).await?;
    ctx.send(poise::CreateReply::default()
            .content("")
            .attachment(attachment)
    ).await?;

    // remove file
    let result = fs::remove_file(&filename);
    if let Err(e) = result {
        eprintln!("failed to remove file: {}", e);
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn play(ctx: Context<'_>, #[description = "url"] url: String) -> Result<(), Error> {
    ctx.say(format!("attempting to play: {}", url)).await?;
    
    let guild_id = match ctx.guild_id() {
        Some(guild_id) => guild_id,
        None => {
            ctx.say("this command can only be used in servers").await?;
            return Ok(());
        }
    };

    let user_id = ctx.author().id;
    let channel_id = match ctx.guild() {
        Some(guild) => guild.voice_states.get(&user_id).and_then(|vs| vs.channel_id),
        None => None,
    };

    let channel_id = match channel_id {
        Some(channel) => channel,
        None => {
            ctx.say("you are not in a voice channel").await?;
            return Ok(());
        }
    };

    let do_search = !url.starts_with("http");

    let http_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("songbird voice client placed in at initialization.")
        .clone();

    match manager.join(guild_id, channel_id).await {
        Ok(handler_lock) => {
            let mut handler = handler_lock.lock().await;
            handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);

            let src = if do_search {
                YoutubeDl::new_search(http_client, url)
            } else {
                YoutubeDl::new(http_client, url)
            };

            let _ = handler.play_input(src.into());
            ctx.say("playing song").await?;
        }
        Err(e) => {
            ctx.say(format!("failed to join voice channel: {}", e)).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // login with a bot token from config.json file
    let config_data = read_token();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![video(), play()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::Client::builder(&config_data.token, serenity::GatewayIntents::non_privileged())
        .framework(framework)
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new())
        .await;

    client.unwrap().start().await.unwrap();
}
