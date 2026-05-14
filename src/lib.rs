use std::io::{BufReader, BufRead};
use std::fs::OpenOptions;
use serde::Serialize;

#[derive(Serialize)]
pub struct Word {
    pub word: String,
    pub definitions: Vec<String>,
}

pub fn read_words_and_definitions_from_file(file_path: &str) -> Vec<Word> {
    let mut words: Vec<Word> = Vec::new();

    let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
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
                let definitions = split[1..].iter().map(|s| s.trim().to_string()).collect::<Vec<String>>();
                words.push(Word { word, definitions });
            }
        }
    return words;
}