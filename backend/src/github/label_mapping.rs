use crate::models::task::{TaskPriority, TaskStatus};

/// Label prefix for task status.
const STATUS_PREFIX: &str = "status:";
/// Label prefix for task priority.
const PRIORITY_PREFIX: &str = "priority:";

/// A label definition with name and color.
pub struct LabelDefinition {
    pub name: &'static str,
    pub color: &'static str,
}

/// Convert a TaskStatus to a GitHub label string.
pub fn task_status_to_label(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Backlog => "status:backlog",
        TaskStatus::Todo => "status:todo",
        TaskStatus::InProgress => "status:in_progress",
        TaskStatus::InReview => "status:in_review",
        TaskStatus::Done => "status:done",
    }
}

/// Convert a GitHub label string to a TaskStatus, if it matches.
pub fn label_to_task_status(label: &str) -> Option<TaskStatus> {
    let suffix = label.strip_prefix(STATUS_PREFIX)?;
    match suffix {
        "backlog" => Some(TaskStatus::Backlog),
        "todo" => Some(TaskStatus::Todo),
        "in_progress" => Some(TaskStatus::InProgress),
        "in_review" => Some(TaskStatus::InReview),
        "done" => Some(TaskStatus::Done),
        _ => None,
    }
}

/// Convert a TaskPriority to a GitHub label string.
pub fn task_priority_to_label(priority: &TaskPriority) -> &'static str {
    match priority {
        TaskPriority::Low => "priority:low",
        TaskPriority::Medium => "priority:medium",
        TaskPriority::High => "priority:high",
        TaskPriority::Urgent => "priority:urgent",
    }
}

/// Convert a GitHub label string to a TaskPriority, if it matches.
pub fn label_to_task_priority(label: &str) -> Option<TaskPriority> {
    let suffix = label.strip_prefix(PRIORITY_PREFIX)?;
    match suffix {
        "low" => Some(TaskPriority::Low),
        "medium" => Some(TaskPriority::Medium),
        "high" => Some(TaskPriority::High),
        "urgent" => Some(TaskPriority::Urgent),
        _ => None,
    }
}

/// Extract the TaskStatus from a list of label strings.
/// Returns None if no status label is found.
pub fn extract_status_from_labels(labels: &[String]) -> Option<TaskStatus> {
    labels.iter().find_map(|l| label_to_task_status(l))
}

/// Extract the TaskPriority from a list of label strings.
/// Returns None if no priority label is found.
pub fn extract_priority_from_labels(labels: &[String]) -> Option<TaskPriority> {
    labels.iter().find_map(|l| label_to_task_priority(l))
}

/// Build the set of label strings for a task given its status and priority.
pub fn build_labels_for_task(status: &TaskStatus, priority: &TaskPriority) -> Vec<String> {
    vec![
        task_status_to_label(status).to_string(),
        task_priority_to_label(priority).to_string(),
    ]
}

/// Return all label definitions (status + priority) for ensuring labels exist in a repo.
pub fn all_label_definitions() -> Vec<LabelDefinition> {
    vec![
        LabelDefinition {
            name: "status:backlog",
            color: "e6e6e6",
        },
        LabelDefinition {
            name: "status:todo",
            color: "0075ca",
        },
        LabelDefinition {
            name: "status:in_progress",
            color: "fbca04",
        },
        LabelDefinition {
            name: "status:in_review",
            color: "d876e3",
        },
        LabelDefinition {
            name: "status:done",
            color: "0e8a16",
        },
        LabelDefinition {
            name: "priority:low",
            color: "c5def5",
        },
        LabelDefinition {
            name: "priority:medium",
            color: "bfd4f2",
        },
        LabelDefinition {
            name: "priority:high",
            color: "d93f0b",
        },
        LabelDefinition {
            name: "priority:urgent",
            color: "b60205",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- task_status_to_label ---

    #[test]
    fn status_backlog_to_label() {
        assert_eq!(task_status_to_label(&TaskStatus::Backlog), "status:backlog");
    }

    #[test]
    fn status_todo_to_label() {
        assert_eq!(task_status_to_label(&TaskStatus::Todo), "status:todo");
    }

    #[test]
    fn status_in_progress_to_label() {
        assert_eq!(
            task_status_to_label(&TaskStatus::InProgress),
            "status:in_progress"
        );
    }

    #[test]
    fn status_in_review_to_label() {
        assert_eq!(
            task_status_to_label(&TaskStatus::InReview),
            "status:in_review"
        );
    }

    #[test]
    fn status_done_to_label() {
        assert_eq!(task_status_to_label(&TaskStatus::Done), "status:done");
    }

    // --- label_to_task_status ---

    #[test]
    fn label_to_status_backlog() {
        assert_eq!(
            label_to_task_status("status:backlog"),
            Some(TaskStatus::Backlog)
        );
    }

    #[test]
    fn label_to_status_todo() {
        assert_eq!(label_to_task_status("status:todo"), Some(TaskStatus::Todo));
    }

    #[test]
    fn label_to_status_in_progress() {
        assert_eq!(
            label_to_task_status("status:in_progress"),
            Some(TaskStatus::InProgress)
        );
    }

    #[test]
    fn label_to_status_in_review() {
        assert_eq!(
            label_to_task_status("status:in_review"),
            Some(TaskStatus::InReview)
        );
    }

    #[test]
    fn label_to_status_done() {
        assert_eq!(label_to_task_status("status:done"), Some(TaskStatus::Done));
    }

    #[test]
    fn label_to_status_unknown_returns_none() {
        assert_eq!(label_to_task_status("status:unknown"), None);
    }

    #[test]
    fn label_to_status_non_status_returns_none() {
        assert_eq!(label_to_task_status("priority:high"), None);
        assert_eq!(label_to_task_status("bug"), None);
    }

    // --- task_priority_to_label ---

    #[test]
    fn priority_low_to_label() {
        assert_eq!(task_priority_to_label(&TaskPriority::Low), "priority:low");
    }

    #[test]
    fn priority_medium_to_label() {
        assert_eq!(
            task_priority_to_label(&TaskPriority::Medium),
            "priority:medium"
        );
    }

    #[test]
    fn priority_high_to_label() {
        assert_eq!(task_priority_to_label(&TaskPriority::High), "priority:high");
    }

    #[test]
    fn priority_urgent_to_label() {
        assert_eq!(
            task_priority_to_label(&TaskPriority::Urgent),
            "priority:urgent"
        );
    }

    // --- label_to_task_priority ---

    #[test]
    fn label_to_priority_low() {
        assert_eq!(
            label_to_task_priority("priority:low"),
            Some(TaskPriority::Low)
        );
    }

    #[test]
    fn label_to_priority_medium() {
        assert_eq!(
            label_to_task_priority("priority:medium"),
            Some(TaskPriority::Medium)
        );
    }

    #[test]
    fn label_to_priority_high() {
        assert_eq!(
            label_to_task_priority("priority:high"),
            Some(TaskPriority::High)
        );
    }

    #[test]
    fn label_to_priority_urgent() {
        assert_eq!(
            label_to_task_priority("priority:urgent"),
            Some(TaskPriority::Urgent)
        );
    }

    #[test]
    fn label_to_priority_unknown_returns_none() {
        assert_eq!(label_to_task_priority("priority:critical"), None);
    }

    #[test]
    fn label_to_priority_non_priority_returns_none() {
        assert_eq!(label_to_task_priority("status:done"), None);
        assert_eq!(label_to_task_priority("enhancement"), None);
    }

    // --- extract_status_from_labels ---

    #[test]
    fn extract_status_finds_status_label() {
        let labels = vec![
            "bug".to_string(),
            "status:in_progress".to_string(),
            "priority:high".to_string(),
        ];
        assert_eq!(
            extract_status_from_labels(&labels),
            Some(TaskStatus::InProgress)
        );
    }

    #[test]
    fn extract_status_returns_none_when_absent() {
        let labels = vec!["bug".to_string(), "priority:high".to_string()];
        assert_eq!(extract_status_from_labels(&labels), None);
    }

    // --- extract_priority_from_labels ---

    #[test]
    fn extract_priority_finds_priority_label() {
        let labels = vec![
            "status:todo".to_string(),
            "priority:urgent".to_string(),
            "enhancement".to_string(),
        ];
        assert_eq!(
            extract_priority_from_labels(&labels),
            Some(TaskPriority::Urgent)
        );
    }

    #[test]
    fn extract_priority_returns_none_when_absent() {
        let labels = vec!["status:done".to_string(), "bug".to_string()];
        assert_eq!(extract_priority_from_labels(&labels), None);
    }

    // --- build_labels_for_task ---

    #[test]
    fn build_labels_includes_status_and_priority() {
        let labels = build_labels_for_task(&TaskStatus::Todo, &TaskPriority::High);
        assert_eq!(labels.len(), 2);
        assert!(labels.contains(&"status:todo".to_string()));
        assert!(labels.contains(&"priority:high".to_string()));
    }

    #[test]
    fn build_labels_done_urgent() {
        let labels = build_labels_for_task(&TaskStatus::Done, &TaskPriority::Urgent);
        assert!(labels.contains(&"status:done".to_string()));
        assert!(labels.contains(&"priority:urgent".to_string()));
    }

    // --- all_label_definitions ---

    #[test]
    fn all_label_definitions_returns_nine_labels() {
        let defs = all_label_definitions();
        // 5 status + 4 priority = 9
        assert_eq!(defs.len(), 9);
    }

    #[test]
    fn all_label_definitions_have_valid_hex_colors() {
        let defs = all_label_definitions();
        for def in &defs {
            assert!(
                def.color.len() == 6 && def.color.chars().all(|c| c.is_ascii_hexdigit()),
                "Invalid color '{}' for label '{}'",
                def.color,
                def.name
            );
        }
    }

    #[test]
    fn all_label_definitions_contain_all_statuses() {
        let defs = all_label_definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name).collect();
        assert!(names.contains(&"status:backlog"));
        assert!(names.contains(&"status:todo"));
        assert!(names.contains(&"status:in_progress"));
        assert!(names.contains(&"status:in_review"));
        assert!(names.contains(&"status:done"));
    }

    #[test]
    fn all_label_definitions_contain_all_priorities() {
        let defs = all_label_definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name).collect();
        assert!(names.contains(&"priority:low"));
        assert!(names.contains(&"priority:medium"));
        assert!(names.contains(&"priority:high"));
        assert!(names.contains(&"priority:urgent"));
    }
}
