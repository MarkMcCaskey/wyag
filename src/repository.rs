use ini::Ini;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Repo {
    worktree: PathBuf,
    gitdir: PathBuf,
    conf: Ini,
}

impl Repo {
    pub fn new(path: PathBuf, force: bool) -> Result<Self, String> {
        trace!("Repo::new, {:?}", path);
        let gitdir = path.join(".git");

        let mut repo = Self {
            worktree: path,
            gitdir,
            conf: Ini::default(),
        };
        let config_path = repo.repo_file("config", false);
        match config_path.and_then(|c| Ini::load_from_file(c).map_err(|e| e.to_string())) {
            Ok(ini) => repo.conf = ini,
            Err(_) if force => (),
            Err(e) => return Err(format!("Failed to create repository object: {}", e)),
        };

        // validate version if not forcing
        if !force {
            let version = repo
                .conf
                .get_from(Some("core"), "repositoryformatversion")
                .and_then(|v| v.parse::<usize>().ok())
                .ok_or_else(|| format!("Could not get repo format version"))?;

            if version != 0 {
                return Err(format!("Unsupported repo version found: {}", version));
            }
        }

        Ok(repo)
    }

    fn repo_path<P>(&self, path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.gitdir.join(path)
    }

    pub fn repo_file<P>(&self, path: P, mkdir: bool) -> Result<PathBuf, String>
    where
        P: AsRef<Path>,
    {
        let mut pb: PathBuf = path.as_ref().to_owned();
        pb.pop();
        self.repo_dir(pb, mkdir)?;
        Ok(self.repo_path(path))
    }

    fn repo_dir<P>(&self, path: P, mkdir: bool) -> Result<PathBuf, String>
    where
        P: AsRef<Path>,
    {
        let p = self.repo_path(path);

        if p.exists() {
            if p.is_dir() {
                return Ok(p);
            } else {
                return Err(format!("{:?} is not a directory", p));
            }
        }

        if mkdir {
            trace!("Creating directories: {:?}", &p);
            fs::create_dir_all(&p).map_err(|e| format!("Could not create directories: {:?}", e))?;
            Ok(p)
        } else {
            Err(format!("Error: path {:?} does not exist", p))
        }
    }
}

fn repo_default_config() -> Ini {
    let mut ret = Ini::new();
    ret.with_section(Some("core".to_owned()))
        .set("repositoryformatversion", "0")
        .set("filemode", "false")
        .set("bare", "false");

    ret
}

pub fn repo_create<P>(path: P) -> Result<Repo, String>
where
    P: AsRef<Path>,
{
    let pb: PathBuf = path.as_ref().to_owned();
    trace!("repo_create: {:?}", &pb);

    let repo = Repo::new(pb, true)?;
    trace!("REPO CREATED");

    if repo.worktree.exists() {
        if fs::read_dir(&repo.worktree)
            .map_err(|e| format!("Error reading worktree entries: {:?}", e))?
            .count()
            != 0
        {
            return Err(format!(
                "Error: repo worktree {:?} is not empty!",
                repo.worktree
            ));
        }
    } else {
        fs::create_dir_all(&repo.worktree)
            .map_err(|e| format!("Could not create directories: {:?}", e))?;
    }
    repo.repo_dir("branches", true)?;
    repo.repo_dir("objects", true)?;
    repo.repo_dir("refs/tags", true)?;
    repo.repo_dir("refs/heads", true)?;

    {
        let mut oo = fs::OpenOptions::new();
        let mut writer = oo
            .write(true)
            .create(true)
            .open(repo.repo_file("description", false)?)
            .map_err(|e| format!("could not open file description: {:?}", e))?;
        writer
            .write_all(
                b"Unnamed repository; edit this file 'description' to name the repository.\n",
            )
            .unwrap();
    }

    {
        let mut oo = fs::OpenOptions::new();
        let mut writer = oo
            .write(true)
            .create(true)
            .open(repo.repo_file("HEAD", false)?)
            .map_err(|e| format!("could not open file HEAD: {:?}", e))?;
        writer.write_all(b"ref: refs/heads/master\n").unwrap();
    }

    {
        let config_file_path = repo.repo_file("config", false)?;
        let config = repo_default_config();
        config
            .write_to_file(config_file_path)
            .expect("write config to FS");
    }

    Ok(repo)
}

pub fn repo_find<P>(path: Option<P>, required: bool) -> Result<Repo, String>
where
    P: AsRef<Path>,
{
    let pb: PathBuf = if let Some(p) = path {
        p.as_ref().to_owned()
    } else {
        Path::new(".").to_owned()
    };
    let mut pb = pb
        .canonicalize()
        .expect("Could not canonicalize path in repo_find");

    let with_git = pb.join(".git");

    if with_git.is_dir() {
        return Repo::new(pb, false);
    }

    if pb.pop() {
        repo_find(Some(pb), required)
    } else {
        Err("At root, could not find git repo".to_owned())
    }
}
