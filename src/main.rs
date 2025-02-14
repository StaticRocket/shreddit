use std::error::Error;

use access_token::new_access_token;
use clap::Parser;
use cli::Config;
use futures_util::{pin_mut, StreamExt};
use reqwest::Client;
use things::Shred;
use tracing::{debug, error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{
    sources::gdpr,
    things::{comment, post, Comment, Friend, Post, SavedComment, SavedPost, ThingType},
};

mod access_token;
mod cli;
mod sources;
mod things;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let config_file = dotenv::from_filename("shreddit.env").ok();
    let config = Config::parse();

    init_tracing();

    match config_file {
        Some(p) => debug!(
            "Loaded environment variables from file: {}",
            p.to_string_lossy()
        ),
        None => debug!("No shreddit.env config file found."),
    }

    let client = Client::new();
    let access_token = match new_access_token(&config, &client).await {
        Ok(token) => token,
        Err(e) => {
            error!("{e}");
            return Err(e.into());
        }
    };

    match &config.gdpr_export_dir {
        Some(export_path) => {
            for thing_type in config.thing_types.iter() {
                info!("Shredding {thing_type:?}...");

                match thing_type {
                    ThingType::Comments => {
                        let comments = gdpr::list::<Comment>(export_path);

                        for comment in comments {
                            comment.shred(&client, &access_token, &config).await;
                        }
                    }

                    ThingType::Friends => {
                        let friends = gdpr::list::<Friend>(export_path);

                        for friend in friends {
                            friend.shred(&client, &access_token, &config).await;
                        }
                    }

                    ThingType::Posts => {
                        let posts = gdpr::list::<Post>(export_path);

                        for post in posts {
                            post.shred(&client, &access_token, &config).await;
                        }
                    }

                    ThingType::SavedPosts => {
                        let saved_posts = gdpr::list::<SavedPost>(export_path);

                        for saved_post in saved_posts {
                            saved_post.shred(&client, &access_token, &config).await;
                        }
                    }

                    ThingType::SavedComments => {
                        let saved_comments = gdpr::list::<SavedComment>(export_path);

                        for saved_comment in saved_comments {
                            saved_comment.shred(&client, &access_token, &config).await;
                        }
                    }
                }

                info!("Completed shredding {thing_type:?}");
            }

            info!("Completed shredding {:?}", config.thing_types);
        }
        None => {
            for thing_type in config.thing_types.iter() {
                info!("Shredding {thing_type:?}...");

                match thing_type {
                    ThingType::Posts => {
                        let posts = post::list(&client, &config).await;
                        pin_mut!(posts);

                        while let Some(post) = posts.next().await {
                            post.shred(&client, &access_token, &config).await;
                        }
                    }

                    ThingType::Comments => {
                        let comments = comment::list(&client, &config).await;
                        pin_mut!(comments);

                        while let Some(comment) = comments.next().await {
                            comment.shred(&client, &access_token, &config).await;
                        }
                    }

                    ThingType::Friends => {
                        error!("Shredding friends based on API is a TODO");
                        todo!();
                    }

                    ThingType::SavedPosts => {
                        error!("Shredding saved posts based on API is a TODO");
                        todo!();
                    }

                    ThingType::SavedComments => {
                        error!("Shredding saved comments based on API is a TODO");
                        todo!();
                    }
                }

                info!("Completed shredding {thing_type:?}");
            }

            info!("Completed shredding {:?}", config.thing_types);
        }
    };

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("shreddit"))
        .unwrap();

    let format = fmt::layer().with_target(false).pretty();

    tracing_subscriber::registry()
        .with(filter)
        .with(format)
        .init();
}
