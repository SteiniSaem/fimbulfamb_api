
use serde::{Deserialize, Serialize};
use fimbulfamb_api::Word;
use tokio::sync::broadcast;
use rand::seq::SliceRandom;

#[derive(Deserialize)]
#[derive(Serialize)]
#[derive(Clone)]
pub struct Player {
    pub name: String,
    pub points: i16,
}

pub struct Game {
    pub owner: String,
    pub players: Vec<Player>,
    pub current_player_index: usize, // index to the player whose turn it currently is
    pub word_pool: Vec<Word>,
    pub current_word_index: usize, // index to the current word in the word pool
    pub has_started: bool,
    pub open_for_submissions: bool,
    pub tx: broadcast::Sender<String>
}

impl Game {
    pub fn new(owner_name: &str, words: &Vec<Word>) -> Self {
        let mut rng = rand::rng();
        let mut word_pool = words.clone();
        word_pool.shuffle(&mut rng);

        let (tx, _) = broadcast::channel(16);
        Self {
            owner: owner_name.to_string(),
            players: vec![Player {name: owner_name.to_string(), points: 0}],
            current_player_index: 0,
            word_pool: word_pool,
            current_word_index: 0,
            open_for_submissions: true,
            has_started: false,
            tx: tx
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

    pub fn next_round(&mut self) -> Result<(), String> {
        if self.current_player_index == self.players.len() - 1 {
            self.current_player_index = 0;
        } else {
            self.current_player_index += 1;
        }

        if self.current_word_index == self.word_pool.len() -1 {
            return Err(String::from("No more words"))
        }
        else {
            self.current_word_index += 1
        }
        Ok(())
    }

    pub fn update_player_score(&mut self, name: &str, points: i16) -> Result<(), String>{
        if let Some(player) = self.players.iter_mut().find(|p| p.name == name){
            player.points = points;
            return Ok(())
        }
        else {
            return Err(format!("No player with name {}", name))
        }
    }

    pub fn get_current_player(&self) -> String{
        let current_player = &self.players[self.current_player_index];
        return current_player.name.to_string()
    }

    pub fn get_current_word(&self) -> Word {
        return self.word_pool[self.current_word_index].clone();
    }

    pub fn next_word(&mut self) -> Result<Word, String> {
        if self.current_word_index == self.word_pool.len() - 1 {
            return Err(String::from("No more words"))
        }
        else {
            self.current_word_index += 1;
            return Ok(self.get_current_word());
        }
    }

    pub fn start_game(&mut self) {
        self.has_started = true;
    }
}
