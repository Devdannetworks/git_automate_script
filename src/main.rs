use git2::{Repository, Signature};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, AUTHORIZATION};
use serde_json::json;
use clap::{Arg, Command};
use std::{fs, path::Path, error::Error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("GitHub Repo Manager")
        .version("1.0")
        .author("Your Name")
        .about("CLI tool to create and manage a GitHub repository")
        .arg(
            Arg::new("token")
                .short('t')
                .long("token")
                .value_name("TOKEN")
                .help("Your GitHub personal access token")
                .required(true),
        )
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .value_name("PATH")
                .help("Local path to initialize the repository")
                .required(true),
        )
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .value_name("NAME")
                .help("Name of the GitHub repository to create")
                .required(true),
        )
        .arg(
            Arg::new("description")
                .short('d')
                .long("description")
                .value_name("DESCRIPTION")
                .help("Description of the repository")
                .required(false),
        )
        .arg(
            Arg::new("private")
                .short('r')
                .long("private")
                .help("Create a private repository")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let token = matches.get_one::<String>("token").unwrap();
    let path = matches.get_one::<String>("path").unwrap();
    let name = matches.get_one::<String>("name").unwrap();
    let description = matches.get_one::<String>("description");
    let is_private = matches.get_flag("private");

    fs::create_dir_all(path)?;
    let repo = Repository::init(path)?;
    fs::write(Path::new(path).join("README.md"), "# Initial Commit")?;

    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;
    let signature = Signature::now("Devdannetworks", "officialdevduncan@gmail.com")?;
    let head = repo.head().ok().and_then(|h| h.target());
    let parents = if let Some(oid) = head {
        if let Ok(commit) = repo.find_commit(oid) {
            vec![commit] // Store as owned values
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &parent_refs,
    )?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token))?);
    headers.insert("User-Agent", HeaderValue::from_static("GitHub-Repo-Manager"));

    let body = json!({
        "name": name,
        "description": description.unwrap_or(&String::from("")),
        "private": is_private,
    });

    let response = client
        .post("https://api.github.com/user/repos")
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Repository '{}' created successfully on GitHub!", name);
    } else if response.status() == 422 {
        println!("Repo already exists skipping this step....");
    }
    else {
        eprintln!("Failed to create repository: {}", response.text().await?);
        return Ok(());
    }

    let remote_url = format!("https://{}@github.com/devdannetworks/{}.git", token, name);
    println!("remote url: {}", remote_url);

    // Check if the remote 'origin' exists
    match repo.find_remote("origin") {
        Ok(_) => {
            // If the remote exists, skip adding it
            println!("Remote 'origin' already exists. Skipping addition.");
        }
        Err(_) => {
            // If the remote doesn't exist, add it
            repo.remote("origin", &remote_url)?;
            println!("Remote 'origin' added.");
        }
    }

        // Check if the 'main' branch exists, and create it if it doesn't
        match repo.find_branch("main", git2::BranchType::Local) {
            Ok(_) => {
                // If the 'main' branch exists, skip creating it
                println!("Branch 'main' already exists. Skipping creation.");
            }
            Err(_) => {
                // If the 'main' branch doesn't exist, create it
                let _ = repo.branch("main", &repo.head().unwrap().peel_to_commit()?, false)?;
                println!("Branch 'main' created.");
            }
        }

    // Push the commit to GitHub
    let mut remote = repo.find_remote("origin")?;
    let _ = remote.push(&["refs/heads/main:refs/heads/main"], None)?;

    println!("Successfully pushed local repository to GitHub!");

    Ok(())
}
