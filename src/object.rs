use crate::repository::Repo;
use crypto::{digest::Digest, sha1::Sha1};
use flate2::{read::ZlibDecoder, write::ZlibEncoder};
use std::collections::*;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::{fs, str};

/// Generic VCS object type
pub trait Object: std::fmt::Debug {
    fn serialize(&self) -> Vec<u8>;
    //fn deserialize(bytes: &[u8]) -> Self;
    fn fmt_header(&self) -> &'static str;
    fn get_specific(&self) -> ObjectSelect;
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

const OBJECT_TYPE_VARIANTS: &[&str] = &["commit", "tag", "tree", "blob"];

impl ObjectType {
    pub const fn variants() -> &'static [&'static str] {
        OBJECT_TYPE_VARIANTS
    }
}

impl str::FromStr for ObjectType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "commit" => ObjectType::Commit,
            "tag" => ObjectType::Tag,
            "tree" => ObjectType::Tree,
            "blob" => ObjectType::Blob,
            _ => return Err(format!("Invalid ObjectType variant: {}", s)),
        })
    }
}

/// Used to select a specific type of object
#[derive(Debug, Clone)]
pub enum ObjectSelect {
    Commit(Commit),
    Tag(Tag),
    Tree(Tree),
    Blob(Blob),
}

#[derive(Debug, Clone)]
pub struct Commit {
    inner: BTreeMap<String, Vec<String>>,
}

impl Commit {
    pub fn deserialize(bytes: &[u8]) -> Self {
        Self {
            inner: kvlm_parse(std::str::from_utf8(bytes).unwrap()),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Vec<String>> {
        self.inner.get(key)
    }
}

impl Object for Commit {
    fn serialize(&self) -> Vec<u8> {
        kvlm_serializie(&self.inner)
    }

    fn fmt_header(&self) -> &'static str {
        "commit"
    }

    fn get_specific(&self) -> ObjectSelect {
        ObjectSelect::Commit(self.clone())
    }
}

#[derive(Debug, Clone)]
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

    fn get_specific(&self) -> ObjectSelect {
        ObjectSelect::Tag(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct Tree {
    leaves: Vec<TreeLeaf>,
}

impl Tree {
    pub fn deserialize(mut bytes: &[u8]) -> Self {
        let mut leaves = vec![];
        while let Ok((b, leaf)) = TreeLeaf::deserialize(bytes) {
            leaves.push(leaf);
            bytes = b;
        }

        Self { leaves }
    }

    pub fn iterate_leaves(&self) -> impl Iterator<Item = &TreeLeaf> {
        self.leaves.iter()
    }
}

impl Object for Tree {
    fn serialize(&self) -> Vec<u8> {
        let mut out = vec![];
        self.leaves.iter().for_each(|leaf| leaf.serialize(&mut out));

        out
    }

    fn fmt_header(&self) -> &'static str {
        "tree"
    }

    fn get_specific(&self) -> ObjectSelect {
        ObjectSelect::Tree(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct TreeLeaf {
    pub mode: u32,
    pub path: PathBuf,
    pub sha: String,
}

fn hex_digit_to_num(digit: u8) -> Option<u8> {
    Some(match digit {
        b'0' => 0,
        b'1' => 1,
        b'2' => 2,
        b'3' => 3,
        b'4' => 4,
        b'5' => 5,
        b'6' => 6,
        b'7' => 7,
        b'8' => 8,
        b'9' => 9,
        b'A' => 10,
        b'B' => 11,
        b'C' => 12,
        b'D' => 13,
        b'E' => 14,
        b'F' => 15,
        _ => return None,
    })
}

impl TreeLeaf {
    fn serialize(&self, out: &mut Vec<u8>) {
        out.extend(format!("{}", self.mode).as_bytes());
        out.push(b' ');
        out.extend(format!("{}", self.path.to_string_lossy()).as_bytes());
        out.push(0);
        self.sha_to_bytes(out);
    }

    fn sha_to_bytes(&self, bytes: &mut Vec<u8>) {
        let sha_bytes: &[u8] = self.sha.as_ref();
        for byte_seg in sha_bytes.chunks_exact(2) {
            let byte = hex_digit_to_num(byte_seg[1]).unwrap()
                | (hex_digit_to_num(byte_seg[0]).unwrap() << 4);
            bytes.push(byte);
        }
    }

    fn deserialize<'a>(bytes: &'a [u8]) -> Result<(&'a [u8], Self), String> {
        let spc_pos = bytes
            .iter()
            .position(|i| *i == b' ')
            .ok_or_else(|| "Error parsing tree node, expected space".to_owned())?;
        if !(spc_pos == 5 || spc_pos == 6) {
            return Err(format!(
                "Mode string wrong length. Expected 5 or 6 bytes found: {} bytes",
                spc_pos
            ));
        }
        let mode_str = str::from_utf8(&bytes[..spc_pos])
            .map_err(|e| format!("Error reading mode: {:?}", e))?;
        let mode = mode_str
            .parse::<u32>()
            .map_err(|e| format!("Error parsing mode: {:?}", e))?;

        let nul_pos = bytes[spc_pos + 1..]
            .iter()
            .position(|i| *i == 0)
            .ok_or_else(|| "Error parsing tree node, expected nul terminator".to_owned())?;
        let path_str = str::from_utf8(&bytes[spc_pos + 1..nul_pos])
            .map_err(|e| format!("Error reading path: {:?}", e))?;
        let path = PathBuf::from(path_str);
        let sha_bytes = &bytes[nul_pos + 1..nul_pos + 21];
        if sha_bytes.len() < 20 {
            return Err(format!(
                "Error: expected 20 bytes for the hash, found {}",
                sha_bytes.len()
            ));
        }
        // TODO: clean up this code; can be done without extra allocation and format
        let mut sha = String::with_capacity(20);
        for i in 0..20 {
            sha += &format!("{:X}", sha_bytes[i]);
        }

        Ok((&bytes[nul_pos + 21..], Self { mode, path, sha }))
    }
}

#[derive(Debug, Clone)]
pub struct Blob {
    pub data: Vec<u8>,
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

    fn get_specific(&self) -> ObjectSelect {
        ObjectSelect::Blob(self.clone())
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
        size_str.parse::<usize>().map_err(|e| {
            format!(
                "could not parse size field, \"{}\" as a number: {:?}",
                size_str, e
            )
        })?
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

pub fn object_find<'a>(
    repo: &Repo,
    name: &'a str,
    fmt: Option<ObjectType>,
    follow: bool,
) -> &'a str {
    return name;
}

/// Passing a repo means it will write
pub fn object_write(repo: Option<&Repo>, object: &dyn Object) -> Result<String, String> {
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

pub fn object_hash<R>(
    reader: &mut R,
    _type: ObjectType,
    repo: Option<&Repo>,
) -> Result<String, String>
where
    R: Read,
{
    let mut data = vec![];
    reader.read_to_end(&mut data).unwrap();
    // TODO: refactor to avoid Box
    let obj: Box<dyn Object> = match _type {
        ObjectType::Blob => Box::new(Blob::deserialize(&data)),
        ObjectType::Commit => Box::new(Commit::deserialize(&data)),
        ObjectType::Tag => Box::new(Tag::deserialize(&data)),
        ObjectType::Tree => Box::new(Tree::deserialize(&data)),
    };

    object_write(repo, &*obj)
}

pub fn kvlm_parse(raw: &str) -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::new();
    kvlm_parse_inner(raw, &mut map);
    map
}

fn kvlm_parse_inner(raw: &str, map: &mut BTreeMap<String, Vec<String>>) {
    let space_idx = raw.find(' ');
    let newline_idx = raw.find('\n');

    match (space_idx, newline_idx) {
        (Some(spc), Some(nl)) => {
            if nl < spc {
                if nl != 0 {
                    todo!("return error here");
                }
                map.insert("message".to_string(), vec![raw[1..].to_string()]);
                return;
            }
            let key = raw[..spc].to_owned();
            let mut inner = raw;
            let mut value = String::new();
            loop {
                if let Some(n) = inner.find('\n') {
                    value += &inner[1..=n];
                    if inner.chars().nth(n + 1) != Some(' ') {
                        inner = &inner[n..];
                        break;
                    }
                    inner = &inner[n..];
                } else {
                    break;
                }
            }

            map.entry(key).or_default().push(value);
            return kvlm_parse_inner(inner, map);
        }
        _ => return,
    }
}

pub fn kvlm_serializie(map: &BTreeMap<String, Vec<String>>) -> Vec<u8> {
    let mut out = vec![];

    for (k, v) in map.iter() {
        if k == "message" {
            continue;
        }
        out.extend(k.as_bytes());
        for line in v {
            out.extend(k.as_bytes());
            out.push(b' ');
            out.extend(line.replace("\n", "\n ").as_bytes());
            out.push(b'\n');
        }
    }
    out.push(b'\n');
    out.extend(map["message"][0].as_bytes());

    out
}
