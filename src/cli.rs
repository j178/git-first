use anyhow::{bail, Result};
use octocrab::OctocrabBuilder;

use git_first::get_first_commit;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().len() != 2 {
        println!("Usage: git-first <owner>/<repo>");
        bail!("Invalid arguments")
    }

    let arg = std::env::args().nth(1).unwrap();
    let full_name = arg
        .trim_start_matches("https://github.com/")
        .trim_matches('/')
        .split('/')
        .collect::<Vec<_>>();

    if full_name.len() != 2 {
        println!("Usage: git-first <owner>/<repo>");
        bail!("Invalid arguments")
    }

    let (owner, repo) = (full_name[0], full_name[1]);

    let crab = OctocrabBuilder::new()
        .personal_token(std::env::var("GITHUB_TOKEN").unwrap())
        .build()
        .unwrap();

    let first_commit_url = get_first_commit(&crab, owner, repo).await?;
    println!("{}", first_commit_url);

    Ok(())
}
