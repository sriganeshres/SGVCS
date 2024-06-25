use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{
    fmt::{self, Debug},
    io::Result,
    path::{Path, PathBuf},
};
use tokio::io::AsyncReadExt;
use tokio::{fs, io::AsyncWriteExt};

#[derive(Debug)]
pub struct Sgvcs {
    repo_path: PathBuf,
    objects_path: PathBuf,
    index_path: PathBuf,
    head_path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct IndexData {
    path: String,
    hash: String,
}

#[derive(Serialize, Deserialize)]
struct CommitData {
    message: String,
    time_stamp: String,
    files: Vec<IndexData>,
    parent: String,
}

impl Debug for CommitData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "message: {:?}\ntime_stamp: {:?}\n",
            self.message, self.time_stamp
        )
    }
}

impl Sgvcs {
    pub async fn new_async() -> std::io::Result<Sgvcs> {
        let sgvcs: Sgvcs = Sgvcs::new();
        sgvcs.init().await?;
        Ok(sgvcs)
    }

    pub fn new() -> Sgvcs {
        let curr_dir: PathBuf = std::env::current_dir().expect("Cannot get current directory");
        let repo_path: PathBuf = curr_dir.join(".sgvcs");
        let objects_path: PathBuf = repo_path.join("objects");
        let index_path: PathBuf = repo_path.join("index");
        let head_path: PathBuf = repo_path.join("HEAD");

        Sgvcs {
            repo_path,
            objects_path,
            index_path,
            head_path,
        }
    }

    pub async fn init(&self) -> Result<()> {
        if !self.repo_path.exists() {
            fs::create_dir_all(&self.repo_path).await?;
            println!("Created repo directory: {:?}", self.repo_path);
        } else {
            println!("Repo directory already exists: {:?}", self.repo_path);
        }

        // Create the objects directory if it does not exist
        if !self.objects_path.exists() {
            fs::create_dir_all(&self.objects_path).await?;
            println!("Created objects directory: {:?}", self.objects_path);
        } else {
            println!("Objects directory already exists: {:?}", self.objects_path);
        }

        // Create the index file and write an empty array if it does not exist
        if !self.index_path.exists() {
            let mut index_file: fs::File = fs::File::create(&self.index_path).await?;
            index_file.write_all(b"[]").await?;
            println!("Created index file with empty array: {:?}", self.index_path);
        } else {
            println!("Index file already exists: {:?}", self.index_path);
        }

        // Create the HEAD file if it does not exist
        if !self.head_path.exists() {
            fs::File::create(&self.head_path).await?;
            println!("Created HEAD file: {:?}", self.head_path);
        } else {
            println!("HEAD file already exists: {:?}", self.head_path);
        }

        Ok(())
    }

    pub async fn add_file(&mut self, path: &Path) {
        println!("{:?}", path);
        let mut file: fs::File = fs::File::open(path).await.unwrap();
        let mut content: Vec<u8> = Vec::new();
        file.read_to_end(&mut content).await.unwrap();
        let hashed_data: String = Self::hash(content.as_slice());
        let object_path: PathBuf = self.objects_path.join(hashed_data.clone());
        if !object_path.exists() {
            let mut object_file: fs::File = fs::File::create(&object_path).await.unwrap();
            object_file.write_all(content.as_slice()).await.unwrap();
        } else {
            let mut object_file: fs::File = fs::File::open(&object_path).await.unwrap();
            object_file.write_all(content.as_slice()).await.unwrap();
        }
        self.update_staging_area(path, hashed_data.clone()).await;
        println!("Added {:?} to index", path);
    }

    pub async fn update_staging_area(&mut self, file_path: &Path, file_hash: String) {
        let mut index_file = fs::File::open(&self.index_path).await.unwrap();
        let mut buffer = String::new();
        index_file.read_to_string(&mut buffer).await.unwrap();
        let mut data: Vec<IndexData> = serde_json::from_str(&buffer).unwrap();
        let index_data = IndexData {
            path: file_path.to_str().unwrap().to_string(),
            hash: file_hash.to_string(),
        };
        data.push(index_data);
        let data_json = serde_json::to_string_pretty(&data).unwrap();
        let mut index_file = fs::File::create(&self.index_path).await.unwrap();
        index_file.write_all(data_json.as_bytes()).await.unwrap();
    }

    pub async fn commit(&mut self, message: String) {
        let mut index_file: fs::File = fs::File::open(&self.index_path).await.unwrap();
        let mut buffer: String = String::new();
        index_file.read_to_string(&mut buffer).await.unwrap();
        let parent_commit: String = self.get_current_head().await;

        let commit = CommitData {
            message,
            time_stamp: Utc::now().format("%d-%m-%Y %H:%M:%S").to_string(),
            files: serde_json::from_str(&buffer).unwrap(),
            parent: parent_commit,
        };

        let commit_json = serde_json::to_string_pretty(&commit).unwrap();
        let commit_hash = Self::hash(commit_json.as_bytes());
        let commit_path = self.objects_path.join(commit_hash.clone());
        let mut commit_file = fs::File::create(&commit_path).await.unwrap();
        commit_file.write_all(commit_json.as_bytes()).await.unwrap();

        let mut head_file = fs::File::create(&self.head_path).await.unwrap();
        head_file.write_all(commit_hash.as_bytes()).await.unwrap();

        let mut index_file = fs::File::create(&self.index_path).await.unwrap();
        index_file.write_all(b"[]").await.unwrap();

        println!("Committed: {:?}", commit_hash);
    }

    async fn get_current_head(&self) -> String {
        match fs::File::open(&self.head_path).await {
            Ok(mut head_file) => {
                let mut buffer = String::new();
                match head_file.read_to_string(&mut buffer).await {
                    Ok(_) => buffer,
                    Err(_) => String::new(), // Return empty string on read error
                }
            }
            Err(_) => String::new(), // Return empty string if file cannot be opened
        }
    }

    pub async fn log(&mut self) {
        let mut current_hash: String = self.get_current_head().await;
        while !current_hash.is_empty() {
            let mut commit_file = fs::File::open(self.objects_path.join(current_hash.clone()))
                .await
                .unwrap();
            let mut buffer = String::new();
            commit_file.read_to_string(&mut buffer).await.unwrap();

            let commit: CommitData = serde_json::from_str(&buffer).unwrap();

            println!("\nCommit: {}", current_hash);
            println!("{:?}", commit);

            current_hash = commit.parent.clone();
        }
    }

    pub async fn show_commit_diff(&self, commithash: String) {
        let commit_data: Option<CommitData> = self.get_commit_data(commithash).await;
        match commit_data {
            Some(commit) => {
                println!("Changes in the last commit are: ");
                for file in commit.files {
                    println!("File: {}", file.path.to_string());
                    let file_content: String = self.get_file_contents(file.hash).await;
                    println!("{:?}", file_content);
                    if !commit.parent.is_empty() {
                        let parent_data: Option<CommitData> =
                            self.get_commit_data(commit.parent.clone()).await;
                        match parent_data {
                            Some(data) => {
                                let file_parent_contents = self
                                    .get_parent_file_content(
                                        data,
                                        &self.objects_path.join(file.path.clone()),
                                    )
                                    .await;
                                println!("{:?}", file_parent_contents);
                            }
                            None => println!(
                                "Parent commit not found for this file: {}",
                                file.path.clone()
                            ),
                        }
                    } else {
                        println!("First commit");
                    }
                }
            },
            None => println!("Commit not found"),
        }
    }

    async fn get_commit_data(&self, commithash: String) -> Option<CommitData> {
        let commit_file = fs::File::open(self.objects_path.join(commithash.clone())).await;
        match commit_file {
            Ok(mut commit_data) => {
                let mut buffer = String::new();
                commit_data.read_to_string(&mut buffer).await.unwrap();
                let data: CommitData = serde_json::from_str(&buffer).unwrap();
                Some(data)
            }
            Err(e) => {
                println!("Commit not found {}", e);
                None
            }
        }
    }

    fn hash(content: &[u8]) -> String {
        let mut hasher: sha1::digest::core_api::CoreWrapper<sha1::Sha1Core> = Sha1::new();
        hasher.update(content);
        let result = hasher.finalize();
        let mut hash_hex = String::new();
        for byte in result.iter() {
            hash_hex.push_str(&format!("{:02x}", byte));
        }
        hash_hex
    }

    async fn get_file_contents(&self, file_hash: String) -> String {
        let mut file: fs::File = fs::File::open(self.objects_path.join(file_hash))
            .await
            .unwrap();
        let mut content: String = String::new();
        file.read_to_string(&mut content).await.unwrap();
        content
    }

    async fn get_parent_file_content(
        &self,
        parent_commit_data: CommitData,
        file_path: &Path,
    ) -> Option<String> {
        let file_hash = parent_commit_data
            .files
            .iter()
            .find(|file| file.path == file_path.to_str().unwrap());
        match file_hash {
            Some(file) => {
                let file_content = self.get_file_contents(file.hash.clone()).await;
                Some(file_content)
            }
            None => None,
        }
    }
}
