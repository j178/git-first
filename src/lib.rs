use anyhow::{anyhow, Result};
use octocrab::Octocrab;
use serde::Deserialize;
use serde_json::{json, Value};

const REPO_FIRST_COMMIT_QUERY: &str = "
query($owner: String!, $repo: String!, $after: String) {
    repository(owner: $owner, name: $repo) {
      defaultBranchRef {
        target {
          ... on Commit {
            history(first: 1, after: $after) {
              totalCount
              pageInfo {
                hasNextPage
                endCursor
              }
              edges {
                node {
                  commitUrl
                }
              }
            }
          }
        }
      }
    }
}";

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    has_next_page: bool,
    end_cursor: String,
}

pub async fn get_first_commit(crab: &Octocrab, owner: &str, repo: &str) -> Result<String> {
    let mut body = json!( {
        "query": REPO_FIRST_COMMIT_QUERY,
        "variables": {
            "owner": owner,
            "repo": repo,
        }
    });

    let mut resp: Value = crab.post("/graphql", Some(&body)).await?;
    let total_count = resp
        .pointer("/data/repository/defaultBranchRef/target/history/totalCount")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow!("total count not found"))? as usize;

    let page_info: PageInfo = resp
        .pointer("/data/repository/defaultBranchRef/target/history/pageInfo")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| anyhow!("page info not found"))?;

    if page_info.has_next_page {
        let mut end_cursor = page_info.end_cursor.split(' ').next().unwrap().to_string();
        end_cursor = format!("{} {}", end_cursor, total_count - 2);

        body["variables"]["after"] = json!(end_cursor);
        resp = crab.post("/graphql", Some(&body)).await?;
    }

    let first_commit_url = resp
        .pointer("/data/repository/defaultBranchRef/target/history/edges/0/node/commitUrl")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("first commit url not found"))?;

    Ok(first_commit_url.to_string())
}
