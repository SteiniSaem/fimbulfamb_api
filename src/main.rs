#[macro_use] extern crate rocket;
mod game;
mod middleware;
mod utils;
use middleware::PingFairing;
use game::Player;
use std::collections::HashMap;
use std::sync::MutexGuard;
use std::vec;
use rocket_cors::{CorsOptions, AllowedOrigins};
use utils::{Word, generate_code, read_words_and_definitions_from_file};
use rocket::State;
use rocket::serde::json::Json;
use crate::game::{Definition, Game};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use rocket::http::Status;
use rocket::futures::{SinkExt};
use tokio::time::{interval, Duration};
use std::time::{SystemTime, UNIX_EPOCH};


fn get_games(games: &State<Arc<Mutex<HashMap<String, Game>>>>) -> Result<MutexGuard<'_, HashMap<String, Game>>, (Status, String)> {
    match games.lock() {
        Ok(g) => Ok(g),
        Err(_) => return Err((Status::InternalServerError, String::from("Þjónn hefur endurræsts síðan þinn leikur byrjaði")))
    }
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[put("/createNewGame/<owner_name>")]
fn create_new_game(games: &State<Arc<Mutex<HashMap<String, Game>>>>, owner_name: &str, words: &State<Vec<Word>>) -> Result<String, (Status, String)> {
    let mut games = get_games(&games)?;
    let code = generate_code();
    let new_game = Game::new(&owner_name, &words);
    games.insert(code.clone(), new_game);
    Ok(code)
}

#[put("/startGame/<id>")]
fn start_game(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str) -> Result<String, (Status, String)> {
    let mut games  = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
    };

    game.start_game();
    let current_player = game.get_current_player();
    let _ = game.tx.send(format!("Start Game\t{}", current_player));
    Ok("Success".to_string())
}

#[get("/nextWord/<id>")]
fn next_word(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str) -> Result<Json<Word>, (Status, String)> {
    let mut games = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
    };

    match game.next_word() {
        Ok(w) => return Ok(Json(w)),
        Err(err) => return Err((Status::NotAcceptable, format!("{}", err)))
    };
}

#[put("/nextRound/<id>")]
fn next_round(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str,) -> Result<(), (Status, String)> {
    let mut games = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => {
            return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
        }
    };

    match game.next_round() {
        Ok(_) => {
            if game.word_is_visible {
                let _ = game.tx.send(format!("Next Round\t{}\t{}", game.get_current_player(), game.get_current_word().word));
            } else {
                let _ = game.tx.send(format!("Next Round\t{}", game.get_current_player()));
            }
        },
        Err(err) => {
            let _ = game.tx.send(format!("Error\t{}", err));
            return Err((Status::NotAcceptable, format!("{}", err)))
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
fn submit_definition(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str, body: Json<SubmitDefinitionRequest>) -> Result<(), (Status, String)> {
    let mut games = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => {
            return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
        }
    };

    if !game.open_for_submissions {
        return Err((Status::NotFound, format!("Lokað fyrir nýjar skýringar")))
    }

    let username = &body.username;
    let definition = &body.definition;

    game.add_definition(&username, &definition);

    let _ = game.tx.send(format!("Definition\t{}\t{}", username, definition));

    Ok(())
}

#[derive(Deserialize)]
struct UpdateScoresRequest {
    players: Vec<Player>
}

#[put("/updateScores/<id>", data="<body>")]
async fn update_scores(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str, body: Json<UpdateScoresRequest>) -> Result<Json<bool>, (Status, String)> {
    let mut games = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
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


#[derive(Deserialize)]
struct JoinGameRequest {
    username: String
}

#[derive(Serialize)]
struct JoinGameResponse {
    id: String,
    owner: String,
    players: Vec<Player>,
    current_player: String,
    player_definitions: Vec<Definition>,
    current_word: Word,
    joinable: bool,
    has_started: bool,
    open_for_submissions: bool,
}

#[put("/joinGame/<id>", data="<body>")]
fn join_game(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str, body: Json<JoinGameRequest>) -> Result<Json<JoinGameResponse>, (Status, String)> {
    let mut games = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
    };

    let username = &body.username;

    let current_word = if game.word_is_visible {
        Word {word: game.get_current_word().word, definition: String::new()}
    } else {
        Word {word: String::new(), definition: String::new()}
    };

    if game.joinable {
        match game.add_player(&username) {
            Ok(_) => (),
            Err(err) => return Err((Status::NotAcceptable, format!("{}", err)))
        };
        let response = JoinGameResponse {
            id: id.to_string(),
            owner: game.owner.clone(),
            players: game.players.clone(),
            current_player: game.get_current_player(),
            player_definitions: game.player_definitions.clone(),
            current_word: current_word,
            joinable: game.joinable,
            has_started: game.has_started,
            open_for_submissions: game.open_for_submissions,
        };
        let _ = game.tx.send(format!("New Player\t{}", username));
        return Ok(Json(response))
    }
    else {
        return Err((Status::Forbidden, format!("Leikur lokaður")))
    }
}


#[put("/setGameJoinability/<id>/<is_joinable>")]
fn set_game_joinability(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str, is_joinable: bool) -> Result<(), (Status, String)> {
    let mut games = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
    };

    game.joinable = is_joinable;
    Ok(())
}

#[put("/setWordVisibility/<id>/<word_is_visible>")]
fn set_word_visibility(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str, word_is_visible: bool) -> Result<(), (Status, String)> {
    let mut games = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
    };

    game.word_is_visible = word_is_visible;
    if game.word_is_visible {
        let _ = game.tx.send(format!("Show word\t{}", game.get_current_word().word));
    }
    else {
        let _ = game.tx.send(format!("Hide word"));
    }
    Ok(())
}

#[derive(Deserialize)]
struct LeaveGameRequest {
    name: String,
}
#[put("/leaveGame/<id>", data="<body>")]
fn leave_game(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str, body: Json<LeaveGameRequest>) -> Result<(), (Status, String)> {
    let mut games = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => {
            return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
        }
    };
    if game.players.len() == 1 {
        games.remove(id);
        return Ok(());
    }

    let quitter = &body.name;
    // find who next player should be if current player is removed
    let next_player_name = game.get_next_player();
    let current_player_name = game.get_current_player();
    game.remove_player(quitter);

    let _ = game.tx.send(format!("Quitter\t{}", quitter));

    if quitter == &game.owner { // owner is always the first in the players list so just put the new first one as owner
        game.owner = game.players[0].name.clone();
        let _ = game.tx.send(format!("New Owner\t{}", game.owner));
    }

    if quitter == &current_player_name { // if current player is removed, find index of next_player and set current_player_index to that
        let idx = match game.players.iter().position(|p| p.name == next_player_name) {
            Some(i) => i,
            None => return Err((Status::NotFound, String::from("Fann ekki næsta leikmann")))
        };

        game.current_player_index = idx;
        if game.word_is_visible {
            let _ = game.tx.send(format!("Next Round\t{}\t{}", game.get_current_player(), game.get_current_word().word));
        } else {
            let _ = game.tx.send(format!("Next Round\t{}", game.get_current_player()));
        }
    }
    else { // if quitter is not current player, find the index of current player after having removed qutter
        let idx = match game.players.iter().position(|p| p.name == current_player_name) {
            Some(i) => i,
            None => return Err((Status::NotFound, String::from("Fann ekki næsta leikmann")))
        };

        game.current_player_index = idx;
    }



    Ok(())
}


#[get("/players/<id>")]
fn get_players(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str) -> Result<Option<Json<Vec<Player>>>, (Status, String)> {
    let games = get_games(&games)?;
    let game = match games.get(id) {
        Some(g) => g,
        None => return Ok(None)
    };
    Ok(Some(Json(game.players.clone())))
}


#[get("/currentWord/<id>")]
fn get_current_word(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str) -> Result<Option<Json<Word>>, (Status, String)> {
    let games = get_games(&games)?;
    let game = match games.get(id) {
        Some(g) => g,
        None => return Ok(None)
    };

    Ok(Some(Json(game.get_current_word())))
}

#[put("/openForSubmissions/<id>")]
fn open_for_submissions(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str) -> Result<Json<bool>, (Status, String)> {
    let mut games = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
    };
    game.open_for_submissions = true;
    Ok(Json(true))
}

#[put("/closeForSubmissions/<id>")]
fn close_for_submissions(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str) -> Result<Json<bool>, (Status, String)> {
    let mut games = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
    };
    game.open_for_submissions = false;
    Ok(Json(true))
}

#[put("/ping/<id>")]
fn ping(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str) -> Result<Json<bool>, (Status, String)> {
    let mut games = get_games(&games)?;
    let game = match games.get_mut(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
    };
    game.update_timestamp();
    Ok(Json(true))
}

#[delete("/endGame/<id>")]
fn end_game(games: &State<Arc<Mutex<HashMap<String, Game>>>>, id: &str) -> Result<String, (Status, String)> {
    let mut games = get_games(&games)?;
    games.remove_entry(id);
    Ok(id.to_string())
}



#[get("/game/<id>/ws")]
async fn game_ws<'a>(id: &str, ws: rocket_ws::WebSocket, games: &State<Arc<Mutex<HashMap<String, Game>>>>) -> Result<rocket_ws::Channel<'static>, (Status, String)> {
    let games = get_games(&games)?;
    let game = match games.get(id) {
        Some(g) => g,
        None => return Err((Status::NotFound, format!("Enginn leikur með kóða {}", id)))
    };
    let mut rx = game.tx.subscribe();
    Ok(ws.channel(move |mut stream| Box::pin(async move {
        while let Ok(msg) = rx.recv().await {
            stream.send(msg.into()).await?;
        }
        Ok(())
    })))
}


#[rocket::main]
async fn main() {
    let words = read_words_and_definitions_from_file("words.txt");

    let games: Arc<Mutex<HashMap<String, Game>>> = Arc::new(Mutex::new(HashMap::new()));

    let figment = rocket::Config::figment();
    let allowed_origins: Vec<String> = figment.extract_inner("allowed_origins").unwrap_or_default();

    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::some_exact(&allowed_origins.iter().map(|s| s.as_str()).collect::<Vec<_>>()))
        .to_cors()
        .unwrap();

    let games_clone = Arc::clone(&games);

    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60)); // every minute
        loop {
            ticker.tick().await;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let mut games = games_clone.lock().unwrap();
            games.retain(|_, game| {
                now - game.time_of_last_activity < 3600 // keep if less than 1 hour
            });
        }
    });


    rocket::build()
        .attach(cors)
        .attach(PingFairing)
        .manage(words)
        .manage(games)
        .mount("/", routes![
            index,
            create_new_game,
            end_game,
            start_game,
            join_game,
            set_game_joinability,
            set_word_visibility,
            leave_game,
            get_players,
            get_current_word,
            next_round,
            next_word,
            open_for_submissions,
            close_for_submissions,
            ping,
            submit_definition,
            update_scores,
            game_ws
        ])
        .launch()
        .await
        .unwrap();
}

