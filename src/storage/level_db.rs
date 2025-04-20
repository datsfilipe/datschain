use rusty_leveldb::{DBIterator, LdbIterator, Options, DB};

struct Storage {
    db: DB,
}

impl Storage {
    pub fn new(path: &str) -> Storage {
        let opts = rusty_leveldb::in_memory();
        let db = DB::open(path, opts);
        match db {
            Ok(db) => Storage { db },
            Err(e) => panic!("failed to open database: {}", e),
        }
    }

    pub fn store(&mut self, key: &[u8], value: &[u8]) {
        match self.db.put(key, value) {
            Ok(_) => {}
            Err(e) => panic!("failed to store key: {}", e),
        }
    }

    pub fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        let result = self.db.get(key)?;
        Some(result)
    }
}
