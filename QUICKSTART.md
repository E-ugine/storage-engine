# Quick Start Guide

Get the storage engine running in 5 minutes.

## Installation
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone and build
git clone <your-repo-url>
cd storage-engine
cargo build --release
```

## Basic Usage

### 1. Run the Demo
```bash
cargo run
```

Output:
```
🚀 Storage Engine Demo - Automatic Flush
=========================================

📝 Writing 150 entries (flush threshold = 100)...
   Written 25 entries, MemTable size: 25
   Written 50 entries, MemTable size: 50
   Written 75 entries, MemTable size: 75
✅ Flushed 100 entries to sstable_000000.sst
   Written 100 entries, MemTable size: 0
...
```

### 2. Test Crash Recovery
```bash
# Run once to write data
cargo run

# Run again - data recovered from WAL!
cargo run
```

### 3. Clear All Data
```bash
cargo run clear
```

## Code Example
```rust
use memtable::MemTable;

fn main() {
    // Create storage engine
    let mut db = MemTable::new("data.log").expect("Failed to open");
    
    // Write data
    db.put("user_1".to_string(), "Alice".to_string()).unwrap();
    db.put("user_2".to_string(), "Bob".to_string()).unwrap();
    
    // Read data
    println!("{:?}", db.get("user_1")); // Some("Alice")
    println!("{:?}", db.get("user_2")); // Some("Bob")
    
    // Delete data
    db.delete("user_1").unwrap();
    println!("{:?}", db.get("user_1")); // None
}
```

## Running Tests
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_crash_recovery

# Run with output
cargo test -- --nocapture
```

## What's Happening Under the Hood?

### On Write:
1. **Log to WAL** (`data.log`) - ensures durability
2. **Write to MemTable** (in-memory HashMap) - fast O(1)
3. **Auto-flush to SSTable** when MemTable reaches 100 entries

### On Read:
1. **Check MemTable first** - O(1) lookup
2. **Check SSTables** (newest to oldest) if not found
3. **Return None** if key doesn't exist

### On Crash:
1. **Replay WAL** on restart
2. **Rebuild MemTable** from log entries
3. **Continue operations** - no data lost!

## File Structure
```
storage-engine/
├── data.log              # Write-Ahead Log (created on first run)
├── sstable_000000.sst    # Sorted String Table (created after 100 writes)
├── sstable_000001.sst    # Additional SSTables as data grows
└── src/
    ├── main.rs           # Demo application
    ├── memtable.rs       # Core storage engine
    ├── wal.rs            # Write-Ahead Log
    └── sstable.rs        # Persistent storage
```

## Configuration

Edit `src/memtable.rs` line 15:
```rust
max_size: 100,  // Change this value
```

- **Smaller** (e.g., 10) = more frequent flushes, many small files
- **Larger** (e.g., 1000) = less frequent flushes, fewer larger files

## Performance Tips

### For Write-Heavy Workloads:
- Increase `max_size` to reduce flush frequency
- Use SSD for better fsync performance

### For Read-Heavy Workloads:
- Keep data in MemTable (don't trigger flushes)
- Future: Add bloom filters to skip SSTables

## Benchmarking

Simple benchmark:
```rust
use std::time::Instant;

let mut db = MemTable::new("bench.log").unwrap();
let start = Instant::now();

for i in 0..10_000 {
    db.put(format!("key_{}", i), format!("value_{}", i)).unwrap();
}

let duration = start.elapsed();
println!("Writes/sec: {:.0}", 10_000.0 / duration.as_secs_f64());
```

Expected on modern hardware: **10,000-50,000 writes/second**

## Common Operations

### Clear specific data:
```bash
rm data.log sstable_*.sst
```

### Check data files:
```bash
ls -lh *.log *.sst
```

### View WAL contents:
```bash
cat data.log
```

### View SSTable (binary):
```bash
hexdump -C sstable_000000.sst | head -20
```

## Troubleshooting

### Error: "Failed to create MemTable"
**Cause:** No write permission in current directory  
**Fix:** Run from a directory you own, or check permissions

### Error: "Too many open files"
**Cause:** File handles not closed properly  
**Fix:** Restart terminal, check ulimit: `ulimit -n`

### Tests failing with file errors:
**Cause:** Test files not cleaned up  
**Fix:** `rm test_*.log test_*.sst && cargo test`

### Data not persisting after restart:
**Cause:** WAL not fsyncing properly  
**Fix:** Check that `sync_all()` is called in `wal.rs`

## Next Steps

- Read [ARCHITECTURE.md](ARCHITECTURE.md) for design details
- Follow [TUTORIAL.md](TUTORIAL.md) to extend the engine
- Read [README.md](README.md) for full documentation

## Questions?

- Check existing tests in `src/*/tests` modules
- Read the well-commented source code
- Compare with LevelDB/RocksDB documentation

---

**Now you're ready to explore, modify, and extend the storage engine!** 🚀