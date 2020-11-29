use clap;
use clap::{crate_authors, crate_description, crate_name, crate_version};

#[derive(Debug)]
pub struct Args {
    pub cmd: String,
    pub username: Option<String>,
    pub filepart: Option<String>,
    pub is_print: bool,
    pub is_prompt: bool,
    pub notes: Option<String>,
}

pub fn parse_args() -> Args {
    let a = define_args();
    let matches = a.get_matches();

    if let Some(f) = matches.value_of("filepart") {
        // Usage: kip <name>
        return Args::new_get(String::from(f));
    }
    // Usage: kip cmd [opts]
    match matches.subcommand() {
        ("get", Some(m)) => {
            return Args::new_get(String::from(m.value_of("filepart").unwrap()));
        }
        ("add", Some(m)) => {
            let mut a = Args {
                cmd: String::from("add"),
                filepart: Some(String::from(m.value_of("filepart").unwrap())),
                // TODO: username should be Option<String>, prompt on add if None
                username: m.value_of("username").map(|s| String::from(s)),
                is_print: m.is_present("is_print"),
                is_prompt: m.is_present("is_prompt"),
                notes: None,
            };
            if let Some(n) = m.value_of("notes") {
                a.notes = Some(String::from(n));
            }
            return a;
        }
        ("list", Some(m)) => {
            return Args::new_list(m.value_of("filepart"));
        }
        _ => panic!("unknown command"),
        // maybe: a.print_help()
    }
}

// 'static lifetime says how long the app name, description etc strings live. We get them from
// app_from_crate! macro, so they are static.
fn define_args() -> clap::App<'static, 'static> {
    let filepart = clap::Arg::with_name("filepart")
        .help("Filename to display, or part thereof")
        .required(true);

    // GET

    let cmd_get = clap::SubCommand::with_name("get")
        .about(
            "kipr get ebay.com
 Decrypts {home}ebay.com using gpg
 Copies password (first line) to clipboard
 Echoes ebay username and notes (other lines)
",
        )
        .arg(filepart.clone())
        .arg(
            clap::Arg::with_name("is_print")
                .long("print")
                .help("Display password instead of copying to clipboard"),
        );

    // ADD

    let cmd_add = clap::SubCommand::with_name("add")
        .about(
            "kipr add ebay.com --username graham_king --notes 'And some notes'
 Generate random password (pwgen -s1 19)
 Creates file {home}ebay.com with format:
    pw
    username
    notes
 Encrypts and signs it with gpg.
",
        )
        .arg(filepart.clone())
        .arg(
            clap::Arg::with_name("username")
                .short("u")
                .long("username")
                .takes_value(true)
                .help("Username to store. Will prompt if not given."),
        )
        .arg(
            clap::Arg::with_name("is_prompt")
                .short("p")
                .long("prompt")
                .help("Prompt for password on command line instead of generating it"),
        )
        .arg(
            clap::Arg::with_name("notes")
                .short("n")
                .long("notes")
                .takes_value(true)
                .help("Notes - anything you want"),
        );

    // LIST

    let cmd_list = clap::SubCommand::with_name("list")
        .about(
            "kipr list [filepart]
List accounts. Same as `ls` in pwd directory.
",
        )
        .arg(
            clap::Arg::with_name("filepart")
                .help("Prefix to limit list")
                .required(false),
        );

    clap::app_from_crate!()
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .setting(clap::AppSettings::SubcommandsNegateReqs)
        .arg(filepart.clone())
        .subcommand(cmd_get)
        .subcommand(cmd_add)
        .subcommand(cmd_list)
}

// TODO: Args should be an enum, with different commands having different fields

impl Args {
    fn new_get(filepart: String) -> Args {
        Args {
            filepart: Some(filepart),
            cmd: String::from("get"),
            username: None,
            is_print: false,
            is_prompt: false,
            notes: None,
        }
    }
    fn new_list(filepart: Option<&str>) -> Args {
        let mut a = Args {
            filepart: None,
            cmd: String::from("list"),
            username: None,
            is_print: false,
            is_prompt: false,
            notes: None,
        };
        if filepart.is_some() {
            a.filepart = Some(String::from(filepart.unwrap()));
        }
        a
    }
}
