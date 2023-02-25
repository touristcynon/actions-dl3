use actions_dl::{batch_translate, insert_subtitle};

#[tokio::main]
async fn main() {
    let dir_path = "downloads";
    batch_translate(dir_path).await;
    insert_subtitle(dir_path);
}