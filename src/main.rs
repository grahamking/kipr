use std::env::var;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use anyhow::anyhow;
use anyhow::Context;
use configparser::ini::Ini;
use rand::seq::{IteratorRandom, SliceRandom};
use rpassword::read_password_from_tty;

mod args;
use args::{parse_args, AddEditData, Command, GetData, Print};
mod config;
use config::Config;

fn main() -> Result<(), anyhow::Error> {
    let conf = load_config();
    let args = parse_args();
    //println!("Args: {:?}", args);

    // Ensure our home directory exists
    let d = conf.dir();
    if !d.exists() {
        fs::create_dir_all(d)?;
        println!("Created: {}", d.display());
    }

    run(conf, args)?;

    Ok(())
}

fn load_config() -> Config {
    let mut conf_files = Ini::new();

    // This includes built-in defaults
    let mut conf = Config::new();

    // global defaults
    if let Ok(hm) = conf_files.load("/etc/kip/kip.conf") {
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

fn run(conf: Config, args: Command) -> anyhow::Result<()> {
    use Command::*;
    match args {
        Add(data) => cmd_add(conf, data),
        Del { filepart } => cmd_del(conf, &filepart),
        Gen => cmd_gen(conf),
        Get(data) => cmd_get(conf, data),
        Edit(data) => cmd_edit(conf, data),
        List { filepart } => cmd_list(conf, filepart),
    }
}

// Command to get a password
fn cmd_get(conf: Config, data: GetData) -> anyhow::Result<()> {
    if let Err(e) = show(conf, &data.filepart, data.print) {
        eprintln!("get failed: {}", e);
    }
    Ok(())
}

// Command to create a new entry
fn cmd_add(conf: Config, data: AddEditData) -> anyhow::Result<()> {
    let username = match data.username {
        Some(u) => u,
        None => ask("Username: ")?,
    };
    let pw = if data.prompt.is() {
        read_password_from_tty(Some("Password: "))?
    } else {
        generate_pw(conf.choices(), conf.pw_len())
    };
    create(
        conf,
        &data.filepart,
        &username,
        data.print,
        data.notes,
        &pw,
        false,
    )
}

// Command to generate and print a password, and copy to clipboard.
// Useful if you need a secure string not attached to an account,
// or to test kipr's password gen rules.
fn cmd_gen(conf: Config) -> anyhow::Result<()> {
    let pw = generate_pw(conf.choices(), conf.pw_len());
    copy_to_clipboard(&pw, conf.clip_cmd())?;
    println!("{}", pw);
    Ok(())
}

// Command to edit an existing entry
fn cmd_edit(conf: Config, data: AddEditData) -> anyhow::Result<()> {
    let filename = find(&data.filepart, conf.dir())?;
    let entry = extract(&filename, conf.decrypt_cmd())?;
    let username = match &data.username {
        Some(m) => m,
        None => &entry.username,
    };
    let pw = if data.prompt.is() {
        read_password_from_tty(Some("Password: "))?
    } else {
        entry.password
    };
    let notes = match data.notes {
        Some(m) => m,
        None => entry.notes,
    };
    create(
        conf,
        &filename.to_string_lossy(),
        username,
        data.print,
        Some(notes),
        &pw,
        true,
    )
}

// Command to delete an existing entry
fn cmd_del(conf: Config, name: &str) -> anyhow::Result<()> {
    let filename = find(name, conf.dir())?;
    let y_n = ask(&format!("Delete {}? [y|N] ", filename.display()))?;
    if !y_n.eq_ignore_ascii_case("y") {
        println!("Not deleted");
        return Ok(());
    }
    fs::remove_file(filename).context("remove_file")
}

// Command to list accounts
fn cmd_list(conf: Config, filepart: Option<String>) -> anyhow::Result<()> {
    let prefix = filepart.unwrap_or_default();
    let list_glob = conf.dir().join(format!("{}*", prefix));
    println!("Listing {}:", bold(list_glob.to_str().unwrap()));
    let mut files: Vec<PathBuf> = glob::glob(list_glob.to_str().unwrap())?
        .filter_map(Result::ok)
        .collect();
    files.sort();
    if files.is_empty() {
        println!("No matches");
    }
    for f in files {
        println!("{}", f.as_path().file_name().unwrap().to_string_lossy());
    }
    Ok(())
}

fn create(
    conf: Config,
    name: &str,
    username: &str,
    is_print: Print,
    notes: Option<String>,
    pw: &str,
    overwrite: bool,
) -> anyhow::Result<()> {
    let n = notes.unwrap_or_default();
    let file_contents = format!(
        "{password}\n{username}\n{notes}\n",
        password = pw,
        username = username,
        notes = n
    );
    let enc = execute(conf.encrypt_cmd(), Some(&file_contents), true)?;

    let dest_filename = conf.dir().join(name);
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
    show(conf, name, is_print)
}

// Display accounts details for name, and put password on clipboard
fn show(conf: Config, name: &str, is_print: Print) -> anyhow::Result<()> {
    let filename = find(name, conf.dir())?;
    let entry = extract(&filename, conf.decrypt_cmd())?;
    println!("{}", bold(&entry.username));
    if is_print.is() {
        println!("{}", entry.password);
    } else {
        copy_to_clipboard(&entry.password, conf.clip_cmd())?;
    }
    println!("{}", entry.notes);

    Ok(())
}

// Find a file matching 'name', prompting for user's help if needed.
fn find(name: &str, in_dir: &Path) -> anyhow::Result<PathBuf> {
    let mut filepath = in_dir.join(name);
    if filepath.metadata().is_err() {
        filepath = guess(name, in_dir)?;
        let basename = filepath.as_path().file_name().unwrap().to_str().unwrap();
        println!("Guessing {}", bold(basename));
    }

    Ok(filepath)
}

// Guess filename from part of name
fn guess(name: &str, in_dir: &Path) -> anyhow::Result<PathBuf> {
    let search_glob = in_dir.join(format!("*{}*", name));
    let mut globs: Vec<PathBuf> = glob::glob(search_glob.to_str().unwrap())?
        .filter_map(Result::ok)
        .collect();
    match globs.len() {
        0 => Err(anyhow!("No match for: {}", search_glob.display())),
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
fn extract(filename: &PathBuf, decrypt_cmd: &str) -> anyhow::Result<Entry> {
    let enc = fs::read_to_string(filename)?;
    let contents = execute(decrypt_cmd, Some(&enc), true)?;
    let mut parts = contents.split('\n');
    let password = parts.next().unwrap();
    let username = parts.next().unwrap();
    let notes = parts.collect::<Vec<&str>>().join("");
    Ok(Entry {
        username: username.to_string(),
        password: password.to_string(),
        notes,
    })
}

// Copy given message to clipboard
fn copy_to_clipboard(msg: &str, clip_cmd: &str) -> anyhow::Result<String> {
    execute(clip_cmd, Some(msg), false)
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

// Read from stdin
fn ask(msg: &str) -> anyhow::Result<String> {
    io::stdout().write_all(msg.as_bytes())?;
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer).unwrap();
    Ok(answer.trim().to_string())
}

// A random password of given length.
// It uses a-z, A-Z, 0-9 and a subset of the special characters from here:
// https://owasp.org/www-community/password-special-characters
fn generate_pw(choices: &str, length: usize) -> String {
    let rng = &mut rand::thread_rng();
    let mut pw = choices.chars().choose_multiple(rng, length);
    pw.shuffle(rng);
    pw.iter().collect()
}
