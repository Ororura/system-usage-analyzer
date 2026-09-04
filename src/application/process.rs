use std::{cmp::Ordering, collections::HashMap};

use crate::domain::process::{
    ProcessError, ProcessId, ProcessInfo, ProcessNode, ProcessRepository, ProcessSort, ProcessTree,
};

pub struct GetProcessTree;

impl GetProcessTree {
    pub fn execute(
        repository: &mut impl ProcessRepository,
        root: Option<ProcessId>,
        sort: ProcessSort,
    ) -> Result<ProcessTree, ProcessError> {
        build_tree(repository.processes()?, root, sort)
    }
}

pub fn build_tree(
    processes: Vec<ProcessInfo>,
    root: Option<ProcessId>,
    sort: ProcessSort,
) -> Result<ProcessTree, ProcessError> {
    let mut index = HashMap::with_capacity(processes.len());
    for (i, process) in processes.iter().enumerate() {
        if index.insert(process.pid, i).is_some() {
            return Err(ProcessError::DuplicatePid(process.pid.0));
        }
    }
    let selected = root
        .map(|pid| {
            index
                .get(&pid)
                .copied()
                .ok_or(ProcessError::NotFound(pid.0))
        })
        .transpose()?;
    let mut parents: Vec<_> = processes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            p.parent_pid
                .and_then(|pid| index.get(&pid).copied())
                .filter(|&p| p != i)
        })
        .collect();
    break_cycles(&mut parents, &processes);
    let mut children = vec![Vec::new(); processes.len()];
    let mut roots = Vec::new();
    for (i, parent) in parents.iter().enumerate() {
        match parent {
            Some(parent) => children[*parent].push(i),
            None => roots.push(i),
        }
    }
    let compare = |a: &usize, b: &usize| compare_processes(&processes[*a], &processes[*b], sort);
    roots.sort_unstable_by(compare);
    for siblings in &mut children {
        siblings.sort_unstable_by(compare);
    }
    if let Some(selected) = selected {
        roots = vec![selected];
    }
    let mut nodes: Vec<_> = processes
        .into_iter()
        .zip(children)
        .map(|(process, children)| ProcessNode {
            process,
            children,
            depth: 0,
        })
        .collect();
    let mut stack: Vec<_> = roots.iter().map(|&i| (i, 0)).collect();
    while let Some((i, depth)) = stack.pop() {
        nodes[i].depth = depth;
        stack.extend(nodes[i].children.iter().map(|&child| (child, depth + 1)));
    }
    Ok(ProcessTree { nodes, roots })
}

fn compare_processes(a: &ProcessInfo, b: &ProcessInfo, sort: ProcessSort) -> Ordering {
    let order = match sort {
        ProcessSort::Pid => Ordering::Equal,
        ProcessSort::Cpu => b.cpu_usage.total_cmp(&a.cpu_usage),
        ProcessSort::Memory => b.memory_bytes.cmp(&a.memory_bytes),
        ProcessSort::Name => a.name.cmp(&b.name),
    };
    order.then_with(|| a.pid.cmp(&b.pid))
}

/// Each vertex enters and leaves the path once, including vertices in cycles.
fn break_cycles(parents: &mut [Option<usize>], processes: &[ProcessInfo]) {
    let mut state = vec![0u8; parents.len()];
    let mut path = Vec::new();
    for start in 0..parents.len() {
        let mut current = Some(start);
        while let Some(i) = current {
            if state[i] != 0 {
                if state[i] == 1 {
                    let mut smallest = i;
                    let mut next = parents[i];
                    while let Some(j) = next {
                        if j == i {
                            break;
                        }
                        if processes[j].pid < processes[smallest].pid {
                            smallest = j;
                        }
                        next = parents[j];
                    }
                    parents[smallest] = None;
                }
                break;
            }
            state[i] = 1;
            path.push(i);
            current = parents[i];
        }
        for i in path.drain(..) {
            state[i] = 2;
        }
    }
}
