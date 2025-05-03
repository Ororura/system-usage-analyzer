use std::{env, thread, time};
use sysinfo::{Pid, System};

fn main() {
    let args: Vec<String> = env::args().collect();
    let refresh_interval = args.get(1).and_then(|f| f.parse().ok()).unwrap_or(500);
    let process_count = args.get(2).and_then(|f| f.parse().ok()).unwrap_or(10);
    let pid = Pid::from(std::process::id() as usize);

    let mut system = System::new_all();
    let delay = time::Duration::from_millis(refresh_interval);

    loop {
        print!("\x1B[2J\x1B[1;1H");

        system.refresh_all();
        thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        system.refresh_cpu_all();

        if let Some(process) = system.process(pid) {
            println!(
                "\nProgramm usage: CPU {:.1}%, RAM {:.1} MB",
                process.cpu_usage(),
                process.memory() as f64 / 1024.0 / 1024.0
            );
        }

        println!("\nCPU Information:");
        for cpu in system.cpus() {
            println!("CPU: {} - {:.1}%", cpu.name(), cpu.cpu_usage());
        }

        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        println!(
            "\nMemory: {:.1} GB/{:.1} GB used",
            used_memory as f64 / 1024.0 / 1024.0,
            total_memory as f64 / 1024.0 / 1024.0
        );

        println!("\nTop Memory-Consuming Processes:");
        let mut processes: Vec<_> = system.processes().iter().collect();
        processes.sort_by_key(|(_, process)| process.memory());
        processes.reverse();

        for (pid, process) in processes.iter().take(process_count) {
            println!(
                "PID: {}, Name: {:?}, CPU: {:.1}%, Memory: {:.1} MB",
                pid,
                process.name(),
                process.cpu_usage(),
                process.memory() as f64 / 1024.0 / 1024.0
            );
        }

        thread::sleep(delay);
    }
}
