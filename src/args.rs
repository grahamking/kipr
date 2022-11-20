pub struct AddEditData {
    pub filepart: String,
    pub username: Option<String>,
    pub print: Print,
    pub prompt: Prompt,
    pub notes: Option<String>,
}

pub struct GetData {
    pub filepart: String,
    pub print: Print,
}

pub enum Command {
    Add(AddEditData),
    Del { filepart: String },
    Edit(AddEditData),
    Gen,
    Get(GetData),
    List { filepart: Option<String> },
}

pub struct Print(bool);
impl Print {
    pub fn is(&self) -> bool {
        self.0
    }
}

pub struct Prompt(bool);
impl Prompt {
    pub fn is(&self) -> bool {
        self.0
    }
}

pub fn parse_args() -> Command {
    let a = define_args();
    let matches = a.get_matches();

    if let Some(f) = matches.value_of("filepart") {
        // Usage: kip <name>
        return Command::Get(GetData {
            filepart: f.to_string(),
            print: Print(matches.is_present("is_print")),
        });
    }
    // Usage: kip cmd [opts]
    use Command::*;
    match matches.subcommand() {
        Some(("list", m)) => List {
            filepart: m.value_of("filepart").map(String::from),
        },
        Some(("get", m)) => Get(GetData {
            filepart: m.value_of("filepart").unwrap().to_string(),
            print: Print(m.is_present("is_print")),
        }),
        Some(("add", m)) => Add(AddEditData {
            filepart: m.value_of("filepart").unwrap().to_string(),
            username: m.value_of("username").map(String::from),
            print: Print(m.is_present("is_print")),
            prompt: Prompt(m.is_present("is_prompt")),
            notes: m.value_of("notes").map(String::from),
        }),
        Some(("edit", m)) => Edit(AddEditData {
            filepart: m.value_of("filepart").unwrap().to_string(),
            username: m.value_of("username").map(String::from),
            print: Print(m.is_present("is_print")),
            prompt: Prompt(m.is_present("is_prompt")),
            notes: m.value_of("notes").map(String::from),
        }),
        Some(("del", m)) => Del {
            filepart: m.value_of("filepart").unwrap().to_string(),
        },
        Some(("gen", _)) => Gen,
        _ => panic!("unknown command"),
        // maybe: a.print_help()
    }
}

// 'static lifetime says how long the app name, description etc strings live. We get them from
// app_from_crate! macro, so they are static.
pub fn define_args() -> clap::App<'static> {
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

    let cmd_edit = cmd_add.clone().name("edit").about(
        "kipr edit ebay.com --username graham_king_2 --notes 'Edited notes --prompt'
Change details in an account file. Only changes the part you provide.",
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

    // DEL

    let cmd_del = clap::SubCommand::with_name("del")
        .about(
            "kipr del <filepart>
Delete an account file. Same as 'rm' in .kip/passwords/ dir.",
        )
        .arg(filepart.clone());

    // GEN

    let cmd_gen = clap::SubCommand::with_name("gen").about(
        "kipr gen
Generate and print a password, and copy it to clipboard",
    );

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
        .subcommand(cmd_gen)
}
