use anyhow::anyhow;
use anyhow::Context;
use configparser::ini::Ini;
use std::env::{args, var};
use std::fs;
use std::io;
use std::io::Write;
use std::path;
use std::process;

mod config;

const DEFAULT_CONFIG: &str = "
[gnupg]
key_fingerprint:
encrypt_cmd:gpg --quiet --encrypt --sign --default-recipient-self --armor
decrypt_cmd:gpg --quiet --decrypt
[passwords]
home:~/.kip/passwords
len:19
[tools]
clip:xclip
";
// TODO: clip (above) should be 'pbcopy' if sys.platform == 'darwin'

fn main() {
    let conf = load_config();

    let args_vec = args().collect();
    let args = parse_args(args_vec);

    println!("Args: {:?}", args);

    let kip = Kip { conf, args };
    kip.cmd_get();

    // TODO WORK HERE
    // 'add' cmd

    /*
    if not args:
        return 1

    // Ensure our home directory exists
    let d = conf.dir();
    if not os.path.exists(d):
        os.makedirs(d)

    if args.cmd not in CMDS:
        args.filepart = args.cmd
        args.cmd = "get"

    retcode = CMDS[args.cmd](args)

    return retcode
    */
}

fn load_config() -> config::Config {
    let mut conf_files = Ini::new();
    let mut conf = config::Config::new();

    // built-in defaults - we always have these
    let hm = conf_files.read(String::from(DEFAULT_CONFIG));
    conf.add(hm.unwrap());

    // global defaults
    let hm = conf_files.load(&"/etc/kip/kip.conf");
    if hm.is_ok() {
        conf.add(hm.unwrap());
    }

    // user overrides
    let home = var("HOME");
    if home.is_ok() {
        let hm = conf_files.load(&(home.unwrap() + "/.kip/kip.conf"));
        if hm.is_ok() {
            conf.add(hm.unwrap());
        }
    }

    conf
}

fn parse_args(args: Vec<String>) -> Args {
    return Args {
        cmd: String::from("get"),
        username: String::from(""),
        filepart: String::from(&args[1]),
        is_print: false,
    };
}

#[derive(Debug)]
struct Args {
    cmd: String,
    username: String,
    filepart: String,
    is_print: bool,
}

impl Args {
    // Command to get a password
}

struct Kip {
    args: Args,
    conf: config::Config,
}

impl Kip {
    fn cmd_get(&self) {
        if let Err(e) = self.show(&self.args.filepart, self.args.is_print) {
            eprintln!("get failed: {}", e);
        }
    }

    // Display accounts details for name, and put password on clipboard
    fn show(&self, name: &str, is_visible: bool) -> anyhow::Result<()> {
        let entry = match self.find(name)? {
            Some(filename) => self.extract(filename)?,
            None => {
                return Err(anyhow!("File not found: {}", name));
            }
        };
        println!("{}", bold(&entry.username));
        if is_visible {
            println!("{}", entry.password);
        } else {
            self.copy_to_clipboard(&entry.password)?;
        }
        println!("{}", entry.notes);

        Ok(())
    }

    // Find a file matching 'name', prompting for user's help if needed.
    fn find(&self, name: &str) -> anyhow::Result<Option<path::PathBuf>> {
        let mut filepath = path::Path::new(&self.conf.dir()).join(name);
        if let Err(_) = filepath.metadata() {
            filepath = self.guess(name)?;
            let basename = filepath.as_path().file_name().unwrap().to_str().unwrap();
            println!("Guessing {}", bold(basename));
        }

        Ok(Some(filepath))
    }

    // Guess filename from part of name
    fn guess(&self, name: &str) -> anyhow::Result<path::PathBuf> {
        let search_glob = &format!("{}/*{}*", self.conf.dir(), name);
        let mut globs: Vec<path::PathBuf> =
            glob::glob(search_glob)?.filter_map(Result::ok).collect();
        match globs.len() {
            0 => Err(anyhow!("File not found: {}", name)),
            1 => Ok(globs.remove(0)),
            _ => {
                println!("Did you mean:");
                for (index, option) in globs.iter().enumerate() {
                    let fname = option.as_path().file_name().unwrap();
                    println!("{} - {}", index, fname.to_str().unwrap())
                }
                io::stdout().write(b"Select a choice ? ")?;
                io::stdout().flush()?;
                let mut choice_str = String::new();
                io::stdin().read_line(&mut choice_str).unwrap();
                let choice_int: usize = choice_str.trim().parse().with_context(|| {
                    format!("The choice must be a number, not '{}'", choice_str.trim())
                })?;
                if choice_int >= globs.len() {
                    return Err(anyhow!("Select a number 0-{}", globs.len() - 1));
                }
                Ok(globs.remove(choice_int))
            }
        }
    }

    // Extracts username, password and notes from given file,
    // and returns as tuple (username, password, notes).
    fn extract(&self, filename: path::PathBuf) -> anyhow::Result<Entry> {
        let enc = fs::read_to_string(filename)?;
        let contents: String = self.decrypt(&enc)?;
        let mut parts = contents.split('\n');
        let password = parts.next().unwrap();
        let username = parts.next().unwrap();
        let notes = parts.collect::<Vec<&str>>().join("");
        Ok(Entry {
            username: username.to_string(),
            password: password.to_string(),
            notes: notes,
        })
    }

    fn decrypt(&self, contents: &str) -> anyhow::Result<String> {
        execute(&self.conf.decrypt_cmd(), Some(contents), true)
    }

    // Copy given message to clipboard
    fn copy_to_clipboard(&self, msg: &str) -> anyhow::Result<String> {
        execute(&self.conf.clip_cmd(), Some(msg), false)
    }
}

#[derive(Debug)]
struct Entry {
    username: String,
    password: String,
    notes: String,
}

// Run 'cmd' in sub-process on 'data_in' and return output.
fn execute(cmd: &str, data_in: Option<&str>, has_out: bool) -> anyhow::Result<String> {
    let mut parts_iter = cmd.split(' ');
    let bin = parts_iter.next().unwrap();
    let mut proc = process::Command::new(bin);
    if has_out {
        proc.stdout(process::Stdio::piped());
    } else {
        proc.stdout(process::Stdio::null());
    }
    proc.stderr(process::Stdio::null()); // comment out for debugging
    for a in parts_iter {
        proc.arg(a);
    }
    if data_in.is_some() {
        proc.stdin(process::Stdio::piped());
    }
    let mut child = proc.spawn()?;
    if data_in.is_some() {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(data_in.unwrap().as_bytes())?;
    }
    if !has_out {
        child.wait()?;
        return Ok(String::new());
    }
    let out = child.wait_with_output()?;
    let out_str = String::from_utf8(out.stdout)?;
    Ok(out_str)
}

// 'msg' wrapped in ANSI escape sequence to make it bold
fn bold(msg: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", msg)
}
