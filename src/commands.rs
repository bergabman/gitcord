use std::{collections::HashSet, str::FromStr};

use chrono::{DateTime, Utc};
use octocrab::Octocrab;
use serenity::{all::{ChannelId, Context, Message}, model::user};

use crate::utils::{check_url, Config};

pub async fn git_check(ctx: &Context) -> Option<HashSet<String>> {

    let data = ctx.data.read().await;
    let current_config = data.get::<Config>()?.clone();
    let mut return_hashes = HashSet::new();
    let octocrab = Octocrab::builder().personal_token(current_config.github_token.clone()).build().unwrap();
    let last_checked: DateTime<Utc> = DateTime::from_str(&current_config.last_checked).unwrap();
    for mentee in current_config.mentees {
        // let link_parts = check_url(&student.git_repo_url).unwrap();
        // let repo = octocrab.repos(link_parts.0, link_parts.1);
        let user_handler = octocrab.users(mentee.git_username.clone()).repos().send().await.unwrap();
        // let all_repos_by_user = user_handler.repos();
        for repo in &user_handler {
            if repo.updated_at > Some(last_checked) {
                // println!("repo updated since last check");
                // println!("repo name {:?}", repo.name);
                // println!("repo owner {:?}", repo.owner.clone().unwrap().login);
                // println!("repo updated_at {:?}", repo.updated_at);

                let repo = octocrab.repos(mentee.git_username.clone(), repo.name.clone());
                let commits = repo.list_commits().since(last_checked).send().await.unwrap();
                for commit in commits {
                    if !current_config.seen_commit_hashes.contains(&commit.sha) {
                        return_hashes.insert(commit.sha.clone());
                        let author = commit.commit.author.unwrap();
                        // println!("commmit url {:?}", commit.html_url);
        
                        ChannelId::new(current_config.report_channel_ID).say(
                            &ctx.http, 
                            format!("{} | **Date:** {} **Author:** {} **Commit message:** {} <{}>",
                                mentee.mentor, author.date.unwrap(), author.name, commit.commit.message, commit.html_url
                            )
                        ).await.unwrap();
                    }
                }
            }
        }
    }

    if return_hashes.len()>0 {
        Some(return_hashes)
    } else {
        None
    }
}