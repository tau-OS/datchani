use std::path::{Path, PathBuf};

use crate::query::{parse_query, query};
use crate::{db::IndexBackend, files::IndexedFile};
use color_eyre::Result;
use ignore::WalkState;
use rayon::prelude::*;
use tokio::sync::mpsc;
use tracing::{debug, info, log::warn};
use walkdir::WalkDir;

// TODO: Generic Indexer trait
// we are gonna index files for now

pub struct Indexer {
    backend: Box<dyn IndexBackend>,
}

impl Indexer {
    async fn index_all(&mut self, path: &Path) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);

        let path = path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            ignore::WalkBuilder::new(path)
                .git_ignore(true)
                .git_exclude(true)
                .ignore(true)
                .require_git(false)
                .hidden(true)
                .standard_filters(true)
                .build_parallel()
                .run(|| {
                    Box::new(|entry| {
                        let entry = match entry {
                            Ok(e) => e,
                            Err(e) => return WalkState::Continue,
                        };

                        let entry = match IndexedFile::new(entry.path().to_path_buf()) {
                            Ok(e) => e,
                            Err(e) => return WalkState::Continue,
                        };

                        tx.blocking_send(entry);

                        WalkState::Continue
                    })
                });
        });

        while let Some(entry) = rx.recv().await {
            self.backend.push_file(entry).await?;
        }

        Ok(())
    }

    async fn watch() -> Result<()> {
        todo!()
    }
}

#[tokio::test]
async fn test_indexer() -> Result<()> {
    color_eyre::install()?;
    let mut indexer = Indexer {
        backend: Box::new(crate::db::SurrealBackend::new().await?),
    };

    indexer.index_all(&std::env::current_dir()?).await?;

    indexer.backend.query(&parse_query("ext:rs")?).await?;

    // let results = indexer.backend.search("test").await?;

    // for result in results {
    // println!("{:?}", result);
    // }

    Ok(())
}
