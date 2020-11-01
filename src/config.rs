use std::collections::HashMap;
use std::env::var;

#[derive(Debug)]
pub struct Config(HashMap<String, HashMap<String, String>>);

impl Config {
    pub fn new() -> Config {
        Config(HashMap::new())
    }

    pub fn dir(&self) -> String {
        // TODO &str pointing into our hashmap
        let section_pw = self.0.get("passwords").unwrap();
        String::from(section_pw.get("home").unwrap())
    }

    pub fn decrypt_cmd(&self) -> String {
        String::from(self.0.get("gnupg").unwrap().get("decrypt_cmd").unwrap())
    }

    pub fn clip_cmd(&self) -> String {
        String::from(self.0.get("tools").unwrap().get("clip").unwrap())
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
