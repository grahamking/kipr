use std::collections::HashMap;
use std::env::var;
use std::path::Path;

#[derive(Debug)]
pub struct Config(HashMap<String, HashMap<String, String>>);

impl Config {
    pub fn new() -> Config {
        Config(HashMap::new())
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
        self.0.get("tools").unwrap().get("clip").unwrap()
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

    pub fn add(&mut self, v: HashMap<String, HashMap<String, Option<String>>>) {
        for (s_name, s_vals) in v {
            let mut section = self.0.remove(&s_name).unwrap_or_else(|| HashMap::new());
            for (k_name, k_opt) in s_vals {
                if k_opt.is_some() {
                    let mut k = k_opt.unwrap();
                    if k.starts_with("~") {
                        k = k.replace("~", &var("HOME").unwrap());
                    }
                    section.insert(String::from(k_name), k);
                } else {
                    section.remove(&k_name);
                }
            }
            self.0.insert(String::from(s_name), section);
        }
    }
}
