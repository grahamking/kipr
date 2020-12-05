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

fn main() -> Result<(), anyhow::Error> {
    let conf = load_config();
    let args = args::parse_args();
    //println!("Args: {:?}", args);

    // Ensure our home directory exists
    let d = conf.dir();
    if !d.exists() {
        fs::create_dir_all(d)?;
        println!("Created: {}", d.display());
    }

    let kip = Kip { conf, args };
    kip.run()?;

    Ok(())
}

fn load_config() -> config::Config {
    let mut conf_files = Ini::new();

    // This includes built-in defaults
    let mut conf = config::Config::new();

    // global defaults
    if let Ok(hm) = conf_files.load(&"/etc/kip/kip.conf") {
        conf.add(hm);
    }

    // user overrides
    if let Ok(home) = var("HOME") {
        if let Ok(hm) = conf_files.load(&(home + "/.kip/kip.conf")) {
            conf.add(hm);
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
            "edit" => self.cmd_edit(),
            "list" => self.cmd_list(),
            "del" => self.cmd_del(),
            _ => Err(anyhow!("Unknown command")),
        }
    }

    // Command to get a password
    fn cmd_get(&self) -> anyhow::Result<()> {
        if let Err(e) = self.show(self.args.filepart.as_ref().unwrap(), self.args.is_print) {
            eprintln!("get failed: {}", e);
        }
        Ok(())
    }

    // Command to create a new entry
    fn cmd_add(&self) -> anyhow::Result<()> {
        let owned;
        let username = if self.args.username.is_some() {
            &self.args.username.as_ref().unwrap()
        } else {
            owned = ask("Username: ")?;
            &owned
        };
        let pw = if self.args.is_prompt {
            read_password_from_tty(Some("Password: "))?
        } else {
            generate_pw(self.conf.pw_len())
        };
        self.create(
            &self.args.filepart.as_ref().unwrap(),
            username,
            self.args.notes.as_ref(),
            &pw,
            false,
        )
    }

    // Command to edit an existing entry
    fn cmd_edit(&self) -> anyhow::Result<()> {
        let name = &self.args.filepart.as_ref().unwrap();
        let filename = self.find(name)?;
        let entry = match filename.as_ref() {
            Some(fname) => self.extract(&fname)?,
            None => {
                return Err(anyhow!("File not found: {}", name));
            }
        };
        let filename = filename.unwrap();
        let username = match &self.args.username {
            Some(m) => m,
            None => &entry.username,
        };
        let pw = if self.args.is_prompt {
            read_password_from_tty(Some("Password: "))?
        } else {
            entry.password
        };
        let notes = match &self.args.notes {
            Some(m) => m,
            None => &entry.notes,
        };
        self.create(
            &filename.to_string_lossy(),
            &username,
            Some(&notes),
            &pw,
            true,
        )
    }

    // Command to delete an existing entry
    fn cmd_del(&self) -> anyhow::Result<()> {
        let name = &self.args.filepart.as_ref().unwrap();
        let filename = self.find(name)?;
        if filename.is_none() {
            return Err(anyhow!("File not found: {}", name));
        }
        let filename: path::PathBuf = filename.unwrap();
        let y_n = ask(&format!("Delete {}? [y|N] ", filename.display()))?;
        if !y_n.eq_ignore_ascii_case("y") {
            println!("Not deleted");
            return Ok(());
        }
        fs::remove_file(filename).context("remove_file")
    }

    // Command to list accounts
    fn cmd_list(&self) -> anyhow::Result<()> {
        let prefix = if self.args.filepart.is_some() {
            self.args.filepart.as_ref().unwrap()
        } else {
            ""
        };
        let list_glob = self.conf.dir().join(format!("{}*", prefix));
        println!("Listing {}:", bold(list_glob.to_str().unwrap()));
        let mut files: Vec<path::PathBuf> = glob::glob(list_glob.to_str().unwrap())?
            .filter_map(Result::ok)
            .collect();
        files.sort();
        for f in files {
            println!("{}", f.as_path().file_name().unwrap().to_string_lossy());
        }
        Ok(())
    }

    fn create(
        &self,
        name: &str,
        username: &str,
        notes: Option<&String>,
        pw: &str,
        overwrite: bool,
    ) -> anyhow::Result<()> {
        let n = match notes {
            None => "",
            Some(nn) => nn,
        };
        let file_contents = format!(
            "{password}\n{username}\n{notes}\n",
            password = pw,
            username = username,
            notes = n
        );
        let enc = self.encrypt(&file_contents)?;

        let dest_filename = self.conf.dir().join(name);
        if dest_filename.exists() && !overwrite {
            println!("WARNING: {} already exists.", dest_filename.display());
            let mut choice = ask("Overwrite name? [y|N]")?;
            choice.make_ascii_lowercase();
            if choice != "y" {
                return Err(anyhow!("Not overwriting"));
            }
        }

        let mut enc_file = fs::File::create(dest_filename)?;
        enc_file.write_all(enc.as_bytes())?;

        // Now show, because often we do this when signing
        // up for a site, so need pw on clipboard
        self.show(name, self.args.is_print)
    }

    // Display accounts details for name, and put password on clipboard
    fn show(&self, name: &str, is_visible: bool) -> anyhow::Result<()> {
        let entry = match self.find(name)? {
            Some(filename) => self.extract(&filename)?,
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
        if filepath.metadata().is_err() {
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
                io::stdout().write_all(b"Select a choice ? ")?;
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
    fn extract(&self, filename: &path::PathBuf) -> anyhow::Result<Entry> {
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
    let mut child = proc
        .spawn()
        .with_context(|| format!("executing '{}'", cmd))?;
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
    io::stdout().write_all(msg.as_bytes())?;
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
