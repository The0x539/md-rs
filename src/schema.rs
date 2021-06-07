#![allow(dead_code)]

use chrono::{DateTime, NaiveDateTime, Utc};
use optfield::optfield;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use url::Url;
use uuid::Uuid;

macro_rules! wrapper {
    ($(#[$attr:meta])* $name:ident: $inner:ty) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub $inner);
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

macro_rules! id {
    ($name:ident) => {
        wrapper!(
            #[derive(Copy)]
            $name: Uuid
        );
    };
}

id!(MangaId);
id!(TagId);
id!(ChapterId);

wrapper!(Filename: String);
wrapper!(ChapterHash: String);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseUrl {
    pub base_url: Url,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthReport<'a> {
    pub url: &'a str,
    pub success: bool,
    pub cached: bool,
    pub bytes: usize,
    pub duration: u128,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Demographic {
    Shounen,
    Shoujo,
    Josei,
    Seinen,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationStatus {
    Ongoing,
    Completed,
    Hiatus,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingStatus {
    Reading,
    OnHold,
    PlanToRead,
    Dropped,
    ReReading,
    Completed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentRating {
    Safe,
    Suggestive,
    Erotica,
    Pornographic,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    Manga,
    Chapter,
    CoverArt,
    Author,
    Artist,
    ScanlationGroup,
    Tag,
    User,
    CustomList,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Logic {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Success {
    Ok,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SortOrder {
    pub created_at: SortDirection,
    pub updated_at: SortDirection,
}

// TODO: figure out how to add #[serde(skip_serializing_if = "Option::is_none")] to the fields
#[optfield(
    pub MangaListQuery,
    attrs = add(derive(Default))
)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FullMangaListQuery {
    pub limit: u8,
    pub offset: u16,
    pub title: String,
    pub authors: Vec<String>,
    pub artists: Vec<String>,
    pub year: u16,
    pub included_tags: Vec<String>,
    pub included_tags_mode: Logic,
    pub excluded_tags: Vec<String>,
    pub excluded_tags_mode: Logic,
    pub status: Vec<PublicationStatus>,
    pub original_language: Vec<String>,
    pub publication_demographic: Vec<Demographic>,
    pub ids: Vec<Uuid>,
    pub content_rating: Vec<ContentRating>,
    pub created_at_since: NaiveDateTime,
    pub updated_at_since: NaiveDateTime,
    pub order: SortOrder,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListResponse<T> {
    pub results: Vec<T>,
    pub limit: u16,
    pub offset: u16,
    pub total: u16,
}

pub type MangaListResponse = ListResponse<MangaResponse>;

#[derive(Debug, Clone, Deserialize)]
pub struct ItemResponse<T> {
    pub result: Success,
    pub data: T,
    pub relationships: Vec<Relationship>,
}

pub type MangaResponse = ItemResponse<Manga>;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    Manga,
    Tag,
    Chapter,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item<Id, Attrs> {
    pub id: Id,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub attributes: Attrs,
}

pub type Manga = Item<MangaId, MangaAttributes>;

#[derive(Debug, Clone, Deserialize)]
pub struct Relationship {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub rel_type: RelationshipType,
}

type Language = String; // sigh
pub type LocalizedString = HashMap<Language, String>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaAttributes {
    pub title: LocalizedString,
    pub alt_titles: Vec<LocalizedString>,
    pub description: LocalizedString,
    pub is_locked: bool,
    pub links: HashMap<MangaSource, String>,
    pub original_language: String,
    pub last_volume: Option<String>,
    pub last_chapter: Option<String>,
    pub publication_demographic: Option<Demographic>,
    pub status: Option<PublicationStatus>,
    pub year: Option<u16>,
    pub content_rating: Option<ContentRating>,
    pub tags: Vec<Tag>,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize)]
pub enum MangaSource {
    #[serde(rename = "al")]
    Anilist,
    #[serde(rename = "ap")]
    AnimePlanet,
    #[serde(rename = "bw")]
    BookWalker,
    #[serde(rename = "mu")]
    MangaUpdates,
    #[serde(rename = "nu")]
    NovelUpdates,
    #[serde(rename = "kt")]
    KitsuIo,
    #[serde(rename = "amz")]
    Amazon,
    #[serde(rename = "ebj")]
    EbookJapan,
    #[serde(rename = "cdj")]
    CdJapan,
    #[serde(rename = "dj")]
    Doujinshi, // ???
    #[serde(rename = "mal")]
    MyAnimeList,
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "engtl")]
    OfficialEnglish,
}

pub type Tag = Item<TagId, TagAttributes>;

#[derive(Debug, Clone, Deserialize)]
pub struct TagAttributes {
    pub name: LocalizedString,
    // Documented to be a LocalizedString, but I only see empty arrays
    pub description: Vec<()>,
    pub group: String,
    pub version: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MangaAggregateResponse {
    pub result: Success,
    pub volumes: BTreeMap<String, AggregateVolume>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AggregateVolume {
    pub volume: String,
    pub count: u32,
    pub chapters: BTreeMap<String, AggregateChapter>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AggregateChapter {
    pub chapter: String,
    pub count: u32,
}

pub type MangaViewResponse = ItemResponse<Manga>;

pub type MangaFeedResponse = ListResponse<ChapterResponse>;
pub type ChapterResponse = ItemResponse<Chapter>;
pub type Chapter = Item<ChapterId, ChapterAttributes>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterAttributes {
    pub title: String,
    pub volume: Option<String>,
    pub chapter: Option<String>,
    pub translated_language: String,
    pub hash: ChapterHash,
    pub data: Vec<Filename>,
    pub data_saver: Vec<Filename>,
    pub uploader: Option<Uuid>,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub publish_at: DateTime<Utc>,
}
