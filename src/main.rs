// use std::{path::Path, thread};

// use std::time::Duration;
use vcs::Sgvcs;

pub mod vcs;

#[tokio::main]
async fn main() {
    let sgvcs: Result<Sgvcs, std::io::Error> = Sgvcs::new_async().await;
    println!("{:?}", sgvcs);
    match sgvcs {
        Ok(sgvcs) => {
            let mut sgvcs: Sgvcs = sgvcs;
            // sgvcs.add_file(&Path::new("src/sample.txt")).await;
            // sgvcs.commit("Initial Commit".to_string()).await;
            // thread::sleep(Duration::from_secs(1));
            // sgvcs.add_file(&Path::new("src/sample.txt")).await;
            // sgvcs.commit("Second Commit".to_string()).await;
            // thread::sleep(Duration::from_secs(1));
            // sgvcs.add_file(&Path::new("src/sample.txt")).await;
            // sgvcs.add_file(&Path::new("src/sample2.txt")).await;
            // sgvcs.commit("Third Commit".to_string()).await;
            sgvcs.log().await;
            sgvcs.show_commit_diff("53d4e91b205a6448cc644193b353768e783dc5f0".to_string()).await;
        }
        Err(err) => println!("{:?}", err),
    }
}
