use structopt::StructOpt;
use wyag::commands::*;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab")]
enum App {
    Add,
    /// Provide content of repository objects
    CatFile(CatFile),
    /// Checkout a commit inside a directory
    Checkout(Checkout),
    Commit,
    /// Compute object id and optionally create a blob from a file
    HashObject(HashObject),
    /// Initialize an empty repository
    Init(Init),
    /// Display history of a given commit
    Log(Log),
    /// Pretty print a tree object
    LsTree(LsTree),
    Merge,
    Rebase,
    RevParse,
    Rm,
    ShowRef,
    Tag,
}

fn main() -> Result<(), String> {
    env_logger::init();
    let args = App::from_args();

    match args {
        App::Init(init) => cmd_init(&init),
        App::CatFile(cf) => cmd_cat_file(&cf),
        App::HashObject(ho) => cmd_hash_object(&ho),
        App::Log(log) => cmd_log(&log),
        App::LsTree(ls_tree) => cmd_ls_tree(&ls_tree),
        App::Checkout(checkout) => cmd_checkout(&checkout),
        _ => unimplemented!("This command has not been implemented yet!"),
    }
}
