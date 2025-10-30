use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

pub struct SSTable;

impl SSTable {
    /// Write a sorted key-value map to an SSTable file
    pub fn write(path: &str, data: &BTreeMap<String, String>) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        let num_entries = data.len() as u32;
        file.write_all(&num_entries.to_le_bytes())?;

        for (key, value) in data.iter() {
            let key_bytes = key.as_bytes();
            file.write_all(&(key_bytes.len() as u32).to_le_bytes())?;
            file.write_all(key_bytes)?;

            let value_bytes = value.as_bytes();
            file.write_all(&(value_bytes.len() as u32).to_le_bytes())?;
            file.write_all(value_bytes)?;
        }

        file.sync_all()?;
        Ok(())
    }

    pub fn read(path: &str) -> io::Result<BTreeMap<String, String>> {
        if !Path::new(path).exists() {
            return Ok(BTreeMap::new());
        }

        let mut file = File::open(path)?;
        let mut data = BTreeMap::new();

        let mut num_entries_bytes = [0u8; 4];
        file.read_exact(&mut num_entries_bytes)?;
        let num_entries = u32::from_le_bytes(num_entries_bytes);

        for _ in 0..num_entries {
            let mut key_len_bytes = [0u8; 4];
            file.read_exact(&mut key_len_bytes)?;
            let key_len = u32::from_le_bytes(key_len_bytes) as usize;

            let mut key_bytes = vec![0u8; key_len];
            file.read_exact(&mut key_bytes)?;
            let key = String::from_utf8(key_bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            let mut value_len_bytes = [0u8; 4];
            file.read_exact(&mut value_len_bytes)?;
            let value_len = u32::from_le_bytes(value_len_bytes) as usize;

            let mut value_bytes = vec![0u8; value_len];
            file.read_exact(&mut value_bytes)?;
            let value = String::from_utf8(value_bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            data.insert(key, value);
        }

        Ok(data)
    }

    /// Get a value by key from an SSTable file
    pub fn get(path: &str, key: &str) -> io::Result<Option<String>> {
        let data = Self::read(path)?;
        Ok(data.get(key).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_write_and_read_sstable() {
        let path = "test_sstable.sst";
        let _ = fs::remove_file(path);

        let mut data = BTreeMap::new();
        data.insert("key1".to_string(), "value1".to_string());
        data.insert("key2".to_string(), "value2".to_string());
        data.insert("key3".to_string(), "value3".to_string());

        SSTable::write(path, &data).unwrap();

        // Read it back
        let read_data = SSTable::read(path).unwrap();

        assert_eq!(read_data.len(), 3);
        assert_eq!(read_data.get("key1"), Some(&"value1".to_string()));
        assert_eq!(read_data.get("key2"), Some(&"value2".to_string()));
        assert_eq!(read_data.get("key3"), Some(&"value3".to_string()));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_get_from_sstable() {
        let path = "test_sstable_get.sst";
        let _ = fs::remove_file(path);

        let mut data = BTreeMap::new();
        data.insert("user_1".to_string(), "Alice".to_string());
        data.insert("user_2".to_string(), "Bob".to_string());

        SSTable::write(path, &data).unwrap();

        assert_eq!(SSTable::get(path, "user_1").unwrap(), Some("Alice".to_string()));
        assert_eq!(SSTable::get(path, "user_2").unwrap(), Some("Bob".to_string()));
        assert_eq!(SSTable::get(path, "nonexistent").unwrap(), None);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_read_nonexistent_sstable() {
        let result = SSTable::read("nonexistent.sst").unwrap();
        assert_eq!(result.len(), 0);
    }
}