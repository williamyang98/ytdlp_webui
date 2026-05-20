mod schema;
mod api;

pub use schema::*;
pub use api::*;

#[cfg(test)]
mod test {
    use crate::GithubApi;

    #[actix_web::test]
    async fn get_github_releases() {
        let api = GithubApi::default();
        let result = api.get_releases("williamyang98", "ytdlp_webui", 1, 1).await;
        let result = result.unwrap();
        assert!(result.len() > 0);
    }
}
