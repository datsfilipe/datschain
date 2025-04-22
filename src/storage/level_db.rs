use tokio::sync::Mutex;

pub struct Storage {
    db: rusty_leveldb::DB,
    write_lock: Mutex<()>,
}

impl Storage {
    pub fn new(path: &str) -> Self {
        println!("Initializing storage at path: {}", path);

        let mut opts = rusty_leveldb::Options::default();
        opts.create_if_missing = true;

        match rusty_leveldb::DB::open(path, opts) {
            Ok(db) => {
                println!("Successfully opened database");
                Storage {
                    db,
                    write_lock: Mutex::new(()),
                }
            }
            Err(e) => {
                eprintln!("Failed to open database: {}", e);
                panic!("Database initialization failed: {}", e);
            }
        }
    }

    pub async fn store(
        &mut self,
        key: &[u8; 32],
        value: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Storing data with key: {:?}", key);

        let _guard = self.write_lock.lock().await;

        match self.db.put(key, value.as_bytes()) {
            Ok(_) => {
                println!("Successfully stored data");
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to store key: {}", e);
                Err(format!("Failed to store key: {}", e).into())
            }
        }
    }
}
