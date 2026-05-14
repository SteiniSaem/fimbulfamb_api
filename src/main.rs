#[macro_use] extern crate rocket;
use fimbulfamb_api::{read_words_and_definitions_from_file, Word};
use rocket::State;
use rocket::serde::json::Json;
use rand::seq::IndexedRandom;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/getRandomWord")]
fn get_random_word(words: &State<Vec<Word>>) -> Option<Json<&Word>> {
    let mut rng = rand::rng();
    let word = match words.choose(&mut rng){
        Some(w) => w,
        None => {
            eprintln!("Failed to choose a random word from the list");
            return None;
        }
    };
    Some(Json(word))
}

#[launch]
fn rocket() -> _ {
    let words = read_words_and_definitions_from_file("words.txt");

    rocket::build()
        .manage(words)
        .mount("/", routes![index, get_random_word])
}

