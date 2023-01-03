//! Database module
//! Datchani will use SurrealDB as its database.
//! It would contain the following tables:
//! - files
//! - tags
//!
//! This is currently a stub. The database will be implemented in the future. Right now make do with a JSON file.

use crate::files::Index;
use crate::query::query;
// TODO Implement database
use crate::{files::IndexedFile, query::Query};
use async_trait::async_trait;
use color_eyre::Result;
use surrealdb::engines::any::{connect, Any};
use surrealdb::sql;
use surrealdb::Surreal;

#[async_trait]
pub trait IndexBackend {
    async fn push_file(&mut self, entry: IndexedFile) -> Result<IndexedFile>;
    async fn query(&mut self, query: &Query) -> Result<()>;
}

pub struct SurrealBackend(Surreal<Any>);

impl SurrealBackend {
    pub async fn new() -> Result<Self> {
        let db = connect("file://owo.db").await?;
        db.use_ns("datchani").use_db("datchani").await?;
        Ok(Self(db))
    }
}

#[async_trait]
impl IndexBackend for SurrealBackend {
    async fn push_file(&mut self, entry: IndexedFile) -> Result<IndexedFile> {
        // println!("{:?}", a);
        let res: IndexedFile = self.0.update(("file", entry.clone().path.to_str().unwrap())).content::<IndexedFile>(entry).await?;
        Ok(res)
    }

    async fn query(&mut self, q: &Query) -> Result<()> {
        let mut results = self.0.select("file").await?;

        let res: Vec<IndexedFile> = results;

        // println!("{:#?}", res);
        // ! TEMP
        let index = Index {
            files: res,
        };

        let res = query(q, &index);

        println!("{:#?}", res);

        Ok(())
    }
}
