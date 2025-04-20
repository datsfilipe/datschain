use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct Storage {
    db: rusty_leveldb::DB,
    write_lock: Mutex<()>,
}

impl Storage {
    pub fn new(path: &str) -> Self {
        let opts = rusty_leveldb::Options::default();
        let db = rusty_leveldb::DB::open(path, opts).expect("Failed to open database");

        Storage {
            db,
            write_lock: Mutex::new(()),
        }
    }

    pub async fn store(
        &mut self,
        key: &[u8; 32],
        value: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _guard = self.write_lock.lock().await;

        self.db
            .put(key, value.as_bytes())
            .map_err(|e| format!("Failed to store key: {}", e).into())
    }

    pub async fn get(
        &mut self,
        key: &[u8; 32],
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        match self.db.get(key) {
            Some(bytes) => {
                let value = String::from_utf8(bytes)
                    .map_err(|e| format!("Invalid UTF-8 sequence: {}", e))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub async fn batch_write(
        &mut self,
        entries: HashMap<[u8; 32], String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _guard = self.write_lock.lock().await;

        for (key, value) in entries {
            self.db
                .put(&key, value.as_bytes())
                .map_err(|e| format!("Failed to store key in batch: {}", e))?;
        }

        Ok(())
    }
}
