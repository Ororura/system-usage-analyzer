use system_usage_analyzer::{
    application::process::{build_tree, GetProcessTree},
    domain::process::*,
};

fn process(pid: u32, parent: Option<u32>) -> ProcessInfo {
    ProcessInfo {
        pid: ProcessId(pid),
        parent_pid: parent.map(ProcessId),
        name: format!("process-{pid}"),
        executable: None,
        command: None,
        cpu_usage: pid as f32,
        memory_bytes: u64::from(pid),
        status: ProcessStatus::Running,
    }
}
fn pids(tree: &ProcessTree, indices: &[usize]) -> Vec<u32> {
    indices
        .iter()
        .map(|&i| tree.nodes[i].process.pid.0)
        .collect()
}
fn tree(processes: Vec<ProcessInfo>) -> ProcessTree {
    build_tree(processes, None, ProcessSort::Pid).unwrap()
}

#[test]
fn nesting_and_depth() {
    let t = tree(vec![
        process(4, Some(2)),
        process(3, Some(1)),
        process(2, Some(1)),
        process(1, None),
    ]);
    assert_eq!(pids(&t, &t.roots), [1]);
    let root = &t.nodes[t.roots[0]];
    assert_eq!(pids(&t, &root.children), [2, 3]);
    assert_eq!(root.children.len(), 2);
    let child = &t.nodes[root.children[0]];
    assert_eq!(child.depth, 1);
    assert_eq!(pids(&t, &child.children), [4]);
    assert_eq!(t.nodes[child.children[0]].depth, 2);
}
#[test]
fn empty_single_orphans_multiple_roots_and_self_parent() {
    assert!(tree(vec![]).roots.is_empty());
    assert_eq!(tree(vec![process(1, None)]).roots.len(), 1);
    let t = tree(vec![
        process(4, Some(4)),
        process(3, Some(999)),
        process(2, Some(0)),
        process(1, None),
    ]);
    assert_eq!(pids(&t, &t.roots), [1, 2, 3, 4]);
    assert_eq!(t.nodes[0].process.parent_pid, Some(ProcessId(4)));
}
#[test]
fn cycles_break_at_smallest_pid_without_losing_descendants() {
    let t = tree(vec![
        process(8, Some(9)),
        process(9, Some(7)),
        process(7, Some(8)),
        process(10, Some(9)),
        process(2, Some(3)),
        process(3, Some(2)),
    ]);
    assert_eq!(pids(&t, &t.roots), [2, 7]);
    assert_eq!(t.nodes[2].process.parent_pid, Some(ProcessId(8)));
    assert_eq!(t.nodes[3].depth, 2);
}
#[test]
fn duplicate_pid_is_error() {
    assert!(matches!(
        build_tree(
            vec![process(1, None), process(1, None)],
            None,
            ProcessSort::Pid
        ),
        Err(ProcessError::DuplicatePid(1))
    ));
}
#[test]
fn selected_branch_depth_and_missing_pid() {
    let t = build_tree(
        vec![process(1, None), process(2, Some(1)), process(3, Some(2))],
        Some(ProcessId(2)),
        ProcessSort::Pid,
    )
    .unwrap();
    assert_eq!(pids(&t, &t.roots), [2]);
    assert_eq!(t.nodes[1].depth, 0);
    assert_eq!(t.nodes[2].depth, 1);
    assert!(matches!(
        build_tree(vec![], Some(ProcessId(42)), ProcessSort::Pid),
        Err(ProcessError::NotFound(42))
    ));
}
#[test]
fn every_sort_and_pid_tiebreaker() {
    for (sort, expected) in [
        (ProcessSort::Pid, vec![2, 3, 4]),
        (ProcessSort::Cpu, vec![4, 2, 3]),
        (ProcessSort::Memory, vec![4, 3, 2]),
        (ProcessSort::Name, vec![3, 4, 2]),
    ] {
        let mut input = vec![
            process(1, None),
            process(3, Some(1)),
            process(4, Some(1)),
            process(2, Some(1)),
        ];
        input[1].cpu_usage = 2.0;
        input[1].name = "a".into();
        input[2].name = "a".into();
        input[3].name = "z".into();
        let t = build_tree(input, None, sort).unwrap();
        assert_eq!(pids(&t, &t.nodes[t.roots[0]].children), expected);
    }
}
#[test]
fn deep_chain_builds_and_drops_without_recursion() {
    let t = tree(
        (1..=50_000)
            .rev()
            .map(|pid| process(pid, Some(pid - 1)))
            .collect(),
    );
    assert_eq!(t.nodes[0].depth, 49_999);
    drop(t);
}
#[test]
fn cycle_resolution_is_input_order_independent() {
    for input in [
        vec![
            process(1, Some(3)),
            process(2, Some(1)),
            process(3, Some(2)),
        ],
        vec![
            process(3, Some(2)),
            process(1, Some(3)),
            process(2, Some(1)),
        ],
    ] {
        let t = tree(input);
        assert_eq!(pids(&t, &t.roots), [1]);
        assert_eq!(pids(&t, &t.nodes[t.roots[0]].children), [2]);
    }
}
struct Fake(Result<Vec<ProcessInfo>, ProcessError>);
impl ProcessRepository for Fake {
    fn processes(&mut self) -> Result<Vec<ProcessInfo>, ProcessError> {
        std::mem::replace(&mut self.0, Ok(vec![]))
    }
}
#[test]
fn use_case_uses_repository_and_propagates_errors() {
    let t = GetProcessTree::execute(
        &mut Fake(Ok(vec![process(1, None)])),
        None,
        ProcessSort::Pid,
    )
    .unwrap();
    assert_eq!(t.nodes.len(), 1);
    assert!(matches!(
        GetProcessTree::execute(
            &mut Fake(Err(ProcessError::Unavailable("denied".into()))),
            None,
            ProcessSort::Pid
        ),
        Err(ProcessError::Unavailable(_))
    ));
}
