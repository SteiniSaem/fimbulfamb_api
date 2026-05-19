#[macro_use] extern crate rocket;
mod game;
use game::Player;
use std::collections::HashMap;
use std::vec;
use rocket_cors::{CorsOptions, AllowedOrigins};
use fimbulfamb_api::{Word, generate_code, read_words_and_definitions_from_file};
use rocket::State;
use rocket::serde::json::Json;
use crate::game::Game;
use std::sync::Mutex;
use serde::{Serialize, Deserialize};
use rocket::http::Status;
use rocket::futures::{SinkExt};

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[put("/createNewGame/<owner_name>")]
fn create_new_game(games: &State<Mutex<HashMap<String, Game>>>, owner_name: &str, words: &State<Vec<Word>>) -> String {
    let mut games = games.lock().unwrap();
    let code = generate_code();
    let new_game = Game::new(&owner_name, &words);
    games.insert(code.clone(), new_game);
    code
}

#[put("/startGame/<id>")]
fn start_game(games: &State<Mutex<HashMap<String, Game>>>, id: &str) -> Result<String, (Status, String)> {
    let mut games = games.lock().unwrap();
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Game with id {} doesn't exist", id)))
    };

    game.start_game();
    let current_player = game.get_current_player();
    let _ = game.tx.send(format!("Start Game\t{}", current_player));
    Ok("Success".to_string())
}

#[get("/nextWord/<id>")]
fn next_word(games: &State<Mutex<HashMap<String, Game>>>, id: &str) -> Result<Json<Word>, (Status, String)> {
    let mut games = games.lock().unwrap();
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Game with id {} doesn't exist", id)))
    };

    match game.next_word() {
        Ok(w) => return Ok(Json(w)),
        Err(err) => return Err((Status::NoContent, format!("{}", err)))
    };
}

#[put("/nextRound/<id>")]
fn next_round(games: &State<Mutex<HashMap<String, Game>>>, id: &str,) -> Result<(), (Status, String)> {
    let mut games = games.lock().unwrap();
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => {
            return Err((Status::NotFound, format!("Game with id {} doesn't exist", id)))
        }
    };

    match game.next_round() {
        Ok(_) => {
            let _ = game.tx.send(format!("Next Round\t{}", game.get_current_player()));
        },
        Err(err) => {
            let _ = game.tx.send(format!("Error\t{}", err));
            return Err((Status::NoContent, format!("{}", err)))
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct SubmitDefinitionRequest {
    username: String,
    definition: String
}

#[put("/submitDefinition/<id>", data="<body>")]
fn submit_definition(games: &State<Mutex<HashMap<String, Game>>>, id: &str, body: Json<SubmitDefinitionRequest>) -> Result<(), (Status, String)> {
    let mut games = games.lock().unwrap();
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => {
            return Err((Status::NotFound, format!("Game with id {} doesn't exist", id)))
        }
    };

    if !game.open_for_submissions {
        return Err((Status::NotFound, format!("Lokað fyrir nýjar skýringar")))
    }

    let username = &body.username;
    let definition = &body.definition;

    let _ = game.tx.send(format!("Definition\t{}\t{}", username, definition));

    Ok(())
}

#[derive(Deserialize)]
struct UpdateScoresRequest {
    players: Vec<Player>
}

#[put("/updateScores/<id>", data="<body>")]
fn update_scores(games: &State<Mutex<HashMap<String, Game>>>, id: &str, body: Json<UpdateScoresRequest>) -> Result<Json<bool>, (Status, String)> {
    let mut games = games.lock().unwrap();
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Game with id {} doesn't exist", id)))
    };

    //update scores in the game object
    for player in body.players.iter() {
        match game.update_player_score(&player.name, player.points) {
            Ok(_) => (),
            Err(err) => return Err((Status::NotFound, format!("{}", err)))
        }
    };

    // create string to send to frontend
    let scores: Vec<String> = body.players.iter()
        .map(|p| format!("{}\t{}", p.name, p.points))
        .collect();

    let message = format!("Scores\t{}", scores.join("\t"));

    let _ = game.tx.send(message);
    
    Ok(Json(true))
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
        let _ = game.tx.send(format!("New Player\t{}", username));
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
    Some(Json(game.get_current_word()))
}

#[put("/openForSubmissions/<id>")]
fn open_for_submissions(games: &State<Mutex<HashMap<String, Game>>>, id: &str) -> Result<Json<bool>, (Status, String)> {
    let mut games = games.lock().unwrap();
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Game with id {} doesn't exist", id)))
    };
    game.open_for_submissions = true;
    Ok(Json(true))
}

#[put("/closeForSubmissions/<id>")]
fn close_for_submissions(games: &State<Mutex<HashMap<String, Game>>>, id: &str) -> Result<Json<bool>, (Status, String)> {
    let mut games = games.lock().unwrap();
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Game with id {} doesn't exist", id)))
    };
    game.open_for_submissions = false;
    Ok(Json(true))
}

#[delete("/endGame/<id>")]
fn end_game(games: &State<Mutex<HashMap<String, Game>>>, id: &str) -> String {
    let mut games = games.lock().unwrap();
    games.remove_entry(id);
    id.to_string()
}


#[get("/game/<id>/ws")]
async fn game_ws<'a>(id: &str, ws: rocket_ws::WebSocket, games: &State<Mutex<HashMap<String, Game>>>) -> rocket_ws::Channel<'static> {
    let mut rx = games.lock().unwrap().get(id).unwrap().tx.subscribe();
    ws.channel(move |mut stream| Box::pin(async move {
        while let Ok(msg) = rx.recv().await {
            stream.send(msg.into()).await?;
        }
        Ok(())
    }))
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
            create_new_game,
            end_game,
            start_game,
            join_game,
            get_players,
            has_game_started,
            get_current_word,
            next_round,
            next_word,
            open_for_submissions,
            close_for_submissions,
            submit_definition,
            update_scores,
            game_ws
        ])
}

