mod power;
mod tree;
use crate::{
    application::monitor::{MonitorFrame, SnapshotRenderer},
    domain::metrics::SystemSnapshot,
};
use crossterm::{
    queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io::{self, Write};

pub struct TerminalRenderer<W> {
    writer: W,
}
impl<W: Write> TerminalRenderer<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}
impl<W: Write> SnapshotRenderer for TerminalRenderer<W> {
    fn render(&mut self, frame: MonitorFrame<'_>) -> io::Result<()> {
        let w = &mut self.writer;
        write!(w, "\x1B[2J\x1B[3J\x1B[H")?;
        colored(w, "╭──────────────────────────────────────────────────────────────╮\n│                     SYSTEM MONITOR                           │\n╰──────────────────────────────────────────────────────────────╯\n", Color::Cyan)?;
        match frame {
            MonitorFrame::Dashboard {
                system,
                power: metrics,
            } => {
                dashboard(w, system)?;
                power::render(w, metrics)?;
            }
            MonitorFrame::Tree(result) => tree::render(w, result)?,
            MonitorFrame::Power(result) => power::render(w, result)?,
        }
        w.flush()
    }
}

fn dashboard(w: &mut impl Write, snapshot: &SystemSnapshot) -> io::Result<()> {
    section(w, "Application")?;
    if let Some(process) = &snapshot.application {
        write!(w, "PID {:<7}  CPU ", process.pid)?;
        colored(
            w,
            format!("{:>6.1}%", process.cpu_usage),
            usage_color(process.cpu_usage as f64),
        )?;
        colored(
            w,
            format!("  Memory {:>8.1} MB\n", process.memory_mb),
            Color::Magenta,
        )?;
    }
    section(w, "Memory")?;
    let memory = &snapshot.memory;
    let percent = if memory.total_gb > 0.0 {
        memory.used_gb / memory.total_gb * 100.0
    } else {
        0.0
    };
    colored(
        w,
        format!("{}  {:>5.1}%", progress_bar(percent, 32), percent),
        usage_color(percent),
    )?;
    writeln!(w, "   {:.1} / {:.1} GB", memory.used_gb, memory.total_gb)?;
    render_cpu(w, snapshot)?;
    section(w, "Top processes by memory")?;
    if snapshot.top_processes.is_empty() {
        return writeln!(w, "No process information available.");
    }
    colored(
        w,
        format!(
            "{:<8} {:<24} {:>8} {:>12}\n",
            "PID", "PROCESS", "CPU", "MEMORY"
        ),
        Color::DarkGrey,
    )?;
    writeln!(w, "──────── ──────────────────────── ──────── ────────────")?;
    for p in &snapshot.top_processes {
        write!(w, "{:<8} {:<24} ", p.pid, truncate(&safe_text(&p.name), 24))?;
        colored(
            w,
            format!("{:>7.1}% ", p.cpu_usage),
            usage_color(p.cpu_usage as f64),
        )?;
        colored(w, format!("{:>9.1} MB\n", p.memory_mb), Color::Magenta)?;
    }
    Ok(())
}

fn render_cpu(w: &mut impl Write, snapshot: &SystemSnapshot) -> io::Result<()> {
    section(w, "CPU")?;
    if snapshot.cpus.is_empty() {
        return writeln!(w, "CPU information is unavailable.");
    }
    let average =
        snapshot.cpus.iter().map(|c| c.usage as f64).sum::<f64>() / snapshot.cpus.len() as f64;
    colored(
        w,
        format!(
            "Total  {}  {:>5.1}%\n\n",
            progress_bar(average, 32),
            average
        ),
        usage_color(average),
    )?;
    for (index, cpu) in snapshot.cpus.iter().enumerate() {
        write!(w, "CPU {index:>2}  ")?;
        colored(
            w,
            format!(
                "{}  {:>5.1}%\n",
                progress_bar(cpu.usage as f64, 24),
                cpu.usage
            ),
            usage_color(cpu.usage as f64),
        )?;
    }
    Ok(())
}

fn colored(w: &mut impl Write, text: impl AsRef<str>, color: Color) -> io::Result<()> {
    queue!(
        w,
        SetForegroundColor(color),
        Print(text.as_ref()),
        ResetColor
    )
}
fn section(w: &mut impl Write, title: &str) -> io::Result<()> {
    writeln!(w)?;
    colored(w, title, Color::Blue)?;
    writeln!(
        w,
        "\n────────────────────────────────────────────────────────────────"
    )
}
fn usage_color(percentage: f64) -> Color {
    if percentage < 50.0 {
        Color::Green
    } else if percentage < 80.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}
fn progress_bar(percentage: f64, width: usize) -> String {
    let filled = ((percentage.clamp(0.0, 100.0) / 100.0) * width as f64).round() as usize;
    format!(
        "[{}{}]",
        "█".repeat(filled),
        "░".repeat(width.saturating_sub(filled))
    )
}
fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.into();
    }
    let mut result: String = value.chars().take(width.saturating_sub(1)).collect();
    result.push('…');
    result
}
fn safe_text(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
                '�'
            } else {
                c
            }
        })
        .collect()
}
