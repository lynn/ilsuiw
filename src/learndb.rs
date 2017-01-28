extern crate redis;
extern crate regex;

use self::redis::{Commands, PipelineCommands};
use regex::Regex;
use std::cmp::max;
use std::borrow::Borrow;

fn wrap_index(index: isize, length: isize) -> isize {
    max(1, if index < 0 { length + 1 + index } else { index })
}

pub struct LearnDB { con : redis::Connection }

impl LearnDB {
    pub fn connect() -> redis::RedisResult<LearnDB> {
        let client = try!(redis::Client::open("redis://127.0.0.1/"));
        let con = try!(client.get_connection());
        Ok(LearnDB { con: con })
    }

    // Return (index, length, fact).
    fn get(&self, topic: &str, index: isize) -> redis::RedisResult<(isize, isize, Option<String>)> {
        let len: isize = try!(self.con.llen(topic));
        let j = wrap_index(index, len);

        let fact: Option<String> = try!(self.con.lindex(topic, j - 1));
        Ok((j, len, fact))
    }

    // Returns (added index, new length).
    fn add(&self, topic: &str, index: Option<isize>, fact: &str) -> redis::RedisResult<(isize, isize)> {
        let len : isize = try!(self.con.llen(topic));
        match index {
            Some(i) if i < len + 1 => {
                let j = wrap_index(i, len);

                // Insert before the j-th element.
                let (new_size,) = try!(redis::transaction(&self.con, &[topic], |pipe| {
                    let old : String = try!(self.con.lindex(topic, j - 1));
                    pipe
                        .cmd("LSET").arg(topic).arg(j - 1).arg("").ignore()
                        .linsert_before(topic, "", fact)
                        .cmd("LSET").arg(topic).arg(j).arg(old).ignore()
                        .query(&self.con)
                }));
                Ok((j, new_size))
            }
            _ => {
                let _ = try!(self.con.rpush(topic, fact));
                Ok((len + 1, len + 1))
            }
        }
    }

    // Returns the deleted entry.
    fn del(&self, topic: &str, index: isize) -> redis::RedisResult<String> {
        let len: isize = try!(self.con.llen(topic));
        let j = wrap_index(index, len);

        redis::transaction(&self.con, &[topic], |pipe| {
            pipe
                .lindex(topic, j - 1)
                .cmd("LSET").arg(topic).arg(j - 1).arg("")
                .lrem(topic, 0, "")
                .query(&self.con)
        })
    }

    fn learn_add(&self, topic: &str, index: Option<isize>, fact: &str) -> String {
        match self.add(topic, index, fact) {
            Ok((new_index, new_size)) =>
                format!("Added {}[{}/{}].", topic, new_index, new_size),
            Err(why) =>
                format!("Couldn't add: {}", why)
        }
    }

    fn learn_del(&self, topic: &str, index: isize) -> String {
        match self.del(topic, index) {
            Ok(deleted_fact) =>
                format!("Deleted {}[{}]: {}", topic, index, deleted_fact),
            Err(why) =>
                format!("Couldn't delete: {}", why)
        }
    }

    pub fn process_command(&self, cmd: &str) -> String {
        lazy_static! {
            static ref RE_LEARN_ADD: Regex =
                Regex::new(r"^learn add ([^\s\[]+)(?:\[(-?\d+)\])? (.+)$").unwrap();
            static ref RE_LEARN_DEL: Regex =
                Regex::new(r"^learn del ([^\s\[]+)\[(-?\d+)\]$").unwrap();
        }

        let cmd = cmd.trim();

        if let Some(captures) = RE_LEARN_ADD.captures(cmd) {
            let topic = &captures[1];
            let index: Option<isize> = captures.get(2).and_then(|m| {
                m.as_str().parse::<isize>().ok()
            });
            let fact = &captures[3];
            return self.learn_add(topic, index, fact);
        }

        if let Some(captures) = RE_LEARN_DEL.captures(cmd) {
            let topic = &captures[1];
            let index = captures[2].parse::<isize>().unwrap();
            return self.learn_del(topic, index);
        }

        "".to_string() // no response
    }

    pub fn process_query(&self, topic: &str) -> String {
        // Normalize the topic string.
        let topic = Regex::new(r"\s+").unwrap().replace_all(topic.trim(), "_");
        let topic = topic.borrow();

        // Try to split off an index.
        let re_index = Regex::new(r"^(.+)\[(-?\d+)\]?$").unwrap();
        let (topic, index) =
            match re_index.captures(topic) {
                Some(captures) =>
                    (captures.get(1).unwrap().as_str(), captures[2].parse::<isize>().unwrap()),
                None =>
                    (topic, 1)
            };

        // Look up the entry.
        match self.get(topic, index) {
            Ok((display_index, total, fact)) => {
                match fact {
                    Some(s) =>
                        format!("{}[{}/{}]: {}", topic, display_index, total, s),
                    None => 
                        if total == 0 {
                            format!(r"{}? ¯\\(°​_o)/¯", topic)
                        } else {
                            format!("{} has only {} entries.", topic, total)
                        }
                }
            }
            Err(why) => format!("Couldn't get: {}", why)
        }
    }
}
