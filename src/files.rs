use color_eyre::{Report, Result};
use file_type_enum::FileType;
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf};

fn default_file_type() -> FileType {
    FileType::Regular
}

fn filetype_serializer<S>(file_type: &FileType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&file_type.to_string())
}

fn filetype_deserializer<'de, D>(deserializer: D) -> Result<FileType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str().to_lowercase().as_str() {
        "regular file" => Ok(FileType::Regular),
        "directory" => Ok(FileType::Directory),
        "symlink" => Ok(FileType::Symlink),
        "block_device" => Ok(FileType::BlockDevice),
        "char_device" => Ok(FileType::CharDevice),
        "fifo" => Ok(FileType::Fifo),
        "socket" => Ok(FileType::Socket),
        _ => Err(serde::de::Error::custom("Invalid file type")),
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub version: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub files: Vec<IndexedFile>,
    // pub metadata: IndexMetadata,
}

impl Index {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            // metadata: IndexMetadata {
            //     version: String::from("0.1.0"),
            //     created: String::from(""),
            //     updated: String::from(""),
            // },
        }
    }


    pub fn get_file(&self, path: PathBuf) -> Option<&IndexedFile> {
        self.files.iter().find(|f| f.path == path)
    }

    pub fn add_file(&mut self, path: PathBuf) -> Result<()> {
        let file = IndexedFile::new(path)?;
        self.files.push(file);
        Ok(())
    }

    pub fn save(&self, path: PathBuf) -> Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    pub fn load(path: PathBuf) -> Result<Self> {
        let file = File::open(path)?;
        let index = serde_json::from_reader(file)?;
        Ok(index)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    pub path: PathBuf,
    // We are skipping this, but for some reason serde wants a default value
    #[serde(
        default = "default_file_type",
        serialize_with = "filetype_serializer",
        deserialize_with = "filetype_deserializer"
    )]
    pub file_type: FileType,
    // optional because sometimes we don't know the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
}

impl Default for IndexedFile {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            file_type: FileType::Regular,
            data_type: None,
        }
    }
}

impl IndexedFile {
    pub fn new(path: PathBuf) -> Result<Self> {
        let file_type = FileType::from_path(&path)?;
        let data_type = {
            let d = if file_type == FileType::Regular {
                let data_type = infer::get_from_path(&path)?;
                Some(data_type)
            } else {
                None
            };

            d.map(|t| {
                t.map(|t| t.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string())
            })
        };

        Ok(Self {
            path,
            file_type,
            data_type,
        })
    }

    /// Tries to open the file and returns a `File` pointer.
    pub fn open(&self) -> Result<File> {
        File::open(&self.path).map_err(Report::from)
    }
}
