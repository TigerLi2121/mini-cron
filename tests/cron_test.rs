#[cfg(test)]
mod tests {

    use chrono::Local;
    use cron::Schedule;
    use std::str::FromStr;

    #[test]
    fn test_cron_task() {
        let cron_expr = "5 */2 14 22 05 ? 2025";
        let schedule = Schedule::from_str(cron_expr)
            .map_err(|e| format!("无效的Cron表达式: {}", e))
            .unwrap();

        let next_time = schedule.upcoming(Local).next().unwrap();
        println!("下一个执行时间: {}", next_time);
    }

    // #[tokio::test]
    // async fn add_cron_task() {
    //     let mut manager = task::CronManager::new();
    //     match manager.add("*/2 * 16 22 05 ? 2025", || {
    //         println!("{} Hourly check executed", Local::now());
    //     }) {
    //         Ok(task_id) => println!("2 Task id {} added successfully", task_id),
    //         Err(e) => eprintln!("Failed to add task: {}", e),
    //     }

    //     tokio::time::sleep(Duration::from_secs(10)).await;
    //     match manager.remove(1) {
    //         Ok(_) => {
    //             println!("Task 1 removed successfully");
    //         }
    //         Err(e) => eprintln!("Failed to remove task 1: {}", e),
    //     };

    //     loop {
    //         tokio::time::sleep(Duration::from_secs(60)).await;
    //     }
    // }
}
