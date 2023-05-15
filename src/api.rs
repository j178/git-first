use anyhow::Result;
use log::info;
use octocrab::OctocrabBuilder;
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
                .body(Body::from(format!("Error: {}", e)))?),
        }
    };

    run(h).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    info!("Request: {:?}", req);

    let url = req.uri().to_string();
    let paths: Vec<&str> = url.split('/').collect();
    let (owner, repo) = (paths[paths.len() - 2], paths[paths.len() - 1]);
    let crab = OctocrabBuilder::new()
        .personal_token(std::env::var("GITHUB_TOKEN").unwrap())
        .build()
        .unwrap();

    let first_commit_url = get_first_commit(&crab, owner, repo).await?;
    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", first_commit_url)
        .body(().into())?)
}
