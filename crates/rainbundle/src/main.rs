use std::env::args;
use tokio::process::{Command, Child};
use pathbufd::PathBufD as PathBuf;

async fn just(arg: &str, path: &PathBuf) -> Child {
    Command::new("just")
        .arg(arg)
        .current_dir(path)
        .spawn()
        .unwrap()
}

#[tokio::main]
async fn main() {
    let path = args().skip(1).next().unwrap_or("./".to_string());
    let branch = args().skip(2).next().unwrap_or("prod".to_string());

    let path = if path.starts_with("./") {
        PathBuf::current()
    } else {
        PathBuf::new().join(path)
    };

    let mut api = just(if branch == "prod" { "api" } else { "test-api" }, &path).await;
    let mut web = just(if branch == "prod" { "web" } else { "web-dev" }, &path).await;

    let _ = api.wait().await;
    let _ = web.wait().await;
}
