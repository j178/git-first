use anyhow::Result;
use octocrab::OctocrabBuilder;

use git_first::get_first_commit;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (owner, repo) = (&args[1], &args[2]);

    let crab = OctocrabBuilder::new()
        .personal_token(std::env::var("GITHUB_TOKEN").unwrap())
        .build()
        .unwrap();

    let first_commit_url = get_first_commit(&crab, owner, repo).await?;
    println!("{}", first_commit_url);

    Ok(())
}
