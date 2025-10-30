# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-10-30

### Added
- Initial release of LSM-based storage engine
- In-memory key-value store (MemTable) with HashMap
- Write-Ahead Log (WAL) for crash recovery and durability
- Sorted String Tables (SSTables) for persistent storage
- Automatic flush when MemTable reaches size threshold (100 entries)
- Multi-level reads (MemTable → SSTables)
- Binary SSTable file format with length-prefixed entries
- Comprehensive test suite (11 tests covering all components)
- Demo application showing automatic flushing
- Full documentation (README, ARCHITECTURE, TUTORIAL, QUICKSTART)

### Technical Details
- **Language:** Rust 2021 edition
- **Architecture:** Log-Structured Merge Tree (LSM)
- **Durability:** fsync() after every WAL write
- **Storage Format:** Binary format for SSTables, CSV-like for WAL
- **Performance:** Sequential I/O for writes, O(1) MemTable lookups

### Known Limitations
- No bloom filters (reads check all SSTables sequentially)
- No compaction (SSTables accumulate over time)
- Single-threaded (no concurrent access)
- Full SSTable loaded into memory on read
- Delete operations not fully implemented (no tombstones in SSTables)

## [Unreleased]

### Planned Features
- [ ] Bloom filters for faster negative lookups
- [ ] SSTable compaction (merge multiple files)
- [ ] Proper delete with tombstones
- [ ] Range query support
- [ ] Compression (Snappy/LZ4)
- [ ] Block-based SSTable reading (memory-efficient)
- [ ] Performance benchmarks and metrics
- [ ] Multi-threading support
- [ ] Configurable flush threshold via API

---

## Version History

### Semantic Versioning Guide

- **MAJOR** version: Incompatible API changes
- **MINOR** version: New functionality (backward compatible)
- **PATCH** version: Bug fixes (backward compatible)

### Release Notes Template
```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- New features

### Changed
- Changes to existing functionality

### Deprecated
- Soon-to-be removed features

### Removed
- Removed features

### Fixed
- Bug fixes

### Security
- Security vulnerability fixes
```

---

[0.1.0]: https://github.com/yourusername/storage-engine/releases/tag/v0.1.0