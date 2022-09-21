use clap;

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
        return Args::new_get(String::from(f), matches.is_present("is_print"));
    }
    // Usage: kip cmd [opts]
    match matches.subcommand() {
        Some(("list", m)) => {
            return Args::new_list(m.value_of("filepart"));
        }
        Some(("get", m)) => {
            return Args::new_get(
                String::from(m.value_of("filepart").unwrap()),
                m.is_present("is_print"),
            );
        }
        Some((add_edit, m)) if add_edit == "add" || add_edit == "edit" => {
            let mut a = Args {
                cmd: String::from(add_edit),
                filepart: Some(String::from(m.value_of("filepart").unwrap())),
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
        Some(("del", m)) => {
            return Args {
                filepart: Some(String::from(m.value_of("filepart").unwrap())),
                cmd: String::from("del"),
                username: None,
                is_print: false,
                is_prompt: false,
                notes: None,
            }
        }
        _ => panic!("unknown command"),
        // maybe: a.print_help()
    }
}

// 'static lifetime says how long the app name, description etc strings live. We get them from
// app_from_crate! macro, so they are static.
fn define_args() -> clap::App<'static> {
    let filepart = clap::Arg::with_name("filepart")
        .help("Filename to act on, or part thereof")
        .required(true);
    let is_print = clap::Arg::with_name("is_print")
        .long("print")
        .help("Display password instead of copying to clipboard");

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
        .arg(is_print.clone());

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
                .short('u')
                .long("username")
                .takes_value(true)
                .help("Username to store. Will prompt if not given."),
        )
        .arg(
            clap::Arg::with_name("is_prompt")
                .short('p')
                .long("prompt")
                .help("Prompt for password on command line instead of generating it"),
        )
        .arg(
            clap::Arg::with_name("notes")
                .short('n')
                .long("notes")
                .takes_value(true)
                .help("Notes - anything you want"),
        );

    // EDIT

    let cmd_edit = cmd_add
        .clone()
        .name("edit")
        .about("kipr edit ebay.com --username graham_king_2 --notes 'Edited notes'");

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

    // DEL

    let cmd_del = clap::SubCommand::with_name("del")
        .about("kipr del <filepart>")
        .arg(filepart.clone());

    clap::app_from_crate!()
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .setting(clap::AppSettings::SubcommandsNegateReqs)
        .arg(filepart)
        .arg(is_print)
        .subcommand(cmd_get)
        .subcommand(cmd_add)
        .subcommand(cmd_edit)
        .subcommand(cmd_list)
        .subcommand(cmd_del)
}

// TODO: Args should maybe an enum, with different commands having different fields

impl Args {
    fn new_get(filepart: String, is_print: bool) -> Args {
        Args {
            filepart: Some(filepart),
            cmd: String::from("get"),
            username: None,
            is_print: is_print,
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
