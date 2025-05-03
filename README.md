# Rust System Monitor

Lightweight terminal tool for monitoring CPU, memory, and processes.

## Install & Run

1. Install Rust:  
   `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

2. Build and run:
   ```bash
   cargo build --release  # Optimized build
   ./target/release/system_usage_analyzer [SECONDS] [PROCESS_COUNT]
   ```
