use std::path::PathBuf;
use structopt::StructOpt;
use std::{fs, str};

use crate::repository::{self, Repo};
use crate::object::{self, ObjectType};

#[derive(Debug, StructOpt)]
pub struct Init {
    /// Where to create the repository
    #[structopt(parse(from_os_str), default_value = ".")]
    path: PathBuf,
}

pub fn cmd_init(init: &Init) -> Result<(), String> {
    repository::repo_create(&init.path)?;

    Ok(())
}

#[derive(Debug, StructOpt)]
pub struct CatFile {
    /// The type of the object
    #[structopt(possible_values = ObjectType::variants(), name = "type")]
    _type: ObjectType,
    /// the hash string of the object to display
    object: String,
}

pub fn cmd_cat_file(cf: &CatFile) -> Result<(), String> {
    let repo = repository::repo_find::<&str>(None, false)?;
    cat_file(&repo, &cf.object, cf._type)
}

fn cat_file(repo: &Repo, object: &str, _type: ObjectType,) -> Result<(), String> {
    let obj_inner = object::object_find(repo, object, Some(_type), true);
    let obj = object::object_read(repo, obj_inner)?;

    let obj_bytes = obj.serialize();
    if let Ok(as_str) = str::from_utf8(&obj_bytes) {
        print!("{}", as_str);
    } else {
        println!("{:X?}", &obj_bytes);
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
pub struct HashObject {
    /// The type of the object
    #[structopt(possible_values = ObjectType::variants(), short = "t", long = "type", default_value = "blob")]
    _type: ObjectType,
    /// whether to write it or not
    #[structopt(short = "w")]
    write: bool,
    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

pub fn cmd_hash_object(ho: &HashObject) -> Result<(), String> {
    let mut reader = fs::OpenOptions::new().read(true).open(&ho.file).map_err(|e| format!("Could not open file in cmd_hash_object: {:?}", e))?;
    let hash =
    if ho.write {
        let repo = repository::repo_find::<&str>(Some("."), false)?;
        object::object_hash(&mut reader, ho._type, Some(&repo))?
    } else {
        object::object_hash(&mut reader, ho._type, None)?
    };
    println!("{}", hash);
    Ok(())
}
