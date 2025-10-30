use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};

pub struct WriteAheadLog {
    file: File,
    path: String,
}

impl WriteAheadLog {
    pub fn new(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        
        Ok(WriteAheadLog {
            file,
            path: path.to_string(),
        })
    }

    pub fn log_put(&mut self, key: &str, value: &str) -> io::Result<()> {
        let entry = format!("PUT,{},{}\n", key, value);
        self.file.write_all(entry.as_bytes())?;
        self.file.sync_all()?;
        Ok(())
    }

    pub fn log_delete(&mut self, key: &str) -> io::Result<()> {
        let entry = format!("DELETE,{}\n", key);
        self.file.write_all(entry.as_bytes())?;
        self.file.sync_all()?;
        Ok(())
    }

    pub fn replay<F>(&self, mut callback: F) -> io::Result<()>
    where
        F: FnMut(&str, Option<&str>),
    {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split(',').collect();

            match parts[0] {
                "PUT" if parts.len() == 3 => {
                    callback(parts[1], Some(parts[2]));
                }
                "DELETE" if parts.len() == 2 => {
                    callback(parts[1], None);
                }
                _ => {                 
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_wal_log_and_replay() {
        let wal_path = "test_wal.log";
        
        let _ = fs::remove_file(wal_path);

        {
            let mut wal = WriteAheadLog::new(wal_path).unwrap();
            wal.log_put("key1", "value1").unwrap();
            wal.log_put("key2", "value2").unwrap();
            wal.log_delete("key1").unwrap();
        }

        let wal = WriteAheadLog::new(wal_path).unwrap();
        let mut operations = Vec::new();

        wal.replay(|key, value| {
            operations.push((key.to_string(), value.map(|v| v.to_string())));
        }).unwrap();

        assert_eq!(operations.len(), 3);
        assert_eq!(operations[0], ("key1".to_string(), Some("value1".to_string())));
        assert_eq!(operations[1], ("key2".to_string(), Some("value2".to_string())));
        assert_eq!(operations[2], ("key1".to_string(), None));

        fs::remove_file(wal_path).unwrap();
    }
}