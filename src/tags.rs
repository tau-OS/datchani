//! File tagging module
//! Uses xattr to store tags
//! Tags should be stored as a CSV string,
//! With the xattr name "user.tags"

use std::fs::File;

use color_eyre::Result;
use xattr::FileExt;
const TAGS_XATTR: &str = "user.tags";

fn parse_tags(tags: &str) -> Vec<String> {
    if str::is_empty(tags) {
        return Vec::new();
    }
    tags.split(',').map(|s| s.trim().to_string()).collect()
}

/// get tags from a path, can be a file or directory
/// returns a vector of tags
pub fn get_tags(path: &str) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let tags = file.get_xattr(TAGS_XATTR)?.unwrap_or_default();
    Ok(parse_tags(String::from_utf8(tags)?.as_str()))
}

#[test]
fn xattr_test() {
    // read xattr
    let mut file = std::fs::File::open("Cargo.toml").unwrap();
    let xattr = file.list_xattr().unwrap();
    for attr in xattr {
        println!("{:?}", attr);
    }
    let xattr = file.get_xattr("security.selinux").unwrap().map(|x| String::from_utf8(x).unwrap());
    println!("{:?}", xattr);

    let a = parse_tags("");
    println!("{:?}", a);
}
