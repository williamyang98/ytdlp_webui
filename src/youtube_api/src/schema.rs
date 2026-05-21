use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use derive_more::{AsRef, Display, Into};
use thiserror::Error;

#[cfg(feature = "diesel")]
use diesel::{
    expression::AsExpression,
    deserialize::{FromSql, FromSqlRow},
    backend::Backend,
    serialize::{Output, ToSql},
    sql_types::Text,
};

// serialise the unknown parts as well when we cache the results to disk
type Extra = serde_json::Map<String, serde_json::Value>;

// https://www.rfc-editor.org/rfc/rfc3339.html
// RFC3339 is a stricter profile for ISO8601
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(transparent)]
pub struct DateTimeRfc3339(#[serde(with = "time::serde::rfc3339")] pub time::OffsetDateTime);

// Video items
// https://developers.google.com/youtube/v3/docs/videos/list
#[derive(Clone,Debug,Display,PartialEq,Eq,Hash,Serialize,Deserialize,AsRef,Into)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = Text))]
#[serde(transparent)]
pub struct VideoId(String);

impl VideoId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone,Copy,Debug,Error,Serialize)]
pub enum VideoIdError {
    #[error("Invalid length: expected={expected}, given={given}")]
    InvalidLength { expected: usize, given: usize },
    #[error("Invalid character: index={index}, char={char}")]
    InvalidCharacter { index: usize, char: char },
}

impl TryInto<VideoId> for &str {
    type Error = VideoIdError;

    fn try_into(self) -> Result<VideoId, Self::Error> {
        const VALID_YOUTUBE_ID_LENGTH: usize = 11;
        if self.len() != VALID_YOUTUBE_ID_LENGTH {
            return Err(Self::Error::InvalidLength { expected: VALID_YOUTUBE_ID_LENGTH, given: self.len() });
        }
        let invalid_char = self.chars().enumerate().find(|(_,c)| !matches!(c, 'A'..='Z'|'a'..='z'|'0'..='9'|'-'|'_'));
        if let Some((index, c)) = invalid_char {
            return Err(Self::Error::InvalidCharacter { index, char: c });
        }
        Ok(VideoId(self.to_owned()))
    }
}

#[cfg(feature = "diesel")]
impl<DB> ToSql<Text, DB> for VideoId where DB: Backend, str: ToSql<Text, DB> {
    fn to_sql<'a>(&'a self, out: &mut Output<'a, '_, DB>) -> diesel::serialize::Result {
        let value: &str = self.as_ref();
        value.to_sql(out)
    }
}

#[cfg(feature = "diesel")]
impl<DB> FromSql<Text, DB> for VideoId where DB: Backend, *const str: FromSql<Text, DB> {
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        type A = *const str;
        let text = A::from_sql(bytes)?;
        let text = unsafe { &*text };
        let id: VideoId = text.try_into()?;
        Ok(id)
    }
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: usize,
    pub height: usize,
    #[serde(flatten)] _extras: Extra,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoContentDetailsPart {
    pub duration: String,
    pub dimension: String,
    pub definition: String,
    pub caption: String,
    pub licensed_content: bool,
    #[serde(flatten)] _extras: Extra,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSnippetPart {
    pub published_at: DateTimeRfc3339,
    pub channel_id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub thumbnails: HashMap<String, Thumbnail>,
    pub channel_title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub category_id: String,
    #[serde(flatten)] _extras: Extra,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoItem {
    pub id: VideoId,
    pub etag: String,
    pub kind: String,
    pub snippet: VideoSnippetPart,
    pub content_details: VideoContentDetailsPart,
    #[serde(flatten)] _extras: Extra,
}

// Playlist Items
// https://developers.google.com/youtube/v3/docs/playlistItems/list
#[derive(Clone,Debug,Display,PartialEq,Eq,Hash,Serialize,Deserialize,AsRef,Into)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = Text))]
#[serde(transparent)]
pub struct PlaylistId(String);

impl PlaylistId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone,Copy,Debug,Error,Serialize)]
pub enum PlaylistIdError {
    #[error("Id too short: minimum={minimum}, given={given}")]
    TooShort { minimum: usize, given: usize },
    #[error("Invalid character: index={index}, char={char}")]
    InvalidCharacter { index: usize, char: char },
}

impl TryInto<PlaylistId> for &str {
    type Error = PlaylistIdError;

    fn try_into(self) -> Result<PlaylistId, Self::Error> {
        const MINIMUM_LENGTH: usize = 11;
        if self.len() < MINIMUM_LENGTH {
            return Err(Self::Error::TooShort { minimum: MINIMUM_LENGTH, given: self.len() });
        }
        let invalid_char = self.chars().enumerate().find(|(_,c)| !matches!(c, 'A'..='Z'|'a'..='z'|'0'..='9'|'-'|'_'));
        if let Some((index, c)) = invalid_char {
            return Err(Self::Error::InvalidCharacter { index, char: c });
        }
        Ok(PlaylistId(self.to_owned()))
    }
}

#[derive(Clone,Debug,Display,PartialEq,Eq,Hash,Serialize,Deserialize,AsRef,Into)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = Text))]
#[serde(transparent)]
pub struct PlaylistItemId(String);

impl PlaylistItemId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryInto<PlaylistItemId> for &str {
    type Error = PlaylistIdError;

    fn try_into(self) -> Result<PlaylistItemId, Self::Error> {
        const MINIMUM_LENGTH: usize = 11;
        if self.len() < MINIMUM_LENGTH {
            return Err(Self::Error::TooShort { minimum: MINIMUM_LENGTH, given: self.len() });
        }
        let invalid_char = self.chars().enumerate().find(|(_,c)| !matches!(c, 'A'..='Z'|'a'..='z'|'0'..='9'|'-'|'_'));
        if let Some((index, c)) = invalid_char {
            return Err(Self::Error::InvalidCharacter { index, char: c });
        }
        Ok(PlaylistItemId(self.to_owned()))
    }
}

#[cfg(feature = "diesel")]
impl<DB> ToSql<Text, DB> for PlaylistId where DB: Backend, str: ToSql<Text, DB> {
    fn to_sql<'a>(&'a self, out: &mut Output<'a, '_, DB>) -> diesel::serialize::Result {
        let value: &str = self.as_ref();
        value.to_sql(out)
    }
}

#[cfg(feature = "diesel")]
impl<DB> FromSql<Text, DB> for PlaylistId where DB: Backend, *const str: FromSql<Text, DB> {
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        type A = *const str;
        let text = A::from_sql(bytes)?;
        let text = unsafe { &*text };
        let id: PlaylistId = text.try_into()?;
        Ok(id)
    }
}

#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistItem {
    pub id: PlaylistItemId,
    pub etag: String,
    pub kind: String,
    pub content_details: PlaylistContentDetailsPart,
    #[serde(flatten)] _extras: Extra,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistContentDetailsPart {
    pub video_id: VideoId,
    pub video_published_at: DateTimeRfc3339,
    #[serde(flatten)] _extras: Extra,
}

// default paginated response
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub total_results: usize,
    pub results_per_page: usize,
    #[serde(flatten)] _extras: Extra,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub kind: String,
    pub etag: String,
    pub items: Vec<T>,
    pub page_info: PageInfo,
    pub prev_page_token: Option<String>,
    pub next_page_token: Option<String>,
    #[serde(flatten)] _extras: Extra,
}

