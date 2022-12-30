mod db;
mod errors;
mod files;
mod query;
mod indexer;

use std::{
    env,
    sync::{Arc, Mutex, RwLock},
};

use crate::query::query;
use color_eyre::Result;
use ignore::WalkState;
use rayon::prelude::*;
use tracing::{debug, info, log::warn};
use walkdir::WalkDir;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    // default level is debug
    pretty_env_logger::formatted_builder()
        .parse_filters(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| "debug".to_string())
                .as_str(),
        )
        .init();

    info!("Hello, world!");

    // global rayon thread
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build_global()?;

    // lets walk dir
    // let walker = WalkDir::new(env::current_dir().unwrap());

    // for entry in walker.into_iter().filter_map(|e| e.ok()) {
    //     debug!("Found entry: {:?}", entry);
    // }

    let args = env::args().collect::<Vec<String>>()[1..].to_vec().join(" ");

    // use rayon to parallelize the walk

    let index = Arc::new(RwLock::new(files::Index::new()));

    ignore::WalkBuilder::new(env::current_dir().unwrap())
        .git_ignore(true)
        .git_exclude(true)
        .ignore(true)
        .require_git(false)
        // .same_file_system(true)
        .hidden(true)
        .standard_filters(true)
        .build_parallel()
        .run(|| {
            Box::new(|entry| {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => return WalkState::Continue
                };
                let mut index = match index.write() {
                    Ok(i) => i,
                    Err(e) => return WalkState::Continue
                };

                // debug!("Found entry: {:?}", entry);
                index
                    .add_file(entry.path().to_path_buf())
                    .unwrap_or_else(|e| {
                        warn!("Error adding file: {:?}", e);
                    });
                WalkState::Continue
            })
        });
    // WalkDir::new("/home/cappy/Projects")
    //     .into_iter()
    //     .par_bridge()
    //     .for_each(|entry| {
    //         let entry = entry.unwrap();
    //         let mut index = index.write().unwrap();
    //         debug!("Found entry: {:?}", entry);
    //         index.add_file(entry.path().to_path_buf()).unwrap();
    //     });

    // info!("Index: {:#?}", index);

    // index
    //     .read()
    //     .unwrap()
    //     .save(env::current_dir().unwrap().join("index.json"))?;

    // let f = files::Index::load(env::current_dir().unwrap().join("index.json"))?;
    // debug!("Loaded index: {:#?}", f);
    let search_query = query::parse_query(&args).unwrap();
    debug!("Parsed query: {:#?}", search_query);
    let res = query(&search_query, &index.read().unwrap());
    println!("{:#?}", res);
    Ok(())
}
