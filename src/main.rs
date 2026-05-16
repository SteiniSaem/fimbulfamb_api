#[macro_use] extern crate rocket;
mod game;
use game::Player;
use std::collections::HashMap;
use std::vec;
use rocket_cors::{CorsOptions, AllowedOrigins};
use fimbulfamb_api::{Word, generate_code, read_words_and_definitions_from_file};
use rocket::State;
use rocket::serde::json::Json;
use rand::seq::IndexedRandom;
use crate::game::Game;
use std::sync::Mutex;
use serde::Serialize;
use rocket::http::Status;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/getRandomWord")]
fn get_random_word(words: &State<Vec<Word>>) -> Result<Json<&Word>, (Status, String)> {
    let mut rng = rand::rng();
    let word = match words.choose(&mut rng){
        Some(w) => w,
        None => {
            eprintln!("Failed to choose a random word from the list");
            return Err((Status::NoContent, String::from("Word list is empty")));
        }
    };
    Ok(Json(word))
}

#[put("/createNewGame/<owner_name>")]
fn create_new_game(games: &State<Mutex<HashMap<String, Game>>>, owner_name: &str) -> String {
    let mut games = games.lock().unwrap();
    let code = generate_code();
    let new_game = Game::new(&owner_name);
    games.insert(code.clone(), new_game);
    code
}

#[put("/startGame/<id>")]
fn start_game(games: &State<Mutex<HashMap<String, Game>>>, words: &State<Vec<Word>>, id: &str) -> Result<String, (Status, String)> {
    let mut games = games.lock().unwrap();
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Game with id {} doesn't exist", id)))
    };

    let mut rng = rand::rng();
    let word: &fimbulfamb_api::Word = match words.choose(&mut rng){
        Some(w) => w,
        None => {
            eprintln!("Failed to choose a random word from the list");
            return Err((Status::NoContent, String::from("Word list is empty")));
        }
    };

    game.start_game(&word);
    Ok("Success".to_string())
}

#[get("/hasGameStarted/<id>")]
fn has_game_started(games: &State<Mutex<HashMap<String, Game>>>, id: &str) -> Result<Json<bool>, (Status, String)> {
    let games = games.lock().unwrap();
    let game = match games.get(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Game with id {} doesn't exist", id)))
    };

    Ok(Json(game.has_started))
}


#[derive(Serialize)]
struct JoinGameResponse {
    id: String,
    owner: String,
    players: Vec<Player>,
}

#[put("/joinGame/<id>/<username>")]
fn join_game(games: &State<Mutex<HashMap<String, Game>>>, id: &str, username: &str) -> Result<Json<JoinGameResponse>, (Status, String)> {
    let mut games = games.lock().unwrap();
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Game with id {} doesn't exist", id)))
    };

    if !game.has_started {
        match game.add_player(&username) {
            Ok(_) => (),
            Err(err) => return Err((Status::NotAcceptable, format!("{}", err)))
        };
        let response = JoinGameResponse {
            id: id.to_string(),
            owner: game.owner.clone(),
            players: game.players.clone(),
        };
        return Ok(Json(response))
    }
    else {
        return Err((Status::Forbidden, format!("Game not joinable")))
    }
}


#[get("/players/<id>")]
fn get_players(games: &State<Mutex<HashMap<String, Game>>>, id: &str) -> Option<Json<Vec<Player>>> {
    let games = games.lock().unwrap();
    let game = match games.get(id) {
        Some(g) => g,
        None => return None
    };
    Some(Json(game.players.clone()))
}


#[get("/currentWord/<id>")]
fn get_current_word(games: &State<Mutex<HashMap<String, Game>>>, id: &str) -> Option<Json<Word>> {
    let games = games.lock().unwrap();
    let game = match games.get(id) {
        Some(g) => g,
        None => return None
    };
    Some(Json(game.current_word.clone()))
}


#[delete("/endGame/<id>")]
fn end_game(games: &State<Mutex<HashMap<String, Game>>>, id: &str) -> String {
    let mut games = games.lock().unwrap();
    games.remove_entry(id);
    id.to_string()
}


#[launch]
fn rocket() -> _ {
    let words = read_words_and_definitions_from_file("words.txt");
    let games: Mutex<HashMap<String, Game>> = Mutex::new(HashMap::new());

    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::some_exact(&["http://localhost:5173"]))
        .to_cors()
        .unwrap();

    rocket::build()
        .attach(cors)
        .manage(words)
        .manage(games)
        .mount("/", routes![
            index,
            get_random_word,
            create_new_game,
            end_game,
            start_game,
            join_game,
            get_players,
            has_game_started,
            get_current_word
        ])
}

