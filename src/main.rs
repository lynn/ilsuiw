#[macro_use] extern crate lazy_static;
extern crate discord;
extern crate regex;

mod learndb;
use learndb::LearnDB;

use discord::Discord;
use discord::model::Event;
use std::env;
use regex::Regex;

fn main() {
	let ldb = LearnDB::connect().expect("LearnDB connection failed");

	// Log in to Discord using a bot token from the environment
	let discord = Discord::from_bot_token(
		&env::var("DISCORD_TOKEN").expect("Expected token"),
	).expect("login failed");

	// Establish and use a websocket connection
	let (mut connection, _) = discord.connect().expect("connect failed");

	let re_command = Regex::new(r"^!(.+)$").unwrap();
	let re_query = Regex::new(r"^\?\?(.+)$").unwrap();

	println!("Ready.");
	loop {
		match connection.recv_event() {
			Ok(Event::MessageCreate(message)) => {
				println!("<{}> {}", message.author.name, message.content);
				let reply =
					if let Some(captures) = re_command.captures(&message.content) {
						ldb.process_command(&captures[1])
					} else if let Some(captures) = re_query.captures(&message.content) {
						ldb.process_query(&captures[1])
					} else {
						"".to_string()
					};

				let _ = discord.send_message(&message.channel_id, &reply, "", false);
			}
			Ok(_) => {}
			Err(discord::Error::Closed(code, body)) => {
				println!("Gateway closed on us with code {:?}: {}", code, body);
				break
			}
			Err(err) => println!("Receive error: {:?}", err)
		}
	}
}

