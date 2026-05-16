
use serde::Serialize;
use fimbulfamb_api::Word;

#[derive(Serialize)]
#[derive(Clone)]
pub struct Player {
    pub name: String,
    pub points: i16,
}

pub struct Game {
    pub owner: String,
    pub players: Vec<Player>,
    pub turn: usize, //index to the player whose turn it currently is
    pub current_word: Word,
    pub used_words: Vec<String>,
    pub has_started: bool,
}

impl Game {
    pub fn new(owner_name: &str) -> Self {
        Self {
            owner: owner_name.to_string(),
            players: vec![Player {name: owner_name.to_string(), points: 0}],
            turn: 0,
            current_word: Word { word: String::new(), definition: String::new()},
            used_words: vec![],
            has_started: false
        }
    }

    pub fn add_player(&mut self, name: &str) -> Result<(), String> {
        if !self.players.iter().any(|p| p.name == name) {
            let new_player = Player {name: name.to_string(), points: 0};    
            Ok(self.players.push(new_player))
        }
        else{
            return Err(String::from("Name taken"))
        }
    }

    pub fn next_turn(&mut self) {
        if self.turn == self.players.len() - 1 {
            self.turn = 0;
        } else {
            self.turn += 1;
        }
    }

    pub fn start_game(&mut self, word: &Word) {
        self.has_started = true;
        self.current_word = Word {word: word.word.to_string(), definition: word.definition.to_string()};
    }
}
