use chrono::{DateTime, Datelike, Local, Timelike};
use cron::Schedule;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::interval;

// 任务执行函数
type TaskFn = Arc<dyn Fn() + Send + Sync + 'static>;

// 时间轮层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TimeWheelLevel {
    Second,
    Minute,
    Hour,
    Day,
    Month,
    Year,
}

// 时间轮桶，存储待执行的任务和对应的Cron表达式
#[derive(Clone)]
struct TimeWheelBucket {
    tasks: HashMap<u64, (Arc<TaskFn>, String, TimeWheelLevel)>,
}

// 多层级时间轮
struct TimeWheel {
    levels: HashMap<TimeWheelLevel, Vec<TimeWheelBucket>>,
    current_ticks: HashMap<TimeWheelLevel, usize>,
    task_counter: AtomicU64,
    current_date: DateTime<Local>,
    // 添加任务索引，方便快速查找
    task_index: HashMap<u64, (TimeWheelLevel, usize)>,
}

// Cron任务管理器
#[derive(Clone)]
pub struct CronManager {
    time_wheel: Arc<Mutex<TimeWheel>>,
}

// 获取指定年月的天数
fn days_in_month(year: i32, month: u32) -> u32 {
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    match month {
        2 => {
            if is_leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

// 初始化时间轮
impl TimeWheel {
    fn new() -> Self {
        let now = Local::now();
        let current_year = now.year();
        let current_month = now.month();
        let current_day = days_in_month(current_year, current_month);

        let mut levels = HashMap::new();
        let mut current_ticks = HashMap::new();

        // 初始化各层级时间轮
        levels.insert(
            TimeWheelLevel::Second,
            vec![
                TimeWheelBucket {
                    tasks: HashMap::new()
                };
                60
            ],
        );
        levels.insert(
            TimeWheelLevel::Minute,
            vec![
                TimeWheelBucket {
                    tasks: HashMap::new()
                };
                60
            ],
        );
        levels.insert(
            TimeWheelLevel::Hour,
            vec![
                TimeWheelBucket {
                    tasks: HashMap::new()
                };
                24
            ],
        );
        levels.insert(
            TimeWheelLevel::Day,
            vec![
                TimeWheelBucket {
                    tasks: HashMap::new()
                };
                current_day as usize
            ],
        );
        levels.insert(
            TimeWheelLevel::Month,
            vec![
                TimeWheelBucket {
                    tasks: HashMap::new()
                };
                12
            ],
        );
        levels.insert(
            TimeWheelLevel::Year,
            vec![
                TimeWheelBucket {
                    tasks: HashMap::new()
                };
                100
            ],
        );

        // 初始化当前指针
        current_ticks.insert(TimeWheelLevel::Second, 0);
        current_ticks.insert(TimeWheelLevel::Minute, 0);
        current_ticks.insert(TimeWheelLevel::Hour, 0);
        current_ticks.insert(TimeWheelLevel::Day, current_day as usize - 1);
        current_ticks.insert(TimeWheelLevel::Month, current_month as usize - 1);
        current_ticks.insert(TimeWheelLevel::Year, (current_year % 100) as usize);

        TimeWheel {
            levels,
            current_ticks,
            task_counter: AtomicU64::new(1),
            current_date: now,
            task_index: HashMap::new(),
        }
    }

    // 添加Cron任务到时间轮
    fn add_task(&mut self, mut task_id: u64, cron_expr: &str, task: TaskFn) -> Result<u64, String> {
        let schedule =
            Schedule::from_str(cron_expr).map_err(|e| format!("无效的Cron表达式: {}", e))?;

        let next_run = schedule
            .upcoming(chrono::Local)
            .next()
            .ok_or_else(|| format!("无法计算下一次执行时间 cron:{}", cron_expr))?;

        if task_id == 0 {
            task_id = self.task_counter.fetch_add(1, Ordering::SeqCst)
        }

        // 计算任务应该放入哪个层级和桶
        let (level, bucket_index) = self.calculate_bucket_position(&next_run)?;

        // 添加到对应的桶中
        if let Some(level_buckets) = self.levels.get_mut(&level) {
            if bucket_index < level_buckets.len() {
                level_buckets[bucket_index]
                    .tasks
                    .insert(task_id, (Arc::new(task), cron_expr.to_string(), level));

                // 添加到索引
                self.task_index.insert(task_id, (level, bucket_index));

                // println!(
                //     "任务添加成功，任务ID: {}, 下次执行时间: {}, 层级: {:?}, 桶索引: {}, 总任务数: {}",
                //     task_id,
                //     next_run,
                //     level,
                //     bucket_index,
                //     self.task_index.len()
                // );
                return Ok(task_id);
            }
        }

        Err("无法调度任务".to_string())
    }

    fn calculate_bucket_position(
        &self,
        time: &chrono::DateTime<Local>,
    ) -> Result<(TimeWheelLevel, usize), String> {
        // 计算时间差
        let duration = time.signed_duration_since(self.current_date);
        let days = duration.num_days();
        let hours = duration.num_hours();
        let minutes = duration.num_minutes();
        let seconds = duration.num_seconds();

        // 根据时间差确定应该放入哪个层级
        let level = if days > 365 {
            TimeWheelLevel::Year
        } else if days > 30 {
            TimeWheelLevel::Month
        } else if days > 0 {
            TimeWheelLevel::Day
        } else if hours > 0 {
            TimeWheelLevel::Hour
        } else if minutes > 0 {
            TimeWheelLevel::Minute
        } else if seconds >= 0 {
            TimeWheelLevel::Second
        } else {
            return Err("无效的时间差".to_string());
        };

        // 计算桶索引，考虑溢出情况
        let bucket_index = match level {
            TimeWheelLevel::Second => time.second() as usize % 60,
            TimeWheelLevel::Minute => time.minute() as usize % 60,
            TimeWheelLevel::Hour => time.hour() as usize % 24,
            TimeWheelLevel::Day => {
                let day = time.day().checked_sub(1).ok_or("Invalid day")?;
                day as usize % days_in_month(time.year(), time.month()) as usize
            }
            TimeWheelLevel::Month => {
                (time.month().checked_sub(1).ok_or("Invalid month")? as usize) % 12
            }
            TimeWheelLevel::Year => (time.year() % 100) as usize % 100,
        };

        // 验证索引是否在有效范围内
        if let Some(level_buckets) = self.levels.get(&level) {
            if bucket_index >= level_buckets.len() {
                return Err(format!(
                    "桶索引 {} 超出范围 ({})",
                    bucket_index,
                    level_buckets.len()
                ));
            }
        } else {
            return Err(format!("未找到层级 {:?} 的时间轮", level));
        }

        Ok((level, bucket_index))
    }

    // 推进时间轮一个刻度
    fn tick(&mut self, level: TimeWheelLevel) -> Vec<(u64, Arc<TaskFn>, String, TimeWheelLevel)> {
        // 预分配合理大小的 Vec
        let mut tasks_to_execute = Vec::with_capacity(32);

        self.current_date = Local::now();

        let current_tick = match level {
            TimeWheelLevel::Second => self.current_date.second() as usize,
            TimeWheelLevel::Minute => self.current_date.minute() as usize,
            TimeWheelLevel::Hour => self.current_date.hour() as usize,
            TimeWheelLevel::Day => (self.current_date.day() - 1) as usize,
            TimeWheelLevel::Month => (self.current_date.month() - 1) as usize,
            TimeWheelLevel::Year => (self.current_date.year() % 100) as usize,
        };

        if let Some(prev_tick) = self.current_ticks.get(&level) {
            if *prev_tick != current_tick {
                // 执行当前层级的任务
                if let Some(buckets) = self.levels.get_mut(&level) {
                    if let Some(bucket) = buckets.get_mut(current_tick) {
                        tasks_to_execute.extend(bucket.tasks.drain().map(
                            |(task_id, (task, cron_expr, level))| (task_id, task, cron_expr, level),
                        ));
                    }
                }

                self.current_ticks.insert(level, current_tick);

                // 处理高层级任务检查
                match level {
                    TimeWheelLevel::Second => {
                        // 每次秒级tick都检查分钟级任务
                        tasks_to_execute
                            .extend(self.check_higher_level_tasks(TimeWheelLevel::Minute));
                    }
                    TimeWheelLevel::Minute => {
                        tasks_to_execute
                            .extend(self.check_higher_level_tasks(TimeWheelLevel::Hour));
                    }
                    TimeWheelLevel::Hour => {
                        tasks_to_execute.extend(self.check_higher_level_tasks(TimeWheelLevel::Day));
                    }
                    TimeWheelLevel::Day => {
                        if self.current_date.day() == 1 {
                            self.adjust_day_wheel();
                            tasks_to_execute
                                .extend(self.check_higher_level_tasks(TimeWheelLevel::Month));
                        }
                    }
                    TimeWheelLevel::Month => {
                        if self.current_date.month() == 1 {
                            tasks_to_execute
                                .extend(self.check_higher_level_tasks(TimeWheelLevel::Year));
                        }
                    }
                    _ => {}
                }
            }
        }

        tasks_to_execute
    }

    // 新增方法：检查更高层级的任务
    fn check_higher_level_tasks(
        &mut self,
        level: TimeWheelLevel,
    ) -> Vec<(u64, Arc<TaskFn>, String, TimeWheelLevel)> {
        let current_tick = match level {
            TimeWheelLevel::Minute => self.current_date.minute() as usize,
            TimeWheelLevel::Hour => self.current_date.hour() as usize,
            TimeWheelLevel::Day => (self.current_date.day() - 1) as usize,
            TimeWheelLevel::Month => (self.current_date.month() - 1) as usize,
            TimeWheelLevel::Year => (self.current_date.year() % 100) as usize,
            _ => return Vec::new(),
        };

        let mut tasks = Vec::new();
        if let Some(buckets) = self.levels.get_mut(&level) {
            if let Some(bucket) = buckets.get_mut(current_tick) {
                tasks.extend(
                    bucket
                        .tasks
                        .drain()
                        .map(|(task_id, (task, cron_expr, level))| {
                            (task_id, task, cron_expr, level)
                        }),
                );
            }
        }
        tasks
    }

    fn adjust_day_wheel(&mut self) {
        let days_this_month = days_in_month(self.current_date.year(), self.current_date.month());

        if let Some(day_buckets) = self.levels.get_mut(&TimeWheelLevel::Day) {
            // 如果当前桶数量与新月份的天数不一致，则调整
            if day_buckets.len() != days_this_month as usize {
                *day_buckets = vec![
                    TimeWheelBucket {
                        tasks: HashMap::new()
                    };
                    days_this_month as usize
                ];
            }
        }
    }

    // 新增删除任务方法
    fn remove_task(&mut self, task_id: u64) -> Result<(), String> {
        // 使用索引快速定位任务
        if let Some((level, bucket_index)) = self.task_index.remove(&task_id) {
            if let Some(buckets) = self.levels.get_mut(&level) {
                if let Some(bucket) = buckets.get_mut(bucket_index) {
                    bucket.tasks.remove(&task_id);
                    return Ok(());
                }
            }
        }
        Err(format!("未找到任务 ID: {}", task_id))
    }
}

// 实现Cron任务管理器
impl CronManager {
    pub fn new() -> Self {
        let time_wheel = Arc::new(Mutex::new(TimeWheel::new()));

        let manager = CronManager { time_wheel };

        // 启动时间轮调度器
        manager.start_scheduler();
        manager
    }

    // 添加任务
    pub fn add(
        &mut self,
        cron_expr: &str,
        task: impl Fn() + Send + Sync + 'static,
    ) -> Result<u64, String> {
        let task_fn = Arc::new(task);
        let cron_expr = cron_expr.to_string();

        // 只添加到时间轮，不创建单独的调度器
        let mut time_wheel = self
            .time_wheel
            .try_lock()
            .map_err(|_| "获取时间轮锁失败".to_string())?;
        time_wheel.add_task(0, &cron_expr, task_fn)
    }

    // 删除任务
    pub fn remove(&mut self, task_id: u64) -> Result<(), String> {
        let mut time_wheel = self
            .time_wheel
            .lock()
            .map_err(|_| "获取时间轮锁失败".to_string())?;
        time_wheel.remove_task(task_id)
    }

    // 启动时间轮调度器
    fn start_scheduler(&self) {
        let time_wheel = self.time_wheel.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            loop {
                interval.tick().await;

                let tasks = match time_wheel.lock() {
                    Ok(mut wheel) => wheel.tick(TimeWheelLevel::Second),
                    Err(_) => continue,
                };

                // 在 CronManager 的 start_scheduler 方法中修改重调度逻辑
                for (task_id, task, cron_expr, level) in tasks {
                    // 直接执行秒级任务
                    if level == TimeWheelLevel::Second {
                        let task = task.clone();
                        tokio::spawn(async move {
                            task();
                        });
                    }

                    // 重新调度任务
                    let wheel = time_wheel.clone();
                    let task = task.clone();
                    tokio::spawn(async move {
                        if let Ok(mut wheel) = wheel.lock() {
                            // 检查任务是否仍然存在
                            if wheel.task_index.contains_key(&task_id) {
                                if let Err(e) =
                                    wheel.add_task(task_id, &cron_expr, Arc::new(move || task()))
                                {
                                    eprintln!("任务 {} 重新调度失败: {}", task_id, e);
                                }
                            } else {
                                // 任务已被删除,跳过重调度
                                // println!("任务 {} 已被删除,跳过重调度", task_id);
                            }
                        }
                    });
                }
            }
        });
    }
}
