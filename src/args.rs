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

    if let Some(f) = matches.get_one::<String>("filepart") {
        // Usage: kip <name>
        return Command::Get(GetData {
            filepart: f.clone(),
            print: Print(matches.get_flag("is_print")), // previously `is_present`
        });
    }
    // Usage: kip cmd [opts]
    use Command::*;
    match matches.subcommand() {
        Some(("list", m)) => List {
            filepart: m.get_one::<String>("filepart").cloned(),
        },
        Some(("get", m)) => Get(GetData {
            filepart: m.get_one::<String>("filepart").cloned().unwrap(),
            print: Print(m.get_flag("is_print")),
        }),
        Some(("add", m)) => Add(AddEditData {
            filepart: m.get_one::<String>("filepart").cloned().unwrap(),
            username: m.get_one::<String>("username").cloned(),
            print: Print(m.get_flag("is_print")),
            prompt: Prompt(m.get_flag("is_prompt")),
            notes: m.get_one::<String>("notes").cloned(),
        }),
        Some(("edit", m)) => Edit(AddEditData {
            filepart: m.get_one::<String>("filepart").cloned().unwrap(),
            username: m.get_one::<String>("username").cloned(),
            print: Print(m.get_flag("is_print")),
            prompt: Prompt(m.get_flag("is_prompt")),
            notes: m.get_one::<String>("notes").cloned(),
        }),
        Some(("del", m)) => Del {
            filepart: m.get_one::<String>("filepart").cloned().unwrap(),
        },
        Some(("gen", _)) => Gen,
        _ => panic!("unknown command"),
        // maybe: a.print_help()
    }
}

// 'static lifetime says how long the app name, description etc strings live. We get them from
// app_from_crate! macro, so they are static.
pub fn define_args() -> clap::Command {
    let filepart = clap::Arg::new("filepart")
        .help("Filename to act on, or part thereof")
        .required(true);
    let is_print = clap::Arg::new("is_print")
        .long("print")
        .action(clap::ArgAction::SetTrue)
        .help("Display password instead of copying to clipboard");

    // GET

    let cmd_get = clap::Command::new("get")
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

    let cmd_add = clap::Command::new("add")
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
        .arg(is_print.clone())
        .arg(
            clap::Arg::new("username")
                .short('u')
                .long("username")
                .num_args(0..=1)
                .help("Username to store. Will prompt if not given."),
        )
        .arg(
            clap::Arg::new("is_prompt")
                .short('p')
                .long("prompt")
                .action(clap::ArgAction::SetTrue)
                .help("Prompt for password on command line instead of generating it"),
        )
        .arg(
            clap::Arg::new("notes")
                .short('n')
                .long("notes")
                .num_args(0..=1)
                .help("Notes - anything you want"),
        );

    // EDIT

    let cmd_edit = cmd_add.clone().name("edit").about(
        "kipr edit ebay.com --username graham_king_2 --notes 'Edited notes --prompt'
Change details in an account file. Only changes the part you provide.",
    );

    // LIST

    let cmd_list = clap::Command::new("list")
        .about(
            "kipr list [filepart]
List accounts. Same as `ls` in pwd directory.
",
        )
        .arg(
            clap::Arg::new("filepart")
                .help("Prefix to limit list")
                .required(false),
        );

    // DEL

    let cmd_del = clap::Command::new("del")
        .about(
            "kipr del <filepart>
Delete an account file. Same as 'rm' in .kip/passwords/ dir.",
        )
        .arg(filepart.clone());

    // GEN

    let cmd_gen = clap::Command::new("gen").about(
        "kipr gen
Generate and print a password, and copy it to clipboard",
    );

    clap::command!()
        .arg_required_else_help(true)
        .subcommand_negates_reqs(true)
        .arg(filepart)
        .arg(is_print)
        .subcommand(cmd_get)
        .subcommand(cmd_add)
        .subcommand(cmd_edit)
        .subcommand(cmd_list)
        .subcommand(cmd_del)
        .subcommand(cmd_gen)
}
