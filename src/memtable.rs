use std::collections::{HashMap, BTreeMap};
use crate::wal::WriteAheadLog;
use crate::sstable::SSTable;
use std::io;
use std::fs;

pub struct MemTable {
    data: HashMap<String, String>,
    wal: WriteAheadLog,
    wal_path: String,
    max_size: usize,
    sstable_counter: usize,
}

impl MemTable {
    pub fn new(wal_path: &str) -> io::Result<Self> {
        let wal = WriteAheadLog::new(wal_path)?;
        
        let mut memtable = MemTable {
            data: HashMap::new(),
            wal,
            wal_path: wal_path.to_string(),
            max_size: 100, 
            sstable_counter: 0,
        };
        
        // Replay WAL to recover data
        memtable.recover()?;
        
        Ok(memtable)
    }

    fn recover(&mut self) -> io::Result<()> {
        self.wal.replay(|key, value| {
            match value {
                Some(v) => {
                    self.data.insert(key.to_string(), v.to_string());
                }
                None => {
                    self.data.remove(key);
                }
            }
        })
    }

    pub fn put(&mut self, key: String, value: String) -> io::Result<()> {
        // Log FIRST (durability!)
        self.wal.log_put(&key, &value)?;
        
        // Then update memory
        self.data.insert(key, value);
        
        // Check if we need to flush
        if self.data.len() >= self.max_size {
            self.flush()?;
        }
        
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<String> {
    if let Some(value) = self.data.get(key) {
        return Some(value.clone());
    }

    for i in (0..self.sstable_counter).rev() {
        let sstable_path = format!("sstable_{:06}.sst", i);
        if let Ok(Some(value)) = SSTable::get(&sstable_path, key) {
            return Some(value);
        }
    }
    
    None
}

    pub fn delete(&mut self, key: &str) -> io::Result<Option<String>> {
        self.wal.log_delete(key)?;

        let result = self.data.remove(key);
        
        Ok(result)
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.data.is_empty() {
            return Ok(());
        }

        // Convert HashMap to sorted BTreeMap
        let sorted_data: BTreeMap<String, String> = 
            self.data.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

        let sstable_path = format!("sstable_{:06}.sst", self.sstable_counter);
        self.sstable_counter += 1;

        SSTable::write(&sstable_path, &sorted_data)?;

        println!("Flushed {} entries to {}", sorted_data.len(), sstable_path);


        self.data.clear();

        // Truncate WAL (data is now in SSTable)
        fs::remove_file(&self.wal_path)?;
        self.wal = WriteAheadLog::new(&self.wal_path)?;

        Ok(())
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_put_and_get() {
        let wal_path = "test_memtable_put_get.log";
        let _ = fs::remove_file(wal_path);
        
        let mut memtable = MemTable::new(wal_path).unwrap();
        memtable.put("key1".to_string(), "value1".to_string()).unwrap();
        
        assert_eq!(memtable.get("key1"), Some("value1".to_string()));
        
        fs::remove_file(wal_path).unwrap();
    }

    #[test]
    fn test_get_nonexistent_key() {
        let wal_path = "test_memtable_nonexistent.log";
        let _ = fs::remove_file(wal_path);
        
        let memtable = MemTable::new(wal_path).unwrap();
        assert_eq!(memtable.get("nonexistent"), None);
        
        fs::remove_file(wal_path).unwrap();
    }

    #[test]
    fn test_update_existing_key() {
        let wal_path = "test_memtable_update.log";
        let _ = fs::remove_file(wal_path);
        
        let mut memtable = MemTable::new(wal_path).unwrap();
        memtable.put("key1".to_string(), "value1".to_string()).unwrap();
        memtable.put("key1".to_string(), "value2".to_string()).unwrap();
        
        assert_eq!(memtable.get("key1"), Some("value2".to_string()));
        
        fs::remove_file(wal_path).unwrap();
    }

    #[test]
    fn test_delete() {
        let wal_path = "test_memtable_delete.log";
        let _ = fs::remove_file(wal_path);
        
        let mut memtable = MemTable::new(wal_path).unwrap();
        memtable.put("key1".to_string(), "value1".to_string()).unwrap();
        
        let deleted_value = memtable.delete("key1").unwrap();
        assert_eq!(deleted_value, Some("value1".to_string()));
        assert_eq!(memtable.get("key1"), None);
        
        fs::remove_file(wal_path).unwrap();
    }

    #[test]
    fn test_delete_nonexistent_key() {
        let wal_path = "test_memtable_delete_nonexistent.log";
        let _ = fs::remove_file(wal_path);
        
        let mut memtable = MemTable::new(wal_path).unwrap();
        let result = memtable.delete("nonexistent").unwrap();
        assert_eq!(result, None);
        
        fs::remove_file(wal_path).unwrap();
    }

    #[test]
    fn test_crash_recovery() {
        let wal_path = "test_memtable_recovery.log";
        let _ = fs::remove_file(wal_path);
        
        // Simulate: write data and "crash"
        {
            let mut memtable = MemTable::new(wal_path).unwrap();
            memtable.put("key1".to_string(), "value1".to_string()).unwrap();
            memtable.put("key2".to_string(), "value2".to_string()).unwrap();
            memtable.delete("key1").unwrap();
        }
        
        // Simulate: restart and recover
        {
            let memtable = MemTable::new(wal_path).unwrap();
            assert_eq!(memtable.get("key1"), None);
            assert_eq!(memtable.get("key2"), Some("value2".to_string()));
        }
        
        fs::remove_file(wal_path).unwrap();
    }

    #[test]
    fn test_flush_to_sstable() {
        let wal_path = "test_memtable_flush.log";
        let _ = fs::remove_file(wal_path);
        
        let mut memtable = MemTable::new(wal_path).unwrap();
        
        // Add entries to trigger flush (max_size = 100)
        for i in 0..105 {
            memtable.put(format!("key_{}", i), format!("value_{}", i)).unwrap();
        }
        
        // After flush, memtable should have only 5 entries
        assert!(memtable.size() < 100);
        
        // SSTable file should exist
        assert!(std::path::Path::new("sstable_000000.sst").exists());
        
        // Clean up
        fs::remove_file(wal_path).unwrap();
        fs::remove_file("sstable_000000.sst").unwrap();
    }
}