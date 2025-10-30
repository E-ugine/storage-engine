# Architecture Deep Dive

This document explains the internal design of the storage engine and how components interact.

## System Overview
```
                    Client Application
                           |
                           v
                   ┌───────────────┐
                   │   MemTable    │
                   │  (In Memory)  │
                   └───────┬───────┘
                           |
              ┌────────────┼────────────┐
              |            |            |
              v            v            v
         ┌────────┐   ┌────────┐   ┌──────────┐
         │  WAL   │   │SSTable │   │ SSTable  │
         │  Log   │   │   #0   │   │    #1    │
         └────────┘   └────────┘   └──────────┘
            Disk         Disk          Disk
```

## Write Path

### Step-by-Step Write Operation
```
User calls: db.put("key", "value")
     |
     v
┌─────────────────────────────────────┐
│ 1. Write to WAL                     │
│    - Append "PUT,key,value\n"       │
│    - Call fsync() for durability    │
└──────────────┬──────────────────────┘
               |
               v
┌─────────────────────────────────────┐
│ 2. Write to MemTable                │
│    - HashMap.insert(key, value)     │
│    - O(1) operation, in memory      │
└──────────────┬──────────────────────┘
               |
               v
┌─────────────────────────────────────┐
│ 3. Check Size                       │
│    - if size >= 100 entries         │
│    - trigger flush()                │
└──────────────┬──────────────────────┘
               |
               v (only if threshold reached)
┌─────────────────────────────────────┐
│ 4. Flush to SSTable                 │
│    - Sort data (HashMap → BTreeMap) │
│    - Write binary format to disk    │
│    - Clear MemTable                 │
│    - Truncate WAL                   │
└─────────────────────────────────────┘
```

### Why This Order?

1. **WAL first** - If we crash after writing to MemTable but before WAL, data is lost
2. **MemTable second** - Fast in-memory update
3. **Flush last** - Background operation, doesn't block writes

## Read Path

### Step-by-Step Read Operation
```
User calls: db.get("key")
     |
     v
┌─────────────────────────────────────┐
│ 1. Check MemTable                   │
│    - HashMap.get(key)               │
│    - O(1) lookup                    │
│    - If found: return immediately   │
└──────────────┬──────────────────────┘
               |
               v (if not found)
┌─────────────────────────────────────┐
│ 2. Check SSTables (newest → oldest) │
│    - Read sstable_000001.sst        │
│    - Read sstable_000000.sst        │
│    - Parse binary format            │
│    - If found: return value         │
└──────────────┬──────────────────────┘
               |
               v (if not found anywhere)
┌─────────────────────────────────────┐
│ 3. Return None                      │
│    - Key doesn't exist              │
└─────────────────────────────────────┘
```

### Read Optimization (Future)

Currently reads are O(N) where N = number of SSTables. Can be optimized:

1. **Bloom Filters** - Check "definitely not here" before reading file
2. **Block Index** - Jump to relevant section of SSTable
3. **Cache** - Keep hot data in memory

## Crash Recovery

### What Happens on Crash
```
┌──────────────────────────────────────────┐
│  Normal Operation                        │
│  - MemTable has 50 entries               │
│  - WAL has 50 entries                    │
│  - 2 SSTables on disk                    │
└──────────────┬───────────────────────────┘
               |
          💥 CRASH!
               |
               v
┌──────────────────────────────────────────┐
│  On Restart                              │
│  1. Create new empty MemTable            │
│  2. Open WAL file                        │
│  3. Replay all WAL entries:              │
│     - PUT operations → insert to MemTable│
│     - DELETE operations → remove from MT │
│  4. MemTable now has all 50 entries!     │
│  5. Ready to accept new operations       │
└──────────────────────────────────────────┘
```

### WAL Format

Simple CSV-like format for human readability:
```
PUT,user_1,Alice
PUT,user_2,Bob
DELETE,user_1
PUT,user_3,Charlie
```

Each line is:
- Operation type (PUT or DELETE)
- Key
- Value (only for PUT)

## SSTable File Format

Binary format for efficiency:
```
Bytes 0-3:   Number of entries (u32, little-endian)
Bytes 4-7:   Key 1 length (u32)
Bytes 8-...: Key 1 bytes (UTF-8)
Bytes X-X+3: Value 1 length (u32)
Bytes X+4-.: Value 1 bytes (UTF-8)
... (repeat for each entry)
```

### Example SSTable
```
Data: {"alice" => "Alice Smith", "bob" => "Bob Jones"}

Binary representation:
[2, 0, 0, 0]              # 2 entries
[5, 0, 0, 0]              # key length = 5
[a, l, i, c, e]           # key bytes
[11, 0, 0, 0]             # value length = 11
[A, l, i, c, e,  , S, m, i, t, h]  # value bytes
[3, 0, 0, 0]              # key length = 3
[b, o, b]                 # key bytes
[9, 0, 0, 0]              # value length = 9
[B, o, b,  , J, o, n, e, s]  # value bytes
```

## Memory Management

### MemTable Size Threshold

Currently set to 100 entries. Trade-offs:

**Small threshold (e.g., 10 entries):**
- ✅ Low memory usage
- ✅ More frequent flushes (more durable)
- ❌ Many small SSTables (slower reads)
- ❌ More disk I/O overhead

**Large threshold (e.g., 10,000 entries):**
- ✅ Fewer SSTables (faster reads)
- ✅ Better write batching
- ❌ Higher memory usage
- ❌ More data lost on crash (between flushes)

Production systems typically use 64MB-256MB thresholds.

## Concurrency Model

### Current Implementation (Single-threaded)
```
One thread → MemTable → WAL → SSTables
```

Safe because:
- No shared mutable state
- Sequential operations
- Rust's ownership prevents data races

### Future Multi-threaded Design
```
Write Thread → WAL → MemTable ──┐
                                 ├→ Flush Thread → SSTables
Read Threads → MemTable + SSTables
                                 └→ Compaction Thread
```

Would require:
- Read-write locks on MemTable
- Immutable SSTables (already done!)
- Atomic reference counting

## Comparison to Production Systems

### RocksDB Architecture

Similar concepts:
- ✅ MemTable (they call it MemTable too)
- ✅ WAL (they call it WAL too)
- ✅ SSTables (they call them SST files)
- ➕ Block cache (hot data in memory)
- ➕ Bloom filters (skip unnecessary reads)
- ➕ Compaction (merge SSTables)
- ➕ Column families (logical separation)

### Cassandra Architecture

Similar concepts:
- ✅ MemTable
- ✅ CommitLog (their WAL)
- ✅ SSTables
- ➕ Distributed across multiple nodes
- ➕ Consistent hashing for partitioning
- ➕ Read repair
- ➕ Hinted handoff

## Performance Characteristics

### Time Complexity

| Operation | MemTable | SSTable | Total |
|-----------|----------|---------|-------|
| Write     | O(1)     | O(1)*   | O(1)  |
| Read      | O(1)     | O(N)    | O(N)  |
| Delete    | O(1)     | O(1)*   | O(1)  |

*Amortized - flush happens periodically

N = number of SSTables

### Space Complexity

- MemTable: O(M) where M = number of entries before flush
- WAL: O(M) - same as MemTable
- SSTables: O(D) where D = total dataset size
- Total: O(M + D)

### I/O Patterns

**Sequential I/O (fast):**
- WAL writes
- SSTable writes (flush)
- SSTable reads (full scan)

**Random I/O (slow):**
- Currently: SSTable lookups
- Future optimization: Bloom filters eliminate most random reads

## Design Decisions & Trade-offs

### 1. HashMap vs BTreeMap for MemTable

**Chose HashMap:**
- ✅ O(1) lookups
- ✅ O(1) inserts
- ❌ Not sorted (must sort on flush)

**Alternative (BTreeMap):**
- ❌ O(log N) lookups
- ❌ O(log N) inserts
- ✅ Always sorted (no sort on flush)

For write-heavy workload, HashMap is better.

### 2. Simple WAL Format vs Binary

**Chose CSV-like text:**
- ✅ Human readable (easy debugging)
- ✅ Simple to parse
- ❌ Larger file size
- ❌ Slower parsing

Production systems use binary WAL for performance.

### 3. Load All SSTable vs Streaming

**Current: Load entire SSTable to memory**
- ✅ Simple implementation
- ❌ Memory spike on read
- ❌ Doesn't scale to large SSTables

**Future: Stream/Memory-map SSTables**
- ✅ Constant memory usage
- ✅ OS handles caching
- ❌ More complex code

## Testing Strategy

### Test Coverage

1. **Unit Tests** - Each component in isolation
   - MemTable operations
   - WAL replay
   - SSTable read/write

2. **Integration Tests** - Components working together
   - Crash recovery (WAL → MemTable)
   - Flush (MemTable → SSTable)
   - Multi-level reads

3. **Manual Testing** - Real-world scenarios
   - Kill process during write
   - Fill disk
   - Corrupt files

### Future Test Ideas

- **Property-based testing** (randomized operations)
- **Fault injection** (simulate I/O errors)
- **Performance benchmarks** (throughput, latency)
- **Stress tests** (millions of entries)

## Future Enhancements

### Short Term

1. **Bloom Filters** - 10-100x faster negative lookups
2. **Compaction** - Merge SSTables, remove tombstones
3. **Metrics** - Track ops/sec, latency, cache hits

### Medium Term

4. **Compression** - Snappy/LZ4 for SSTables
5. **Block-based SSTables** - Don't load entire file
6. **Range queries** - Iterate over key ranges

### Long Term

7. **Multi-threading** - Parallel reads, background compaction
8. **Replication** - Multiple nodes for durability
9. **Transactions** - ACID guarantees across multiple keys

---

*This architecture document is a living document and will evolve as the storage engine grows.*