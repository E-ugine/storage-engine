mod memtable;
mod wal;
mod sstable;

use memtable::MemTable;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "clear" {
        let _ = std::fs::remove_file("data.log");
        // Remove all SSTable files
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with("sstable_") {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }
        println!(" All data cleared!");
        return;
    }
    
    
    let mut memtable = MemTable::new("data.log").expect("Failed to create MemTable");
    
    println!("Writing 150 entries (flush threshold = 100)...\n");
    
    for i in 0..150 {
        memtable.put(
            format!("user_{:03}", i), 
            format!("User Number {}", i)
        ).expect("Failed to put");
        
        // Show progress every 25 entries
        if (i + 1) % 25 == 0 {
            println!("   Written {} entries, MemTable size: {}", i + 1, memtable.size());
        }
    }
    
    println!("\n All 150 entries written!");
    println!("   Final MemTable size: {}", memtable.size());
    println!("   (Should be ~50 after first flush at 100)\n");
    
    // Test reading some values
    println!(" Reading some values:");
    println!("   user_000: {:?}", memtable.get("user_000"));
    println!("   user_050: {:?}", memtable.get("user_050"));
    println!("   user_100: {:?}", memtable.get("user_100"));
    println!("   user_149: {:?}", memtable.get("user_149"));
    
    println!("\n Note: user_000 to user_099 are in sstable_000000.sst");
    println!("   user_100 to user_149 are still in MemTable");
    
    println!("\n To clear all data: cargo run clear");
}