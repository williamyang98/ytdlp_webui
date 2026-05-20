use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use lazy_static::lazy_static;
use regex::Regex;
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

#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(transparent)]
pub struct DateTimeRfc3339(#[serde(with = "time::serde::rfc3339")] pub time::OffsetDateTime);

#[derive(Clone,Debug,Display,PartialEq,Eq,Hash,Serialize,AsRef,Into)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = Text))]
#[serde(transparent)]
pub struct YoutubeVideoId(String);

impl YoutubeVideoId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone,Copy,Debug,Error,Serialize)]
pub enum YoutubeVideoIdError {
    #[error("Invalid length: expected={expected}, given={given}")]
    InvalidLength { expected: usize, given: usize },
    #[error("Invalid character: index={index}, char={char}")]
    InvalidCharacter { index: usize, char: char },
}

impl TryInto<YoutubeVideoId> for &str {
    type Error = YoutubeVideoIdError;

    fn try_into(self) -> Result<YoutubeVideoId, Self::Error> {
        const VALID_YOUTUBE_ID_LENGTH: usize = 11;
        if self.len() != VALID_YOUTUBE_ID_LENGTH {
            return Err(Self::Error::InvalidLength { expected: VALID_YOUTUBE_ID_LENGTH, given: self.len() });
        }
        let invalid_char = self.chars().enumerate().find(|(_,c)| !matches!(c, 'A'..='Z'|'a'..='z'|'0'..='9'|'-'|'_'));
        if let Some((index, c)) = invalid_char {
            return Err(Self::Error::InvalidCharacter { index, char: c });
        }
        Ok(YoutubeVideoId(self.to_owned()))
    }
}

#[cfg(feature = "diesel")]
impl<DB> ToSql<Text, DB> for YoutubeVideoId where DB: Backend, str: ToSql<Text, DB> {
    fn to_sql<'a>(&'a self, out: &mut Output<'a, '_, DB>) -> diesel::serialize::Result {
        let value: &str = self.as_ref();
        value.to_sql(out)
    }
}

#[cfg(feature = "diesel")]
impl<DB> FromSql<Text, DB> for YoutubeVideoId where DB: Backend, *const str: FromSql<Text, DB> {
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        type A = *const str;
        let text = A::from_sql(bytes)?;
        let text = unsafe { &*text };
        let id: YoutubeVideoId = text.try_into()?;
        Ok(id)
    }
}


#[derive(Clone,Debug,Display,AsRef,Into)]
pub struct YoutubeApiKey(String);

impl TryInto<YoutubeApiKey> for &str {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<YoutubeApiKey, Self::Error> {
        lazy_static! {
            static ref YOUTUBE_API_KEY_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9\\.\-\_]{24,}$").unwrap();
        }
        if !YOUTUBE_API_KEY_REGEX.is_match(self) {
            return Err(anyhow::anyhow!("Invalid youtube api key: {0}", self));
        }
        Ok(YoutubeApiKey(self.to_owned()))
    }
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct ContentDetails {
    pub duration: String,
    pub dimension: String,
    pub definition: String,
    pub caption: String,
    #[serde(rename="licensedContent")]
    pub licensed_content: bool,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct Snippet {
    #[serde(rename="publishedAt")]
    pub published_at: DateTimeRfc3339,
    #[serde(rename="channelId")]
    pub channel_id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub thumbnails: HashMap<String, Thumbnail>,
    #[serde(rename="channelTitle")]
    pub channel_title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename="categoryId")]
    pub category_id: String,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct Item {
    pub id: String,
    pub etag: String,
    pub kind: String,
    pub snippet: Snippet,
    #[serde(rename="contentDetails")]
    pub content_details: ContentDetails,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct PageInfo {
    #[serde(rename="totalResults")]
    pub total_results: usize,
    #[serde(rename="resultsPerPage")]
    pub results_per_page: usize,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct YoutubeMetadata {
    pub kind: String,
    pub etag: String,
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(rename="pageInfo")]
    pub page_info: PageInfo,
}

