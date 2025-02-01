use std::{collections::HashSet, str::FromStr};

use chrono::{DateTime, Utc};
use octocrab::Octocrab;
use serenity::all::{ChannelId, Context, Message};

use crate::utils::{check_url, Config};

pub async fn git_check(ctx: &Context) -> Option<HashSet<String>> {

    let data = ctx.data.read().await;
    let current_config = data.get::<Config>()?.clone();
    let mut return_hashes = HashSet::new();
    let octocrab = Octocrab::builder().personal_token(current_config.github_token.clone()).build().unwrap();

    for student in current_config.students {
        let link_parts = check_url(&student.git_repo_url).unwrap();
        let repo = octocrab.repos(link_parts.0, link_parts.1);
        let commits = repo.list_commits().since(DateTime::from_str(&current_config.last_checked).unwrap()).send().await.unwrap();
        for commit in commits {
            if !current_config.seen_commit_hashes.contains(&commit.sha) {
                return_hashes.insert(commit.sha.clone());
                let author = commit.commit.author.unwrap();

                ChannelId::new(current_config.report_channel_ID).say(
                    &ctx.http, 
                    format!("{} | **Date:** {} **Author:** {} **Commit message:** {} <{}>",
                        student.mentor, author.date.unwrap(), author.name, commit.commit.message, commit.html_url
                    )
                ).await.unwrap();
            }
        }
    }

    if return_hashes.len()>0 {
        Some(return_hashes)
    } else {
        None
    }
}