use std::collections::HashSet;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::offset::Utc;
use commands::git_check;
use serenity::all::Http;
use serenity::async_trait;
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::gateway::ActivityData;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::*;

use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod utils;
use utils::{loadconfig, saveconfig, Config, ShardManagerContainer};

mod commands;

struct Handler {
    is_loop_running: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.starts_with("--ping") {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                info!("Error sending message: {why:?}");
            }
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    // We use the cache_ready event to start checking repos
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        println!("Cache built successfully!");
        let ctx = Arc::new(ctx);
        if !self.is_loop_running.load(Ordering::Relaxed) {
            // We have to clone the Arc, as it gets moved into the new thread.
            tokio::spawn(async move {
                loop {
                    let git_result = git_check(&ctx).await;
                    if let Some(hashes) = git_result {
                        let mut data = ctx.data.write().await;
                        let mut current_config = data.get_mut::<Config>().unwrap();
                        // println!("current config {:?}", current_config);
                        current_config.seen_commit_hashes.extend(hashes);
                        current_config.last_checked = Utc::now().to_string();
                        saveconfig(&current_config).unwrap();
                    }
                    // let data = ctx.data.read().await;
                    // println!("{:?}", data.get::<Config>());
                    // set_activity_to_current_time(&ctx2);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            });

            // Now that the loop is running, we set the bool to true
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }
}

#[tokio::main]
async fn main() {
    let config: Config = loadconfig().expect("Can't load config file botconfig.toml");
    println!("Botconfig loaded {:?}", &config);

    // Initialize the logger to use environment variables. `RUST_LOG`
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start logging");

    // let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new(&config.bot_token);

    // Fetch bot's owners and id
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.unwrap().id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // let intents = GatewayIntents::GUILD_MESSAGES
    //     | GatewayIntents::DIRECT_MESSAGES
    //     | GatewayIntents::GUILDS
    //     | GatewayIntents::MESSAGE_CONTENT;
    let intents = GatewayIntents::all();
    let mut client = Client::builder(&config.bot_token, intents)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
        })
        .await
        .expect("Error creating client");

    // Add config data to bot context so it becomes accessible throughout the bot.
    // It is in a separate scope to drop the mutable reference right after we add the config.
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<Config>(config);
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        warn!("Client error: {why:?}");
    }
}
