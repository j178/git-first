use anyhow::Result;
use log::info;
use octocrab::OctocrabBuilder;
use redis::AsyncCommands;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};

use git_first::get_first_commit;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let h = |req: Request| async move {
        let res = handler(req).await;
        match res {
            Ok(res) => Ok(res),
            Err(e) => Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(Body::from(format!("Error: {}", e)))?),
        }
    };

    run(h).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    let paths: Vec<&str> = req.uri().path().trim_matches('/').split('/').collect();
    if paths.len() != 2 {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "text/plain")
            .body(Body::from(
                "Try https://git-first-commit.vercel.app/{owner}/{repo}\n",
            ))?);
    }

    let (owner, repo) = (paths[paths.len() - 2], paths[paths.len() - 1]);
    let cache_key = format!("{}/{}", owner, repo);

    let kv_url = std::env::var("KV_URL")?.replace("redis://", "rediss://");
    let mut redis_conn = redis::Client::open(kv_url)?.get_async_connection().await?;

    if let Ok(first_commit_url) = redis_conn.get::<_, String>(&cache_key).await {
        info!("Cache hit: {}", &cache_key);
        return Ok(Response::builder()
            .status(StatusCode::MOVED_PERMANENTLY)
            .header("Location", first_commit_url)
            .body(().into())?);
    }

    info!("Cache miss: {}", &cache_key);
    let crab = OctocrabBuilder::new()
        .personal_token(std::env::var("GITHUB_TOKEN")?)
        .build()?;

    let first_commit_url = get_first_commit(&crab, owner, repo).await?;

    redis_conn.set(&cache_key, &first_commit_url).await?;

    Ok(Response::builder()
        .status(StatusCode::MOVED_PERMANENTLY)
        .header("Location", &first_commit_url)
        .body(().into())?)
}
