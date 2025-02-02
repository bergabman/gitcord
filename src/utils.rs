use std::{collections::HashSet, io::Write, sync::Arc};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serenity::{
    all::{ChannelId, Context, CreateEmbed, CreateMessage, RoleId, ShardManager},
    prelude::TypeMapKey,
};
use url::Url;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    // type Value = Arc<Mutex<ShardManager>>;
    type Value = Arc<ShardManager>;
}

impl TypeMapKey for Config {
    type Value = Config;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub bot_token: String,
    pub github_token: String,
    pub report_channel_ID: u64,
    pub last_checked: String,
    pub mentees: Vec<Mentee>,
    pub seen_commit_hashes: HashSet<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mentee {
    pub mentor: String,
    pub discord_username: String,
    pub git_username:String,
}

pub fn loadconfig() -> Result<Config> {
    let configjson = std::fs::read_to_string("botconfig.json")?;
    let config = serde_json::from_str::<Config>(&configjson)?;
    Ok(config)
}

pub fn saveconfig(config: &Config) -> Result<()> {
    let json = serde_json::to_string_pretty(config)?;
    let mut configjson = std::fs::OpenOptions::new().write(true).open("botconfig.json")?;
    configjson.write_all(json.as_bytes())?;
    Ok(())
}

pub async fn log_system_load(ctx: &Context) {
    let cpu_load = sys_info::loadavg().unwrap();
    let mem_use = sys_info::mem_info().unwrap();
    let embed = CreateEmbed::new()
        .title("System Resource Load")
        .field(
            "CPU Load Average",
            format!("{:.2}%", cpu_load.one * 10.0),
            false,
        )
        .field(
            "Memory Usage",
            format!(
                "{:.2} MB Free out of {:.2} MB",
                mem_use.free as f32 / 1000.0,
                mem_use.total as f32 / 1000.0
            ),
            false,
        );
    let builder = CreateMessage::new().embed(embed);
    let message = ChannelId::new(1206302338499878982)
        .send_message(&ctx, builder)
        .await;
    if let Err(why) = message {
        eprintln!("Error sending message: {why:?}");
    };
}

pub fn check_url(url: &str) -> Result<(String, String)> {
    let url_parts = Url::parse(url)?;
    let owner_repo = url_parts.path().replace(".git", "").to_string();
    let owner_repo_parts = owner_repo.split("/").collect::<Vec<&str>>();
    Ok((owner_repo_parts[1].into(), owner_repo_parts[2].into()))
}