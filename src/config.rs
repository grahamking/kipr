use std::collections::HashMap;
use std::env::var;
use std::path::Path;

use configparser::ini::{Ini, IniDefault};

const DEFAULT_CONFIG: &str = "
[gnupg]
key_fingerprint:
encrypt_cmd:gpg --quiet --encrypt --sign --default-recipient-self --armor
decrypt_cmd:gpg --quiet --decrypt
[passwords]
home:~/.kip/passwords
len:19
choices:abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&()*+,-./;=>?@[]^_`{|}~
[tools]
clip:
";

#[derive(Debug)]
pub struct Config(HashMap<String, HashMap<String, String>>);

impl Config {
    pub fn new() -> Config {
        let mut ini_setup = IniDefault::default();
        // customise so we can use more passwords.choices characters
        ini_setup.comment_symbols = vec![];
        ini_setup.delimiters = vec![':'];
        let mut ini = Ini::new_from_defaults(ini_setup);

        // built-in defaults - we always have these
        let hm = ini.read(DEFAULT_CONFIG.to_string());

        let mut c = Config(HashMap::new());
        c.add(hm.unwrap());
        c
    }

    pub fn dir(&self) -> &Path {
        let section_pw = self.0.get("passwords").unwrap();
        Path::new(section_pw.get("home").unwrap())
    }

    pub fn encrypt_cmd(&self) -> &str {
        self.0.get("gnupg").unwrap().get("encrypt_cmd").unwrap()
    }

    pub fn decrypt_cmd(&self) -> &str {
        self.0.get("gnupg").unwrap().get("decrypt_cmd").unwrap()
    }

    pub fn clip_cmd(&self) -> &str {
        let c = self.0.get("tools").unwrap().get("clip").unwrap();
        if !c.is_empty() {
            c // user selected
        } else if std::env::consts::OS == "macos" {
            "pbcopy"
        } else {
            "xclip"
        }
    }

    pub fn pw_len(&self) -> usize {
        self.0
            .get("passwords")
            .unwrap()
            .get("len")
            .unwrap()
            .parse()
            .unwrap()
    }

    pub fn choices(&self) -> &str {
        self.0.get("passwords").unwrap().get("choices").unwrap()
    }

    pub fn add(&mut self, v: HashMap<String, HashMap<String, Option<String>>>) {
        for (s_name, s_vals) in v {
            let mut section = self.0.remove(&s_name).unwrap_or_default();
            for (k_name, k_opt) in s_vals {
                match k_opt {
                    Some(mut k) => {
                        if k.starts_with('~') {
                            k = k.replace('~', var("HOME").as_ref().unwrap());
                        }
                        section.insert(k_name, k);
                    }
                    None => {
                        section.remove(&k_name);
                    }
                }
            }
            self.0.insert(s_name, section);
        }
    }
}
