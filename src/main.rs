use std::env::var;
use std::fs;
use std::io;
use std::io::Write;
use std::path;
use std::process;

use anyhow::anyhow;
use anyhow::Context;
use configparser::ini::Ini;
use rand::seq::{IteratorRandom, SliceRandom};
use rpassword::read_password_from_tty;

mod args;
mod config;

const DEFAULT_CONFIG: &str = "
[gnupg]
key_fingerprint:
encrypt_cmd:gpg --quiet --encrypt --sign --default-recipient-self --armor
decrypt_cmd:gpg --quiet --decrypt
[passwords]
home:~/.kipr
len:19
[tools]
clip:xclip
";
// TODO: clip (above) should be 'pbcopy' if sys.platform == 'darwin'

fn main() -> Result<(), anyhow::Error> {
    let conf = load_config();
    let args = args::parse_args();
    println!("Args: {:?}", args);

    // Ensure our home directory exists
    let d = conf.dir();
    if !d.exists() {
        fs::create_dir_all(d)?;
        println!("Created: {}", d.display());
    }

    let kip = Kip { conf, args };
    kip.run()?;

    // TODO: implement
    // - edit
    // - list
    // - del

    Ok(())
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

struct Kip {
    args: args::Args,
    conf: config::Config,
}

impl Kip {
    fn run(&self) -> anyhow::Result<()> {
        match self.args.cmd.as_str() {
            "get" => self.cmd_get(),
            "add" => self.cmd_add(),
            _ => Err(anyhow!("Unknown command")),
        }
    }

    // Command to get a password
    fn cmd_get(&self) -> anyhow::Result<()> {
        if let Err(e) = self.show(&self.args.filepart, self.args.is_print) {
            eprintln!("get failed: {}", e);
        }
        Ok(())
    }

    // Command to create a new entry
    fn cmd_add(&self) -> anyhow::Result<()> {
        let owned;
        let username = if self.args.username.is_some() {
            self.args.username.as_ref().unwrap()
        } else {
            owned = ask("Username: ")?;
            &owned
        };
        let pw = if self.args.is_prompt {
            read_password_from_tty(Some("Password: "))?
        } else {
            generate_pw(self.conf.pw_len())
        };
        self.create(&self.args.filepart, username, &self.args.notes, &pw)
    }

    fn create(&self, name: &str, username: &str, notes: &str, pw: &str) -> anyhow::Result<()> {
        let file_contents = format!(
            "{password}\n{username}\n{notes}\n",
            password = pw,
            username = username,
            notes = notes
        );
        let enc = self.encrypt(&file_contents)?;

        let dest_filename = self.conf.dir().join(name);
        if dest_filename.exists() {
            println!("WARNING: {} already exists.", dest_filename.display());
            let mut choice = ask("Overwrite name? [y|N]")?;
            choice.make_ascii_lowercase();
            if choice != "y" {
                return Err(anyhow!("Not overwriting"));
            }
        }

        let mut enc_file = fs::File::create(dest_filename)?;
        enc_file.write(enc.as_bytes())?;

        // Now show, because often we do this when signing
        // up for a site, so need pw on clipboard
        self.show(name, self.args.is_print)
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
        let mut filepath = self.conf.dir().join(name);
        if let Err(_) = filepath.metadata() {
            filepath = self.guess(name)?;
            let basename = filepath.as_path().file_name().unwrap().to_str().unwrap();
            println!("Guessing {}", bold(basename));
        }

        Ok(Some(filepath))
    }

    // Guess filename from part of name
    fn guess(&self, name: &str) -> anyhow::Result<path::PathBuf> {
        let search_glob = self.conf.dir().join(format!("*{}*", name));
        let mut globs: Vec<path::PathBuf> = glob::glob(search_glob.to_str().unwrap())?
            .filter_map(Result::ok)
            .collect();
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

    fn encrypt(&self, contents: &str) -> anyhow::Result<String> {
        execute(&self.conf.encrypt_cmd(), Some(contents), true)
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

fn ask(msg: &str) -> anyhow::Result<String> {
    io::stdout().write(msg.as_bytes())?;
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer).unwrap();
    Ok(String::from(answer.trim()))
}

// A random password of given length
fn generate_pw(length: usize) -> String {
    let choices = "abcdefghijklmnopqrstuvwxyz0123456789";
    let rng = &mut rand::thread_rng();
    let mut pw = choices.chars().choose_multiple(rng, length);
    pw.shuffle(rng);
    pw.iter().collect()
}
