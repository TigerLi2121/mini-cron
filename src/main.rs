use std::time::Duration;

use chrono::Local;
mod task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = task::CronManager::new();

    // 添加每年1月1日执行的任务
    match manager.add("30 * * * * ?", || {
        println!("{} Happy New Year!", Local::now());
    }) {
        Ok(task_id) => println!("1 Task id {} added successfully", task_id),
        Err(e) => eprintln!("Failed to add task: {}", e),
    };

    // 添加每小时执行的任务
    // match manager.add("*/2 * 16 22 05 ? 2025", || {
    //     println!("{} Hourly check executed", Local::now());
    // }) {
    //     Ok(task_id) => println!("2 Task id {} added successfully", task_id),
    //     Err(e) => eprintln!("Failed to add task: {}", e),
    // }

    tokio::time::sleep(Duration::from_secs(40)).await;
    match manager.remove(1) {
        Ok(_) => {
            println!("Task 1 removed successfully");
        }
        Err(e) => eprintln!("Failed to remove task 1: {}", e),
    };

    // for idx in 1..=10000 {
    //     match manager.add("10 26 16 22 05 ? 2025", move || {
    //         println!("{} Hourly check executed {}", Local::now(), idx);
    //     }) {
    //         Ok(task_id) => println!("2 Task id {} added successfully", task_id),
    //         Err(e) => eprintln!("Failed to add task: {}", e),
    //     }
    // }

    // 使用更合理的等待间隔
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
