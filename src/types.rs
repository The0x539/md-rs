use crate::schema;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

pub use schema::{MangaAttributes, MangaId, RelationshipType};

pub struct Manga {
    pub id: MangaId,
    pub attributes: MangaAttributes,
    pub relationships: HashMap<RelationshipType, Uuid>,
}

#[derive(Deserialize)]
#[serde(from = "schema::MangaListResponse")]
pub struct MangaList {
    pub series: HashMap<MangaId, Manga>,
}

impl From<schema::MangaListResponse> for MangaList {
    fn from(value: schema::MangaListResponse) -> MangaList {
        let series = value
            .results
            .into_iter()
            .map(|resp: schema::MangaResponse| {
                assert_eq!(resp.result, schema::Success::Ok);

                let relationships = resp
                    .relationships
                    .into_iter()
                    .map(|rel| (rel.rel_type, rel.id))
                    .collect();

                let schema::Manga {
                    item_type,
                    id,
                    attributes,
                } = resp.data;
                assert_eq!(item_type, schema::ItemType::Manga);

                let manga = Manga {
                    id,
                    attributes,
                    relationships,
                };

                (id, manga)
            })
            .collect();
        MangaList { series }
    }
}
