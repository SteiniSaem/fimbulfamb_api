use std::io::{BufReader, BufRead};
use std::fs::OpenOptions;
use serde::Serialize;
use rand::seq::IndexedRandom;


#[derive(Serialize)]
#[derive(Clone)]
pub struct Word {
    pub word: String,
    pub definition: String//definitions: Vec<String>,
}

impl Word {
    pub fn empty() -> Word {
        return Word { word: String::new(), definition: String::new()}
    }
}

pub fn read_words_and_definitions_from_file(file_path: &str) -> Vec<Word> {
    let mut words: Vec<Word> = Vec::new();

    let file = match OpenOptions::new()
        .read(true)
        .open(file_path) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Error opening file: {}", e);
                return words;
            }
        };


    let reader = BufReader::new(file);

    for (i, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("reading line {} failed: {}", i + 1, e);
                continue;
            }
        };
        
        let split: Vec<&str> = line.split('\t').collect();
        if split.len() > 1 {
            let word = split[0].to_string();
            let definition = split[1].to_string();
            words.push(Word { word: word, definition: definition });
        }
    }

    return words;
}


pub fn generate_code() -> String {
    let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect();
    let mut rng = rand::rng();
    let mut code_vec = vec![];
    while code_vec.len() < 6 {
        let c = match chars.choose(&mut rng) {
            Some(c) => c,
            None => continue
        };
        code_vec.push(c)
    }

    code_vec.into_iter().collect()
}