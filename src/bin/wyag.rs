use structopt::StructOpt;
use wyag::commands::*;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab")]
enum App {
    Add,
    CatFile,
    Checkout,
    Commit,
    HashObject,
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
        _ => unimplemented!("This command has not been implemented yet!"),
    }
}
