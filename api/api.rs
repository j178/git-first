use anyhow::Result;
use log::{error, info};
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

    let is_xhr = req
        .headers()
        .get("X-Requested-With")
        .is_some_and(|v| v == "XMLHttpRequest");

    let (owner, repo) = (paths[paths.len() - 2], paths[paths.len() - 1]);
    let cache_key = format!("{}/{}", owner, repo);

    let kv_url = std::env::var("KV_URL")?.replace("redis://", "rediss://");

    // Create Redis connection once
    let mut redis_conn = match redis::Client::open(kv_url)?
        .get_multiplexed_async_connection()
        .await
    {
        Ok(conn) => Some(conn),
        Err(e) => {
            error!("Redis connection error: {}", e);
            None
        }
    };

    // Try Redis cache lookup if connection is available
    let cached_url = if let Some(conn) = redis_conn.as_mut() {
        match conn.get::<_, String>(&cache_key).await {
            Ok(url) => {
                info!("Cache hit: {}", &cache_key);
                Some(url)
            }
            Err(e) => {
                error!("Redis get error: {}", e);
                None
            }
        }
    } else {
        None
    };

    if let Some(first_commit_url) = cached_url {
        if is_xhr {
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(format!("{{\"url\":\"{}\"}}", first_commit_url)))?);
        } else {
            return Ok(Response::builder()
                .status(StatusCode::FOUND)
                .header("Location", first_commit_url)
                .body(Body::Empty)?);
        }
    }

    info!("Cache miss: {}", &cache_key);
    let crab = OctocrabBuilder::new()
        .personal_token(std::env::var("GITHUB_TOKEN")?)
        .build()?;

    let first_commit_url = get_first_commit(&crab, owner, repo).await?;

    // Try to cache the result using the existing connection
    if let Some(mut conn) = redis_conn {
        if let Err(e) = conn.set::<_, _, ()>(&cache_key, &first_commit_url).await {
            error!("Failed to cache result: {}", e);
        }
    }

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
