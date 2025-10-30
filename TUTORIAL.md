# Storage Engine Tutorial

A step-by-step guide to understanding and extending this storage engine.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Understanding the Codebase](#understanding-the-codebase)
3. [Tracing a Write Operation](#tracing-a-write-operation)
4. [Tracing a Read Operation](#tracing-a-read-operation)
5. [Testing Your Changes](#testing-your-changes)
6. [Extending the Engine](#extending-the-engine)

---

## Getting Started

### Prerequisites

Install Rust if you haven't already:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Clone and Run
```bash
git clone <your-repo>
cd storage-engine
cargo run
```

You should see output showing writes, automatic flushing, and reads.

---

## Understanding the Codebase

### File Structure
```
src/
├── main.rs       # Entry point, demo application
├── memtable.rs   # In-memory store, orchestrates everything
├── wal.rs        # Write-Ahead Log for crash recovery
└── sstable.rs    # Persistent sorted storage
```

### Reading Order

Start here:
1. **`sstable.rs`** (simplest, just file I/O)
2. **`wal.rs`** (simple append-only log)
3. **`memtable.rs`** (coordinates everything)
4. **`main.rs`** (see it all working together)

---

## Tracing a Write Operation

Let's trace what happens when you call:
```rust
db.put("user_123".to_string(), "Alice".to_string())
```

### Step 1: Entry Point (main.rs)
```rust
// In main.rs
let mut memtable = MemTable::new("data.log").unwrap();
memtable.put("user_123".to_string(), "Alice".to_string()).unwrap();
```

This calls `MemTable::put()` in `memtable.rs`.

### Step 2: MemTable Receives Write (memtable.rs:38-50)
```rust
pub fn put(&mut self, key: String, value: String) -> io::Result<()> {
    // Step 2a: Log to WAL FIRST
    self.wal.log_put(&key, &value)?;
    
    // Step 2b: Then write to memory
    self.data.insert(key, value);
    
    // Step 2c: Check if flush needed
    if self.data.len() >= self.max_size {
        self.flush()?;
    }
    
    Ok(())
}
```

**Why this order?**
- WAL first = durability (survives crash)
- Memory second = speed (O(1) operation)
- Flush check = prevent OOM

### Step 3: WAL Logging (wal.rs:21-27)
```rust
pub fn log_put(&mut self, key: &str, value: &str) -> io::Result<()> {
    let entry = format!("PUT,{},{}\n", key, value);
    self.file.write_all(entry.as_bytes())?;  // Write to disk
    self.file.sync_all()?;                    // Force fsync
    Ok(())
}
```

**Key points:**
- `write_all()` - writes bytes to file
- `sync_all()` - **CRITICAL** - flushes OS buffers to disk
- Without `sync_all()`, data might be lost on crash!

### Step 4: Memory Update (back in memtable.rs)
```rust
self.data.insert(key, value);
```

This is a standard Rust HashMap insert - O(1) operation.

### Step 5: Flush Check
```rust
if self.data.len() >= self.max_size {
    self.flush()?;
}
```

If we've reached 100 entries (configurable), flush to SSTable.

### Step 6: Flush Operation (memtable.rs:66-88)
```rust
fn flush(&mut self) -> io::Result<()> {
    // Convert HashMap → BTreeMap (sorted)
    let sorted_data: BTreeMap<String, String> = 
        self.data.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

    // Generate filename
    let sstable_path = format!("sstable_{:06}.sst", self.sstable_counter);
    self.sstable_counter += 1;

    // Write to SSTable
    SSTable::write(&sstable_path, &sorted_data)?;

    // Clear memtable
    self.data.clear();

    // Truncate WAL
    fs::remove_file(&self.wal_path)?;
    self.wal = WriteAheadLog::new(&self.wal_path)?;

    Ok(())
}
```

**What's happening:**
1. Sort data (HashMap is unordered, SSTables must be sorted)
2. Generate unique filename (`sstable_000000.sst`)
3. Write to disk in binary format
4. Clear memory (make room for new writes)
5. Reset WAL (old data is now in SSTable)

### Step 7: SSTable Write (sstable.rs:11-33)
```rust
pub fn write(path: &str, data: &BTreeMap<String, String>) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;

    // Write count
    let num_entries = data.len() as u32;
    file.write_all(&num_entries.to_le_bytes())?;

    // Write each entry
    for (key, value) in data.iter() {
        // Write key length and bytes
        file.write_all(&(key.len() as u32).to_le_bytes())?;
        file.write_all(key.as_bytes())?;

        // Write value length and bytes
        file.write_all(&(value.len() as u32).to_le_bytes())?;
        file.write_all(value.as_bytes())?;
    }

    file.sync_all()?;
    Ok(())
}
```

**Binary format:**
```
[4 bytes: count][4 bytes: key1_len][key1 bytes][4 bytes: val1_len][val1 bytes]...
```

---

## Tracing a Read Operation

Let's trace:
```rust
let value = db.get("user_123");
```

### Step 1: Entry Point (memtable.rs:63-78)
```rust
pub fn get(&self, key: &str) -> Option<String> {
    // Check MemTable first (fast path)
    if let Some(value) = self.data.get(key) {
        return Some(value.clone());
    }
    
    // Check SSTables (newest to oldest)
    for i in (0..self.sstable_counter).rev() {
        let sstable_path = format!("sstable_{:06}.sst", i);
        if let Ok(Some(value)) = SSTable::get(&sstable_path, key) {
            return Some(value);
        }
    }
    
    None
}
```

**Read path:**
1. MemTable (O(1)) - if found, return immediately
2. SSTable N (newest)
3. SSTable N-1
4. SSTable N-2
5. ...
6. SSTable 0 (oldest)
7. Return None if not found anywhere

### Step 2: SSTable Read (sstable.rs:82-86)
```rust
pub fn get(path: &str, key: &str) -> io::Result<Option<String>> {
    let data = Self::read(path)?;  // Load entire file
    Ok(data.get(key).cloned())      // Find key
}
```

Currently loads entire SSTable into memory. Future optimization: use bloom filters to skip files that definitely don't have the key.

---

## Testing Your Changes

### Running All Tests
```bash
cargo test
```

### Running Specific Test
```bash
cargo test test_flush_to_sstable
```

### Running with Output
```bash
cargo test -- --nocapture
```

### Adding a New Test

In `memtable.rs`, add to the `tests` module:
```rust
#[test]
fn test_my_new_feature() {
    let wal_path = "test_my_feature.log";
    let _ = fs::remove_file(wal_path);
    
    let mut memtable = MemTable::new(wal_path).unwrap();
    
    // Your test code here
    
    // Cleanup
    fs::remove_file(wal_path).unwrap();
}
```

---

## Extending the Engine

### Exercise 1: Add a Size Counter

**Goal:** Track total bytes stored, not just number of entries.

**Steps:**

1. Add field to `MemTable`:
```rust
pub struct MemTable {
    data: HashMap<String, String>,
    wal: WriteAheadLog,
    wal_path: String,
    max_size: usize,
    sstable_counter: usize,
    total_bytes: usize,  // NEW
}
```

2. Update `put()` to track size:
```rust
pub fn put(&mut self, key: String, value: String) -> io::Result<()> {
    let bytes = key.len() + value.len();
    self.total_bytes += bytes;  // NEW
    
    self.wal.log_put(&key, &value)?;
    self.data.insert(key, value);
    
    // Maybe flush based on bytes instead of count?
    if self.total_bytes >= 1024 * 1024 {  // 1MB
        self.flush()?;
    }
    
    Ok(())
}
```

3. Reset in `flush()`:
```rust
fn flush(&mut self) -> io::Result<()> {
    // ... existing code ...
    
    self.total_bytes = 0;  // NEW
    
    Ok(())
}
```

### Exercise 2: Add Get Statistics

**Goal:** Count cache hits (MemTable) vs disk reads (SSTable).

**Steps:**

1. Add stats struct:
```rust
pub struct Stats {
    pub memtable_hits: usize,
    pub sstable_reads: usize,
}

pub struct MemTable {
    // ... existing fields ...
    pub stats: Stats,  // NEW
}
```

2. Track in `get()`:
```rust
pub fn get(&self, key: &str) -> Option<String> {
    if let Some(value) = self.data.get(key) {
        self.stats.memtable_hits += 1;  // Needs RefCell to mutate
        return Some(value.clone());
    }
    
    for i in (0..self.sstable_counter).rev() {
        self.stats.sstable_reads += 1;  // Needs RefCell
        // ... rest of code ...
    }
    
    None
}
```

**Note:** You'll need to use `RefCell` for interior mutability since `get()` takes `&self`.

### Exercise 3: Implement Delete Properly

**Goal:** Current `delete()` logs but doesn't persist across flushes.

**Challenge:**
- When you flush, deleted keys disappear from MemTable
- But they might still exist in old SSTables!
- Need to write "tombstones" to SSTables

**Solution approach:**
1. Change MemTable value type: `HashMap<String, Option<String>>`
   - `Some(value)` = present
   - `None` = deleted (tombstone)
2. Update SSTable format to support tombstones
3. During reads, check for tombstones and return None

### Exercise 4: Add Bloom Filters

**Goal:** Skip SSTables that definitely don't contain a key.

**High-level approach:**

1. Add bloom filter library:
```toml
# In Cargo.toml
[dependencies]
bloomfilter = "1.0"
```

2. Create bloom filter during SSTable write:
```rust
use bloomfilter::Bloom;

pub struct SSTable {
    pub path: String,
    pub bloom: Bloom<String>,  // NEW
}
```

3. Check bloom filter before reading file:
```rust
pub fn get(&self, key: &str) -> Option<String> {
    // Check bloom filter first
    if !self.bloom.check(key) {
        return None;  // Definitely not here!
    }
    
    // Maybe here, need to check file
    // ... existing read logic ...
}
```

**Expected improvement:** 10-100x faster negative lookups!

### Exercise 5: Add Compaction

**Goal:** Merge multiple small SSTables into one large SSTable.

**Why needed:**
- Too many SSTables = slow reads
- Deleted keys still take space (tombstones)
- Duplicate keys across files (updates)

**Algorithm:**
```rust
pub fn compact(&mut self, files: Vec<String>) -> io::Result<()> {
    // 1. Read all SSTables
    let mut merged = BTreeMap::new();
    for file in files.iter().rev() {  // Newest first
        let data = SSTable::read(file)?;
        // Newer values overwrite older ones
        for (k, v) in data {
            merged.entry(k).or_insert(v);
        }
    }
    
    // 2. Write to new SSTable
    let new_path = format!("sstable_{:06}.sst", self.sstable_counter);
    SSTable::write(&new_path, &merged)?;
    
    // 3. Delete old files
    for file in files {
        fs::remove_file(file)?;
    }
    
    Ok(())
}
```

---

## Debugging Tips

### Enable Logging

Add to `Cargo.toml`:
```toml
[dependencies]
env_logger = "0.10"
log = "0.4"
```

In your code:
```rust
use log::{info, debug};

info!("Flushing {} entries", self.data.len());
debug!("Reading SSTable: {}", path);
```

Run with:
```bash
RUST_LOG=debug cargo run
```

### Inspect Binary Files
```bash
# View SSTable as hex
hexdump -C sstable_000000.sst | head -20

# View WAL (text format)
cat data.log
```

### Common Issues

**Problem:** Tests fail with "Permission denied"
**Solution:** Some test file wasn't cleaned up. Delete manually:
```bash
rm test_*.log test_*.sst
cargo test
```

**Problem:** "Too many open files"
**Solution:** Close files properly. Check for missing `drop()` or file handles.

**Problem:** Data corruption after crash
**Solution:** Ensure `sync_all()` is called after every WAL write.

---

## Performance Analysis

### Measuring Write Throughput
```rust
use std::time::Instant;

let start = Instant::now();
for i in 0..100_000 {
    db.put(format!("key_{}", i), format!("value_{}", i)).unwrap();
}
let duration = start.elapsed();
println!("Writes/sec: {}", 100_000.0 / duration.as_secs_f64());
```

### Profiling with `cargo flamegraph`
```bash
cargo install flamegraph
cargo flamegraph
```

Opens a flame graph showing where time is spent.

### Expected Bottlenecks

1. **fsync()** - Most expensive operation (10-100ms)
2. **SSTable reads** - Many small files = many syscalls
3. **Sorting on flush** - O(N log N), but N is small

---

## Next Steps

After mastering this codebase:

1. **Read Production Code:**
   - [LevelDB](https://github.com/google/leveldb) (C++, well-commented)
   - [BadgerDB](https://github.com/dgraph-io/badger) (Go, readable)

2. **Research Papers:**
   - "The Log-Structured Merge-Tree" (O'Neil et al.)
   - "Bigtable: A Distributed Storage System" (Google)

3. **Advanced Topics:**
   - Compaction strategies (size-tiered vs leveled)
   - Write amplification optimization
   - Read amplification with bloom filters
   - Distributed consensus (Raft/Paxos)

---

## Getting Help

**Stuck?** Check:
1. Rust error messages (they're excellent!)
2. `cargo doc --open` (browse local docs)
3. This storage engine's test cases
4. LevelDB/RocksDB documentation

**Found a bug?** Open an issue with:
- What you expected
- What actually happened
- Minimal code to reproduce

---

*Happy hacking! 🦀*