mod api;
mod schema;

pub use api::*;
pub use schema::*;

#[cfg(test)]
mod test {
    use crate::{Api, VideoId, PlaylistId};

    static VIDEO_ID: &str = "dQw4w9WgXcQ";
    static PLAYLIST_ID: &str = "PLlaN88a7y2_oBUxLd3j23dkAFNtM-P24e";

    #[test_log::test(actix_web::test)]
    async fn get_video() {
        let api = Api::default();
        let video_id: VideoId = VIDEO_ID.try_into().unwrap();
        let response = api.get_video(&video_id).await.unwrap();
        let video = response.value.items.first().unwrap();
        log::debug!("{0:?}", video);
        assert!(video.id == video_id);
    }

    #[test_log::test(actix_web::test)]
    async fn get_playlist() {
        let api = Api::default();
        let playlist_id: PlaylistId = PLAYLIST_ID.try_into().unwrap();
        let response = api.get_playlist(&playlist_id).await.unwrap();
        let playlist = response.value;
        log::debug!("{0:?}", playlist);
        assert!(playlist.items.len() > 0);
    }
}
