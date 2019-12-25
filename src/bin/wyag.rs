use structopt::StructOpt;
use wyag::commands::*;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab")]
enum App {
    Add,
    /// Provide content of repository objects
    CatFile(CatFile),
    Checkout,
    Commit,
    /// Compute object id and optionally create a blob from a file
    HashObject(HashObject),
    /// Initialize an empty repository
    Init(Init),
    Log,
    LsTree,
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
        _ => unimplemented!("This command has not been implemented yet!"),
    }
}
