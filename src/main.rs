mod db;
mod errors;
mod files;
mod query;

use std::env;


use color_eyre::Result;
use tracing::{info};
use walkdir::WalkDir;

use crate::query::{query};

fn main() -> Result<()> {
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
        .num_threads(25)
        .build_global()
        .unwrap();

    // lets walk dir
    // let walker = WalkDir::new(env::current_dir().unwrap());

    // for entry in walker.into_iter().filter_map(|e| e.ok()) {
    //     debug!("Found entry: {:?}", entry);
    // }

    // use rayon to parallelize the walk

    let mut index = files::Index::new();
    let walker = WalkDir::new(env::current_dir().unwrap())
        .into_iter()
        .filter_map(|e| e.ok());
    walker.into_iter().for_each(|entry| {
        // add to data, but this is rayon so it's not thread safe
        // debug!("Found entry: {:?}", entry);
        index.add_file(entry.path().to_path_buf()).unwrap();
    });

    // index.save(env::current_dir().unwrap().join("index.json"))?;

    // let f = files::Index::load(env::current_dir().unwrap().join("index.json"))?;
    // debug!("Loaded index: {:#?}", f);
    let search_query = query::parse_query("release extension:rlib").unwrap();
    let res = query(&search_query, &index);
    println!("{:#?}", res);
    Ok(())
}
