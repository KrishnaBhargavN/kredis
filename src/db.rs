use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
    sync::{Arc, Mutex},
};

use bytes::Bytes;

#[derive(Clone, Debug)]
enum DataType {
    String(Bytes),
    List(VecDeque<Bytes>),
}

type DbState = Mutex<HashMap<String, DataType>>;

#[derive(Clone)]
pub struct Db {
    shared: Arc<DbState>,
}

impl Db {
    pub fn new() -> Db {
        Db {
            shared: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get(&self, key: &str) -> Option<Bytes> {
        let state = self.shared.lock().unwrap();
        match state.get(key) {
            Some(DataType::String(val)) => Some(val.clone()),
            _ => None,
        }
    }

    pub fn set(&self, key: String, value: Bytes) {
        let mut state = self.shared.lock().unwrap();
        state.insert(key, DataType::String(value));
    }

    pub fn lpush(&self, key: String, value: Bytes) -> Result<usize, &'static str> {
        let mut state = self.shared.lock().unwrap();

        let entry = state.entry(key).or_insert(DataType::List(VecDeque::new()));
        match entry {
            DataType::List(list) => {
                list.push_front(value);
                Ok(list.len())
            }
            DataType::String(_) => {
                Err("WRONGTYPE Operation against a key holding the wrong kind of value")
            }
        }
    }

    pub fn rpop(&self, key: &str) -> Option<Bytes> {
        let mut state = self.shared.lock().unwrap();

        match state.get_mut(key) {
            Some(DataType::List(list)) => list.pop_back(),
            _ => None,
        }
    }
}
