use super::{colored, safe_text, section, usage_color};
use crate::domain::process::{ProcessError, ProcessTree};
use crossterm::style::Color;
use std::io::{self, Write};

pub(super) fn render(
    w: &mut impl Write,
    result: &Result<ProcessTree, ProcessError>,
) -> io::Result<()> {
    section(w, "Process Tree")?;
    let tree = match result {
        Ok(tree) => tree,
        Err(error) => return writeln!(w, "{}", safe_text(&error.to_string())),
    };
    if tree.roots.is_empty() {
        return writeln!(w, "No process information available.");
    }
    colored(
        w,
        format!(
            "{:<40} {:>8} {:>8} {:>12}\n",
            "PROCESS", "PID", "CPU", "RAM"
        ),
        Color::DarkGrey,
    )?;
    // Stack stores only traversal state. Ancestor continuation flags are shared,
    // so a chain does not allocate a new path vector per node.
    let mut stack: Vec<_> = tree.roots.iter().rev().map(|&i| (i, 0, true)).collect();
    let mut continuation = Vec::new();
    while let Some((i, depth, last)) = stack.pop() {
        continuation.truncate(depth);
        let mut label = String::new();
        for &continues in continuation.iter().skip(1) {
            label.push_str(if continues { "│   " } else { "    " });
        }
        if depth > 0 {
            label.push_str(if last { "└── " } else { "├── " });
        }
        continuation.push(!last);
        let node = &tree.nodes[i];
        label.push_str(&safe_text(&node.process.name));
        write!(w, "{label:<40} {:>8} ", node.process.pid.0)?;
        colored(
            w,
            format!("{:>7.1}% ", node.process.cpu_usage),
            usage_color(node.process.cpu_usage as f64),
        )?;
        colored(
            w,
            format!(
                "{:>9.1} MB\n",
                node.process.memory_bytes as f64 / 1_048_576.0
            ),
            Color::Magenta,
        )?;
        for (position, &child) in node.children.iter().enumerate().rev() {
            stack.push((child, depth + 1, position + 1 == node.children.len()));
        }
    }
    Ok(())
}
