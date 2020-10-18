use std::env;
use std::fs;
use std::io::Write;
use std::path;
use std::process;

// HOME_PWD = os.path.expanduser(config.get('passwords', 'home'))
const HOME_PWD: &str = "/home/graham/.kip/passwords";

// DECRYPT_CMD = config.get('gnupg', 'decrypt_cmd')
const DECRYPT_CMD: &str = "gpg --quiet --decrypt";

fn main() {
    let args_vec = env::args().collect();
    let args = parse_args(args_vec);

    println!("Args: {:?}", args);
    args.cmd_get();

    /*
    if not args:
        return 1

    // Ensure our home directory exists
    if not os.path.exists(HOME_PWD):
        os.makedirs(HOME_PWD)

    if args.cmd not in CMDS:
        args.filepart = args.cmd
        args.cmd = "get"

    retcode = CMDS[args.cmd](args)

    return retcode
    */
}

fn parse_args(args: Vec<String>) -> Args {
    return Args {
        cmd: String::from("get"),
        username: String::from(""),
        filepart: String::from(&args[1]),
        is_print: true,
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
    fn cmd_get(&self) {
        return show(&self.filepart, self.is_print).unwrap();
    }
}

// Display accounts details for name, and put password on clipboard
fn show(name: &str, is_visible: bool) -> anyhow::Result<()> {
    println!("show: {} {}", name, is_visible);

    let entry = match find(name) {
        Some(filename) => {
            println!("found {}", filename.to_str().unwrap());
            extract(filename)
        }
        None => {
            return Err(anyhow::anyhow!("File not found: {}", name));
        }
    };

    println!("{:?}", entry);

    // TODO: work here

    /*
    print(bold(username))

    if is_visible:
        print(password)
    else:
        copy_to_clipboard(password)

    print(notes)

    return 0
    */
    Ok(())
}

// Find a file matching 'name', prompting for user's help if needed.
// Can raise IOError  - caller must handle it.
fn find(name: &str) -> Option<path::PathBuf> {
    let filename = path::Path::new(HOME_PWD).join(name);
    if let Err(_) = filename.metadata() {
        println!("dunno");
        //filename = guess(name)
        //basename = os.path.basename(filename)
        //print('Guessing {}'.format(bold(basename)))
        return None;
    }

    Some(filename)
}

#[derive(Debug)]
struct Entry {
    username: String,
    password: String,
    notes: String,
}

// Extracts username, password and notes from given file,
// and returns as tuple (username, password, notes).
fn extract(filename: path::PathBuf) -> anyhow::Result<Entry> {
    let enc = fs::read_to_string(filename)?;
    let contents: String = decrypt(&enc)?;
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

fn decrypt(contents: &str) -> anyhow::Result<String> {
    execute(DECRYPT_CMD, Some(contents))
}

// Run 'cmd' in sub-process on 'data_in' and return output.
fn execute(cmd: &str, data_in: Option<&str>) -> anyhow::Result<String> {
    let mut parts_iter = cmd.split(' ');
    let bin = parts_iter.next().unwrap();
    let mut proc = process::Command::new(bin);
    proc.stdout(process::Stdio::piped());
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
    let out = child.wait_with_output()?;
    let out_str = String::from_utf8(out.stdout)?;
    Ok(out_str)
}
