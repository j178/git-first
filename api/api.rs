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
    let path = req.uri().path().trim_matches('/');

    // Serve index page at root path
    if path.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Body::from(include_str!("index.html")))?);
    }

    let paths: Vec<&str> = path.split('/').collect();
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
    let mut redis_conn = redis::Client::open(kv_url)?
        .get_multiplexed_async_connection()
        .await?;

    let is_xhr = req
        .headers()
        .get("X-Requested-With")
        .is_some_and(|v| v == "XMLHttpRequest");

    if let Ok(first_commit_url) = redis_conn.get::<_, String>(&cache_key).await {
        info!("Cache hit: {}", &cache_key);

        if is_xhr {
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(format!("{{\"url\":\"{}\"}}", first_commit_url)))?);
        }

        return Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", first_commit_url)
            .body(Body::Empty)?);
    }

    info!("Cache miss: {}", &cache_key);
    let crab = OctocrabBuilder::new()
        .personal_token(std::env::var("GITHUB_TOKEN")?)
        .build()?;

    let first_commit_url = get_first_commit(&crab, owner, repo).await?;

    let _: () = redis_conn.set(&cache_key, &first_commit_url).await?;

    if is_xhr {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(format!("{{\"url\":\"{}\"}}", first_commit_url)))?);
    }

    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", &first_commit_url)
        .body(Body::Empty)?)
}
