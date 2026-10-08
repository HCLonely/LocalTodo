use chrono::{TimeZone, Utc};
use todo_core::*;
fn main() {
    let mut store = Store::memory().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 10, 8, 2, 0, 0).unwrap();
    for i in 0..10000 {
        store
            .save_task(
                None,
                TaskDraft {
                    title: format!("任务 {i}"),
                    due_date: Some("2026-10-08".parse().unwrap()),
                    ..Default::default()
                },
                "single",
                now,
            )
            .unwrap();
    }
    let query = Query {
        view: "today".into(),
        selected_date: None,
        search: String::new(),
        priority: None,
        offset: 0,
        limit: 50,
    };
    let mut timings = vec![];
    for _ in 0..20 {
        let start = std::time::Instant::now();
        let data = store.snapshot(&query, now).unwrap();
        assert_eq!(data.total, 10000);
        assert_eq!(data.tasks.len(), 50);
        timings.push(start.elapsed().as_millis());
    }
    timings.sort();
    println!(
        "10,000 tasks, 20 queries: median {} ms, p95 {} ms",
        timings[10], timings[18]
    );
}
