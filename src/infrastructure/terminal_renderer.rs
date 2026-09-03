use std::io::{self, Write};

use crate::{application::monitor::SnapshotRenderer, domain::metrics::SystemSnapshot};

use crossterm::{
    cursor::MoveTo,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

pub struct TerminalRenderer;

impl SnapshotRenderer for TerminalRenderer {
    fn render(&self, snapshot: &SystemSnapshot) {
        clear_terminal();

        render_header();
        render_application(snapshot);
        render_memory(snapshot);
        render_cpu(snapshot);
        render_processes(snapshot);

        let _ = io::stdout().flush();
    }
}

fn usage_color(percentage: f64) -> Color {
    match percentage {
        p if p < 50.0 => Color::Green,
        p if p < 80.0 => Color::Yellow,
        _ => Color::Red,
    }
}

fn print_colored(text: impl AsRef<str>, color: Color) {
    let mut stdout = io::stdout();

    execute!(
        stdout,
        SetForegroundColor(color),
        Print(text.as_ref()),
        ResetColor
    )
    .expect("Failed to write colored output");
}

fn clear_terminal() {
    print!("\x1B[2J\x1B[3J\x1B[H");

    io::stdout().flush().expect("Failed to flush stdout");
}

fn render_section_title(title: &str) {
    println!();

    print_colored(title, Color::Blue);

    println!();
    println!("────────────────────────────────────────────────────────────────");
}

fn render_header() {
    print_colored(
        "╭──────────────────────────────────────────────────────────────╮\n",
        Color::Cyan,
    );

    print_colored(
        "│                     SYSTEM MONITOR                           │\n",
        Color::Cyan,
    );

    print_colored(
        "╰──────────────────────────────────────────────────────────────╯\n",
        Color::Cyan,
    );
}

fn render_application(snapshot: &SystemSnapshot) {
    render_section_title("Application");

    if let Some(process) = &snapshot.application {
        print!("PID {:<7}  CPU ", process.pid);

        print_colored(
            format!("{:>6.1}%", process.cpu_usage),
            usage_color(process.cpu_usage as f64),
        );

        print!("  Memory ");

        print_colored(format!("{:>8.1} MB", process.memory_mb), Color::Magenta);

        println!();
    }
}

fn render_memory(snapshot: &SystemSnapshot) {
    render_section_title("Memory");

    let used = snapshot.memory.used_gb;
    let total = snapshot.memory.total_gb;
    let percentage = percentage(used, total);

    let color = usage_color(percentage);

    print_colored(progress_bar(percentage, 32), color);

    print!("  ");

    print_colored(format!("{:>5.1}%", percentage), color);

    println!("   {:.1} / {:.1} GB", used, total);
}

fn render_cpu(snapshot: &SystemSnapshot) {
    render_section_title("CPU");

    if snapshot.cpus.is_empty() {
        println!("CPU information is unavailable.");
        return;
    }

    let average =
        snapshot.cpus.iter().map(|cpu| cpu.usage).sum::<f32>() / snapshot.cpus.len() as f32;

    let average_color = usage_color(average as f64);

    print!("Total  ");
    print_colored(progress_bar(average as f64, 32), average_color);

    print!("  ");

    print_colored(format!("{:>5.1}%", average), average_color);

    println!();
    println!();

    for (index, cpu) in snapshot.cpus.iter().enumerate() {
        let color = usage_color(cpu.usage as f64);

        print!("CPU {:>2}  ", index);

        print_colored(progress_bar(cpu.usage as f64, 24), color);

        print!("  ");

        print_colored(format!("{:>5.1}%", cpu.usage), color);

        println!();
    }
}

fn render_processes(snapshot: &SystemSnapshot) {
    render_section_title("Top processes by memory");

    if snapshot.top_processes.is_empty() {
        println!("No process information available.");
        return;
    }

    print_colored(
        format!(
            "{:<8} {:<24} {:>8} {:>12}\n",
            "PID", "PROCESS", "CPU", "MEMORY"
        ),
        Color::DarkGrey,
    );

    println!(
        "{:<8} {:<24} {:>8} {:>12}",
        "────────", "────────────────────────", "────────", "────────────"
    );

    for process in &snapshot.top_processes {
        print!("{:<8} {:<24} ", process.pid, truncate(&process.name, 24));

        print_colored(
            format!("{:>7.1}%", process.cpu_usage),
            usage_color(process.cpu_usage as f64),
        );

        print!(" ");

        print_colored(format!("{:>9.1} MB", process.memory_mb), Color::Magenta);

        println!();
    }
}

fn progress_bar(percentage: f64, width: usize) -> String {
    let percentage = percentage.clamp(0.0, 100.0);

    let filled = ((percentage / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    return format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));
}

fn percentage(value: f64, total: f64) -> f64 {
    if total <= 0.0 {
        return 0.0;
    }

    value / total * 100.0
}

fn truncate(value: &str, max_width: usize) -> String {
    let count = value.chars().count();

    if count <= max_width {
        return value.to_owned();
    }

    if max_width <= 1 {
        return "…".to_owned();
    }

    let mut result: String = value.chars().take(max_width - 1).collect();

    result.push('…');
    result
}
