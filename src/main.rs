use serde::Deserialize;
use serde::Serialize;
use std::fs;
use poise::serenity_prelude as serenity;
use songbird::SerenityInit;

// Event related imports to detect track creation failures.
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};

// To turn user URLs into playable audio, we'll use yt-dlp.
use songbird::input::YoutubeDl;

// YtDl requests need an HTTP client to operate -- we'll create and store our own.
use reqwest::Client as HttpClient;

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    token: String,
}

struct TrackErrorNotifier;

#[serenity::async_trait]
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

#[poise::command(slash_command, prefix_command)]
async fn video(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("here is your video").await?;
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
