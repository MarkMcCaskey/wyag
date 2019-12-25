use crate::repository::Repo;
use crypto::{digest::Digest, sha1::Sha1};
use flate2::{read::ZlibDecoder, write::ZlibEncoder};
use std::io::{Read, Write};
use std::{fs, str};

/// Generic VCS object type
pub trait Object {
    fn serialize(&self) -> Vec<u8>;
    //fn deserialize(bytes: &[u8]) -> Self;
    fn fmt_header(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Commit,
    Tag,
    Tree,
    Blob,
}

impl std::default::Default for ObjectType {
    fn default() -> Self {
        ObjectType::Blob
    }
}

const OBJECT_TYPE_VARIANTS: &[&str] = &["commit",
          "tag",
          "tree",
          "blob"];

impl ObjectType {
    pub const fn variants() -> &'static [&'static str] {
        OBJECT_TYPE_VARIANTS
    }
}

impl str::FromStr for ObjectType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(
        match s {
            "commit" => ObjectType::Commit,
            "tag" => ObjectType::Tag,
            "tree" => ObjectType::Tree,
            "blob" => ObjectType::Blob,
            _ => return Err(format!("Invalid ObjectType variant: {}", s)),
        })
    }
}

#[derive(Debug)]
pub struct Commit {}

impl Commit {
    pub fn deserialize(bytes: &[u8]) -> Self {
        todo!("deserialize commit");
    }
}

impl Object for Commit {
    fn serialize(&self) -> Vec<u8> {
        todo!("serialize commit");
    }

    fn fmt_header(&self) -> &'static str {
        "commit"
    }
}

#[derive(Debug)]
pub struct Tag {}

impl Tag {
    pub fn deserialize(bytes: &[u8]) -> Self {
        todo!("deserialize tag");
    }
}

impl Object for Tag {
    fn serialize(&self) -> Vec<u8> {
        todo!("serialize tag");
    }

    fn fmt_header(&self) -> &'static str {
        "tag"
    }
}

#[derive(Debug)]
pub struct Tree {}

impl Tree {
    pub fn deserialize(bytes: &[u8]) -> Self {
        todo!("deserialize tree");
    }
}

impl Object for Tree {
    fn serialize(&self) -> Vec<u8> {
        todo!("serialize tree");
    }

    fn fmt_header(&self) -> &'static str {
        "tree"
    }
}

#[derive(Debug)]
pub struct Blob {
    data: Vec<u8>,
}

impl Blob {
    pub fn deserialize(bytes: &[u8]) -> Self {
        Self {
            data: bytes.iter().cloned().collect(),
        }
    }
}

impl Object for Blob {
    fn serialize(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn fmt_header(&self) -> &'static str {
        "blob"
    }
}

pub fn object_read(repo: &Repo, sha_str: &str) -> Result<Box<dyn Object>, String> {
    let file = repo.repo_file(
        format!("objects/{}/{}", &sha_str[0..2], &sha_str[2..]),
        false,
    )?;
    let reader = fs::OpenOptions::new()
        .read(true)
        .open(file)
        .map_err(|e| format!("Could not open file to read in object_read: {:?}", e))?;
    let decoder = ZlibDecoder::new(reader);
    let raw_bytes = decoder
        .bytes()
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|e| format!("Could not decode as zlib: {:?}", e))?;
    let space_idx = raw_bytes
        .iter()
        .position(|i| *i == b' ')
        .ok_or_else(|| format!("Format error, no space byte found"))?;
    let fmt = &raw_bytes[..space_idx];

    let nul_idx = raw_bytes
        .iter()
        .position(|i| *i == 0)
        .ok_or_else(|| format!("Format error, no nul byte found"))?;
    let size = {
        let size_str =
            str::from_utf8(&raw_bytes[space_idx + 1..nul_idx]).expect("valid utf8 for size field");
        size_str
            .parse::<usize>()
            .map_err(|e| format!("could not parse size field, \"{}\" as a number: {:?}", size_str, e))?
    };

    if size != raw_bytes.len() - nul_idx - 1 {
        return Err(format!("Malformed object {}: bad length", sha_str));
    }

    Ok(match fmt {
        b"commit" => Box::new(Commit::deserialize(&raw_bytes[nul_idx + 1..])),
        b"tree" => Box::new(Tree::deserialize(&raw_bytes[nul_idx + 1..])),
        b"tag" => Box::new(Tag::deserialize(&raw_bytes[nul_idx + 1..])),
        b"blob" => Box::new(Blob::deserialize(&raw_bytes[nul_idx + 1..])),
        otherwise => {
            return Err(format!(
                "Unknown object type {:?} for object: {}",
                str::from_utf8(otherwise),
                sha_str
            ));
        }
    })
}

pub fn object_find<'a>(repo: &Repo, name: &'a str, fmt: Option<ObjectType>, follow: bool) -> &'a str {
    return name;
}

/// Passing a repo means it will write
pub fn object_write(
    repo: Option<&Repo>,
    object: &dyn Object,
) -> Result<String, String> {
    let mut obj_bytes: Vec<u8> = vec![];
    let data = object.serialize();

    obj_bytes.extend(object.fmt_header().as_bytes());
    obj_bytes.push(b' ');
    obj_bytes.extend(format!("{}", data.len()).as_bytes());
    obj_bytes.push(0);
    obj_bytes.extend(&data[..]);

    let mut sha = Sha1::new();
    sha.input(&obj_bytes);
    let hex_out = sha.result_str();

    if let Some(repo) = repo {
        let path = repo.repo_file(format!("objects/{}/{}", &hex_out[..2], &hex_out[2..]), true)?;

        let mut oo = fs::OpenOptions::new();
        let f =
            oo.create(true).write(true).open(path).map_err(|e| {
                format!("Could not create/open file for object {}: {:?}", hex_out, e)
            })?;
        let mut enc = ZlibEncoder::new(f, Default::default());
        enc.write_all(&obj_bytes[..]).unwrap();
        enc.finish().unwrap();
    }

    Ok(hex_out)
}

pub fn object_hash<R>(reader: &mut R, _type: ObjectType, repo: Option<&Repo>) -> Result<String, String>
where R: Read {
    let mut data = vec![];
    reader.read_to_end(&mut data).unwrap();
    // TODO: refactor to avoid Box
    let obj: Box<dyn Object> = 
    match _type {
        ObjectType::Blob => Box::new(Blob::deserialize(&data)),
        ObjectType::Commit => Box::new(Commit::deserialize(&data)),
        ObjectType::Tag => Box::new(Tag::deserialize(&data)),
        ObjectType::Tree => Box::new(Tree::deserialize(&data)),
    };

    object_write(repo, &*obj)
}
