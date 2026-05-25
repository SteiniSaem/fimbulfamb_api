use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Request, Data};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::{Arc, Mutex};
use rocket::State;
use std::collections::HashMap;
use crate::game::Game;

pub struct PingFairing;

#[rocket::async_trait]
impl Fairing for PingFairing {
    fn info(&self) -> Info {
        Info {
            name: "Ping",
            kind: Kind::Request,
        }
    }
    
    // þetta keyrist við hvert request og uppfærir time_of_last_acivity
    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        let games = request.guard::<&State<Arc<Mutex<HashMap<String, Game>>>>>().await;
        if let rocket::outcome::Outcome::Success(games) = games {
            // extract game id from the url path if present
            if let Some(id) = request.param::<&str>(1).and_then(|r| r.ok()) {
                if let Ok(mut games) = games.lock() {
                    if let Some(game) = games.get_mut(id) {
                        game.time_of_last_activity = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                    }
                }
            }
        }
    }
}