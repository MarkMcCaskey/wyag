use std::collections::*;
use std::io::Write;
use std::path::PathBuf;
use std::{fs, str};
use structopt::StructOpt;

use crate::object::{self, ObjectSelect, ObjectType, Tree};
use crate::repository::{self, Repo};

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

fn cat_file(repo: &Repo, object: &str, _type: ObjectType) -> Result<(), String> {
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
    let mut reader = fs::OpenOptions::new()
        .read(true)
        .open(&ho.file)
        .map_err(|e| format!("Could not open file in cmd_hash_object: {:?}", e))?;
    let hash = if ho.write {
        let repo = repository::repo_find::<&str>(Some("."), false)?;
        object::object_hash(&mut reader, ho._type, Some(&repo))?
    } else {
        object::object_hash(&mut reader, ho._type, None)?
    };
    println!("{}", hash);
    Ok(())
}

#[derive(Debug, StructOpt)]
pub struct Log {
    /// The commit to inspect
    #[structopt(default_value = "HEAD")]
    commit: String,
}

pub fn cmd_log(log: &Log) -> Result<(), String> {
    let repo = repository::repo_find::<&str>(None, false)?;

    println!("digraph wyaglog{{");
    let mut set = HashSet::new();
    log_graphviz(
        &repo,
        object::object_find(&repo, &log.commit, None, false),
        &mut set,
    );
    println!("}}");

    Ok(())
}

fn log_graphviz(repo: &Repo, sha: &str, seen: &mut HashSet<String>) {
    if seen.contains(sha) {
        return;
    }
    seen.insert(sha.to_owned());

    let obj = object::object_read(repo, sha).unwrap();
    let commit = if let ObjectSelect::Commit(commit) = obj.get_specific() {
        commit
    } else {
        error!("Found non-commit at {}", sha);
        return;
    };

    if let Some(parents) = commit.get("parent") {
        for p in parents {
            println!("C_{} -> C_{};", sha, p);
            log_graphviz(repo, p, seen);
        }
    } else {
        return;
    }
}

#[derive(Debug, StructOpt)]
pub struct LsTree {
    /// The tree to show
    object: String,
}

pub fn cmd_ls_tree(tree: &LsTree) -> Result<(), String> {
    let repo = repository::repo_find::<&str>(None, false)?;
    let obj_inner = object::object_find(&repo, &tree.object, Some(ObjectType::Tree), true);
    let obj = object::object_read(&repo, obj_inner)?;
    if let ObjectSelect::Tree(t) = obj.get_specific() {
        for leaf in t.iterate_leaves() {
            println!(
                "{:6} {:?} {} {}",
                leaf.mode,
                object::object_read(&repo, &leaf.sha),
                leaf.sha,
                leaf.path.to_string_lossy()
            );
        }
    } else {
        return Err(format!("Object \"{}\" is not a tree", tree.object));
    }
    Ok(())
}

#[derive(Debug, StructOpt)]
pub struct Checkout {
    /// The commit or tree to checkout
    commit: String,
    /// The path at which to checkout
    #[structopt(parse(from_os_str))]
    path: PathBuf,
}

pub fn cmd_checkout(checkout: &Checkout) -> Result<(), String> {
    let repo = repository::repo_find::<&str>(None, false)?;
    let obj_inner = object::object_find(&repo, &checkout.commit, None, true);
    let obj = object::object_read(&repo, obj_inner)?;
    let tree = match obj.get_specific() {
        ObjectSelect::Tree(tree) => tree,
        ObjectSelect::Commit(commit) => {
            let t_obj = &commit
                .get("tree")
                .ok_or_else(|| format!("Commit \"{}\" does not have a tree!", checkout.commit))?[0];
            let t_dyn = object::object_read(&repo, &t_obj)?;
            if let ObjectSelect::Tree(tree) = t_dyn.get_specific() {
                tree
            } else {
                return Err(format!(
                    "Commit \"{}\"'s `tree` field points to object \"{}\" which is not a tree!",
                    checkout.commit, t_obj
                ));
            }
        }
        _ => {
            return Err(format!(
                "Object \"{}\" is not a commit or tree",
                checkout.commit
            ))
        }
    };
    if checkout.path.exists() {
        if !checkout.path.is_dir() {
            return Err(format!(
                "\"{}\" is not a directory.",
                checkout.path.to_string_lossy()
            ));
        }
        if checkout.path.read_dir().unwrap().count() != 0 {
            return Err(format!(
                "\"{}\" is not empty!",
                checkout.path.to_string_lossy()
            ));
        }
    } else {
        fs::create_dir(&checkout.path).map_err(|e| format!("Could not create dir: {:?}", e))?;
    }

    tree_checkout(&repo, &tree, checkout.path.clone())?;
    Ok(())
}

fn tree_checkout(repo: &Repo, tree: &Tree, path: PathBuf) -> Result<(), String> {
    for leaf in tree.iterate_leaves() {
        let obj = object::object_read(&repo, &leaf.sha)?;

        let dest = path.join(&leaf.path);

        match obj.get_specific() {
            ObjectSelect::Tree(t) => {
                fs::create_dir(&dest).expect("create dir in checkout");
                return tree_checkout(repo, &t, dest);
            }
            ObjectSelect::Blob(b) => {
                let mut f = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&dest)
                    .expect("open file in checkout");
                f.write_all(&b.data).expect("write blob in checkout");
            }
            _ => (),
        }
    }
    Ok(())
}
