mod api;
mod schema;

pub use api::*;
pub use schema::*;

#[cfg(test)]
mod test {
    use crate::{YoutubeApi, YoutubeVideoId};

    static VIDEO_ID: &str = "dQw4w9WgXcQ";

    #[actix_web::test]
    async fn get_video_metadata() {
        let api = YoutubeApi::default();
        let video_id: YoutubeVideoId = VIDEO_ID.try_into().unwrap();
        let result = api.get_video_metadata(&video_id).await.unwrap();
        assert!(result.items.len() > 0);
    }
}
