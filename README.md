# LSM-Based Storage Engine in Rust

A high-performance, write-optimized key-value storage engine built from scratch in Rust, implementing core database concepts used in production systems like Cassandra, RocksDB, and LevelDB.

## Project Overview

This storage engine demonstrates deep understanding of database internals by implementing:

- **LSM Tree architecture** for write-optimized storage
- **Write-Ahead Logging (WAL)** for crash recovery and durability
- **Sorted String Tables (SSTables)** for persistent storage
- **Multi-level reads** across memory and disk
- **Automatic memory management** with configurable flush thresholds

### Why This Matters

Modern distributed databases (Cassandra, ScyllaDB, RocksDB) use LSM trees because they convert random writes into sequential writes, achieving 10-100x better write throughput than traditional B-tree storage engines.

##  Architecture
```
┌─────────────────────────────────────────────────────────┐
│                     Storage Engine                       │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  Write Path:                                             │
│  1. Log to WAL (durability) ──────────────┐             │
│  2. Write to MemTable (speed)              │             │
│  3. Flush to SSTable when full ────────────┼────> Disk  │
│                                             │             │
│  Read Path:                                 │             │
│  1. Check MemTable first                   │             │
│  2. Check SSTables (newest to oldest) ─────┘             │
│                                                           │
└─────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. **MemTable** (`src/memtable.rs`)
- In-memory write buffer using HashMap
- Fast O(1) reads and writes
- Configurable size threshold (default: 100 entries)
- Automatically flushes to SSTable when full

#### 2. **Write-Ahead Log (WAL)** (`src/wal.rs`)
- Sequential append-only log
- Every operation logged BEFORE applying to memory
- Enables crash recovery by replaying log on restart
- Uses `fsync()` to guarantee durability

#### 3. **SSTable** (`src/sstable.rs`)
- Immutable sorted files on disk
- Binary format: `[count][key_len][key][value_len][value]...`
- Enables handling datasets larger than RAM
- Sequential numbering: `sstable_000000.sst`, `sstable_000001.sst`, etc.

## Features

### Implemented

- [x] In-memory key-value store
- [x] Write-Ahead Logging for durability
- [x] Crash recovery (survives unexpected shutdowns)
- [x] Automatic flush to disk when memory limit reached
- [x] Multi-level reads (memory + disk)
- [x] Comprehensive test suite (11 tests)
- [x] Binary SSTable format

### Future Enhancements

- [ ] Bloom filters (skip unnecessary disk reads)
- [ ] Compaction (merge SSTables, remove duplicates)
- [ ] Range queries
- [ ] Compression (Snappy/LZ4)
- [ ] Block cache for frequently accessed data
- [ ] Multiple compaction strategies (size-tiered, leveled)

## Performance Characteristics

**Write Performance:**
- Sequential writes (WAL) instead of random I/O
- Batching in memory before disk flush
- Expected: 10,000+ writes/second on consumer hardware

**Read Performance:**
- O(1) for data in MemTable
- O(N) for SSTable lookups (N = number of SSTables)
- Can be optimized to O(log N) with bloom filters

**Space:**
- Configurable memory footprint
- Disk usage grows with data (no fixed overhead)
- WAL size bounded by flush threshold

##  Technical Implementation

### Key Design Decisions

**1. Why LSM Trees over B-Trees?**
- B-trees: Optimized for reads, poor write performance (random I/O)
- LSM trees: Optimized for writes (sequential I/O), acceptable read performance
- Use case: Write-heavy workloads (logs, metrics, time-series data)

**2. Why Rust?**
- Memory safety without garbage collection
- Zero-cost abstractions
- Excellent for systems programming
- Same reasons RocksDB chose C++ (performance + control)

**3. File Format Choice**
- Simple length-prefixed binary format
- Easy to parse and extend
- Production systems use similar formats (SSTable, RocksDB's BlockBasedTable)

### Error Handling

All I/O operations return `io::Result<T>`:
```rust
pub fn put(&mut self, key: String, value: String) -> io::Result<()>
```

Ensures callers handle failures explicitly (disk full, permissions, etc.)

##  Getting Started

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- macOS, Linux, or Windows

### Installation
```bash
git clone <your-repo-url>
cd storage-engine
cargo build --release
```

### Usage

**Run the demo:**
```bash
cargo run
```

**Run tests:**
```bash
cargo test
```

**Clear all data:**
```bash
cargo run clear
```

### Example Code
```rust
use memtable::MemTable;

fn main() {
    // Create storage engine with WAL
    let mut db = MemTable::new("data.log").expect("Failed to create storage");
    
    // Write data
    db.put("user_123".to_string(), "Alice".to_string()).unwrap();
    
    // Read data
    if let Some(value) = db.get("user_123") {
        println!("Found: {}", value);
    }
    
    // Data survives crashes - try killing and restarting!
}
```

## Testing

The project includes comprehensive unit tests covering:

- Basic put/get/delete operations
- Crash recovery scenarios
- SSTable flush behavior
- WAL replay correctness
- Edge cases (nonexistent keys, empty stores)

Run with:
```bash
cargo test -- --nocapture  # Shows println! output
cargo test --verbose       # Detailed test info
```

## Benchmarking

To measure performance:
```bash
# Write 100,000 entries
cargo run --release

# Monitor with:
# - Number of SSTables created
# - Time to complete
# - Disk usage
```

Expected results on modern hardware:
- ~10,000-50,000 writes/second
- Read latency: <1ms from MemTable, <10ms from SSTable

## Learning Resources

This project implements concepts from:

- **"Designing Data-Intensive Applications"** by Martin Kleppmann (Chapter 3)
- **"Database Internals"** by Alex Petrov
- Original LSM-Tree paper: O'Neil et al., 1996
- Cassandra and RocksDB documentation

### Related Projects

- [LevelDB](https://github.com/google/leveldb) - Google's LSM implementation (C++)
- [RocksDB](https://github.com/facebook/rocksdb) - Facebook's fork of LevelDB (C++)
- [BadgerDB](https://github.com/dgraph-io/badger) - Go implementation

## Code Structure
```
storage-engine/
├── src/
│   ├── main.rs       # Demo application
│   ├── memtable.rs   # In-memory store + coordination
│   ├── wal.rs        # Write-Ahead Log
│   └── sstable.rs    # Sorted String Tables
├── Cargo.toml        # Rust dependencies
├── README.md         # This file
└── data.log          # WAL file (created at runtime)
```

## Key Insights

### What I Learned

1. **Sequential I/O is king** - Converting random writes to sequential writes provides massive speedups
2. **Immutability simplifies concurrency** - SSTables never change after creation
3. **Trade-offs are everywhere** - Write optimization comes at the cost of read complexity
4. **Durability requires discipline** - Must fsync() before acknowledging writes

### Real-World Applications

This architecture powers:
- **Time-series databases** (InfluxDB, TimescaleDB)
- **Distributed databases** (Cassandra, ScyllaDB, YugabyteDB)
- **Embedded databases** (LevelDB in Chrome, RocksDB in MySQL)
- **Message queues** (Kafka uses similar log-structured storage)

## 🤝 Contributing

This is a learning project, but suggestions are welcome! Areas for contribution:

- Bloom filter implementation
- Compaction strategies
- Performance benchmarks
- Additional test cases

## License

MIT License - feel free to use this for learning or as a foundation for your own projects.

##  Author

**[E-ugine]**
- Built as a deep dive into database internals
- Demonstrates systems programming and Rust expertise
- Contact: [agollaeugine@gmail.com]

---

⭐ **If this project helped you understand storage engines, please star it!**

*Built with ❤️ and lots of coffee ☕*