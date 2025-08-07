use crate::configure_job_notifications;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveTime, TimeDelta, Timelike, Utc};
use sonar_db::{CandlestickInterval, Database};
use std::sync::Arc;
use tokio_cron_scheduler::{job::JobId, Job, JobScheduler, JobSchedulerError};
use tracing::{error, info, instrument, warn};

// Time constants
const MINUTE_IN_SECONDS: i64 = 60;
const HOUR_IN_SECONDS: i64 = 3600;
const DAY_IN_SECONDS: i64 = 86400;

// Cron schedule
const MINUTE_SCHEDULE: &str = "0 * * * * *";
const HOUR_SCHEDULE: &str = "0 0 * * * *";
const DAY_SCHEDULE: &str = "0 0 0 * * *";

/// Generic function to aggregate candlesticks
#[instrument(skip(db, get_end_time), fields(interval = ?interval))]
async fn aggregate_candlesticks(
    db: Arc<Database>,
    interval: CandlestickInterval,
    time_delta: TimeDelta,
    get_end_time: impl FnOnce(DateTime<Utc>) -> Result<DateTime<Utc>>,
) -> Result<()> {
    let db_clone = db.clone();
    let end_time: DateTime<Utc> = get_end_time(Utc::now())?;
    let start_time =
        end_time.checked_sub_signed(time_delta).context("Failed to subtract time delta")?;
    let start_ts = start_time.timestamp();
    let end_ts = end_time.timestamp();

    info!(
        candlesticks_range = ?(start_ts, end_ts),
        interval = ?interval,
        "Aggregating candlesticks"
    );

    db_clone
        .aggregate_into_candlesticks(start_ts, end_ts, interval)
        .await
        .context("Failed to aggregate into candlesticks")?;
    Ok(())
}

/// Aggregate swap events into 1 minute candlesticks
#[instrument(skip(db))]
pub async fn aggregate_minute_candlesticks(db: Arc<Database>) -> Result<()> {
    let time_delta =
        TimeDelta::new(MINUTE_IN_SECONDS, 0).context("Failed to create one minute time delta")?;
    aggregate_candlesticks(db, CandlestickInterval::OneMinute, time_delta, |time| {
        let end_time = time
            .date_naive()
            .and_time(
                NaiveTime::from_hms_opt(time.hour(), time.minute(), 0)
                    .context("Failed to create naive time")?,
            )
            .and_utc();
        Ok(end_time)
    })
    .await
}

/// Aggregate swap events into 1 hour candlesticks
#[instrument(skip(db))]
pub async fn aggregate_hour_candlesticks(db: Arc<Database>) -> Result<()> {
    let time_delta =
        TimeDelta::new(HOUR_IN_SECONDS, 0).context("Failed to create one hour time delta")?;
    aggregate_candlesticks(db, CandlestickInterval::OneHour, time_delta, |time| {
        let end_time = time
            .date_naive()
            .and_time(
                NaiveTime::from_hms_opt(time.hour(), 0, 0)
                    .context("Failed to create naive time")?,
            )
            .and_utc();
        Ok(end_time)
    })
    .await
}

/// Aggregate swap events into 1 day candlesticks
#[instrument(skip(db))]
pub async fn aggregate_day_candlesticks(db: Arc<Database>) -> Result<()> {
    let time_delta =
        TimeDelta::new(DAY_IN_SECONDS, 0).context("Failed to create one day time delta")?;
    aggregate_candlesticks(db, CandlestickInterval::OneDay, time_delta, |time| {
        let end_time = time
            .date_naive()
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).context("Failed to create naive time")?)
            .and_utc();
        Ok(end_time)
    })
    .await
}

/// Run all scheduled jobs
#[instrument(skip(sched, db))]
pub async fn run_jobs(sched: &mut JobScheduler, db: Arc<Database>) -> Result<Vec<JobId>> {
    // Configure shutdown handler before starting jobs
    sched.shutdown_on_ctrl_c();
    sched.set_shutdown_handler(Box::new(|| {
        Box::pin(async move {
            info!("Shutdown done");
        })
    }));

    let jobs = vec![
        create_minute_job(sched, db.clone()).await?,
        create_hour_job(sched, db.clone()).await?,
        create_day_job(sched, db.clone()).await?,
    ];

    if let Err(e) = sched.start().await {
        error!(error = ?e, "Error starting sched");
        return Err(anyhow!("Error starting sched: {}", e));
    }

    Ok(jobs)
}

/// Create and configure the minute candlestick job
async fn create_minute_job(sched: &mut JobScheduler, db: Arc<Database>) -> Result<JobId> {
    let db_clone = db.clone();
    let name = "aggregate minute candlesticks";
    let schedule = MINUTE_SCHEDULE.to_string();

    let job = Job::new_async(&schedule, move |_uuid, _lock| {
        let db = db_clone.clone();
        Box::pin(async move {
            let result = aggregate_minute_candlesticks(db).await;
            match result {
                Ok(()) => {
                    info!("Aggregated minutely candlesticks");
                }
                Err(e) => {
                    error!(error = ?e, "Failed to aggregate minutely candlesticks");
                }
            }
        })
    })?;

    let guid = job.guid();
    info!(job_id = ?guid, "Created minutely candlestick job");

    // Configure notifications with error handling
    if let Err(e) = configure_job_notifications(name, sched, job.clone()).await {
        warn!(error = ?e, job_id = ?guid, "Failed to configure job notifications, but continuing with job creation");
    }

    // Then add job to scheduler
    sched.add(job).await?;
    Ok(guid)
}

/// Create and configure the hour candlestick job
#[instrument(skip(sched, db))]
async fn create_hour_job(sched: &mut JobScheduler, db: Arc<Database>) -> Result<JobId> {
    let db_clone = db.clone();
    let name = "aggregate hour candlesticks";
    let schedule = HOUR_SCHEDULE.to_string();

    let job = Job::new_async(&schedule, move |_uuid, _lock| {
        let db = db_clone.clone();
        Box::pin(async move {
            let result = aggregate_hour_candlesticks(db).await;
            match result {
                Ok(()) => {
                    info!("Aggregated hourly candlesticks");
                }
                Err(e) => {
                    error!(error = ?e, "Failed to aggregate hour candlesticks");
                }
            }
        })
    })?;

    let guid = job.guid();
    info!(job_id = ?guid, "Created hourly candlestick job");

    // Configure notifications with error handling
    if let Err(e) = configure_job_notifications(name, sched, job.clone()).await {
        warn!(error = ?e, job_id = ?guid, "Failed to configure job notifications, but continuing with job creation");
    }

    // Then add job to sched
    sched.add(job).await?;
    Ok(guid)
}

/// Create and configure the day candlestick job
#[instrument(skip(sched, db))]
async fn create_day_job(sched: &mut JobScheduler, db: Arc<Database>) -> Result<JobId> {
    let db_clone = db.clone();
    let name = "aggregate day candlesticks";
    let schedule = DAY_SCHEDULE.to_string();

    let job = Job::new_async(&schedule, move |_uuid, _lock| {
        let db = db_clone.clone();
        Box::pin(async move {
            let result = aggregate_day_candlesticks(db).await;
            match result {
                Ok(()) => {
                    info!("Aggregated daily candlesticks");
                }
                Err(e) => {
                    error!(error = ?e, "Failed to aggregate day candlesticks");
                }
            }
        })
    })?;

    let guid = job.guid();
    info!(job_id = ?guid, "Created day candlestick job");

    // Configure notifications with error handling
    if let Err(e) = configure_job_notifications(name, sched, job.clone()).await {
        warn!(error = ?e, job_id = ?guid, "Failed to configure job notifications, but continuing with job creation");
    }

    // Then add job to sched
    sched.add(job).await?;
    Ok(guid)
}

/// Stop all jobs and shutdown the scheduler
#[instrument(skip(sched))]
pub async fn stop_jobs(
    sched: &mut JobScheduler,
    _jobs: Vec<JobId>,
    graceful_shutdown_timeout: tokio::time::Duration,
) -> Result<(), JobSchedulerError> {
    tokio::select! {
        shutdown_result = sched.shutdown() => {
            for job in _jobs {
                sched.remove(&job).await?;
            }
            if let Err(e) = shutdown_result {
                warn!(error = ?e, "Error shutting down scheduler");
                return Err(e);
            }
            info!("Scheduler shutdown completed successfully");
        }
        _ = tokio::time::sleep(graceful_shutdown_timeout) => {
            warn!("Scheduler shutdown timeout");
        }
    }
    Ok(())
}

#[cfg(test)]
mod job_tests {
    use super::*;
    use chrono::NaiveTime;

    #[tokio::test]
    async fn test_start_of_day() {
        let now: DateTime<Utc> = "2025-05-23T07:55:29.860Z".parse().expect("Failed to parse date");
        let start_of_day: DateTime<Utc> =
            "2025-05-23T00:00:00.000Z".parse().expect("Failed to parse date");
        let start_ts = start_of_day.timestamp();

        // Get the start of the day (00:00:00) for that date
        let time = now
            .date_naive()
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).expect("Failed to create naive time"));
        let timestamp = time.and_utc().timestamp();
        assert_eq!(time.hour(), 0);
        assert_eq!(time.minute(), 0);
        assert_eq!(time.second(), 0);
        assert_eq!(time.nanosecond(), 0);
        assert_eq!(timestamp, start_ts);
        assert_eq!(start_ts, 1747958400);
    }

    #[tokio::test]
    async fn test_start_of_hour() {
        let now: DateTime<Utc> = "2025-05-23T07:55:29.860Z".parse().expect("Failed to parse date");
        let start_of_hour: DateTime<Utc> =
            "2025-05-23T07:00:00.000Z".parse().expect("Failed to parse date");

        let time = now.date_naive().and_time(
            NaiveTime::from_hms_opt(now.hour(), 0, 0).expect("Failed to create naive time"),
        );
        let timestamp = time.and_utc().timestamp();
        assert_eq!(timestamp, start_of_hour.timestamp());
    }

    #[tokio::test]
    async fn test_start_of_minute() {
        let now: DateTime<Utc> = "2025-05-23T07:55:29.860Z".parse().expect("Failed to parse date");
        let start_of_minute: DateTime<Utc> =
            "2025-05-23T07:55:00.000Z".parse().expect("Failed to parse date");

        let time = now.date_naive().and_time(
            NaiveTime::from_hms_opt(now.hour(), now.minute(), 0)
                .expect("Failed to create naive time"),
        );
        let timestamp = time.and_utc().timestamp();
        assert_eq!(timestamp, start_of_minute.timestamp());
    }

    #[test]
    fn test_constants() {
        assert_eq!(MINUTE_IN_SECONDS, 60);
        assert_eq!(HOUR_IN_SECONDS, 3600);
        assert_eq!(DAY_IN_SECONDS, 86400);
        assert_eq!(MINUTE_SCHEDULE, "0 * * * * *");
        assert_eq!(HOUR_SCHEDULE, "0 0 * * * *");
        assert_eq!(DAY_SCHEDULE, "0 0 0 * * *");
    }

    /// Test the time calculation logic used in aggregate functions
    #[test]
    fn test_time_calculation_logic() {
        // Test minute aggregation logic
        let test_cases = vec![
            ("2025-05-23T07:55:29.860Z", "2025-05-23T07:55:00.000Z"),
            ("2025-05-23T07:59:59.999Z", "2025-05-23T07:59:00.000Z"),
            ("2025-05-23T00:00:00.000Z", "2025-05-23T00:00:00.000Z"),
            ("2025-05-23T23:59:59.999Z", "2025-05-23T23:59:00.000Z"),
        ];

        for (input, expected) in test_cases {
            let input_time: DateTime<Utc> = input.parse().expect("Failed to parse input time");
            let expected_time: DateTime<Utc> =
                expected.parse().expect("Failed to parse expected time");

            // Simulate the minute aggregation logic
            let calculated_time = input_time
                .date_naive()
                .and_time(
                    NaiveTime::from_hms_opt(input_time.hour(), input_time.minute(), 0)
                        .expect("Failed to create naive time"),
                )
                .and_utc();

            assert_eq!(
                calculated_time.timestamp(),
                expected_time.timestamp(),
                "Minute aggregation failed for input: {}, expected: {}, got: {}",
                input,
                expected,
                calculated_time
            );
        }

        // Test hour aggregation logic
        let hour_test_cases = vec![
            ("2025-05-23T07:55:29.860Z", "2025-05-23T07:00:00.000Z"),
            ("2025-05-23T07:59:59.999Z", "2025-05-23T07:00:00.000Z"),
            ("2025-05-23T00:00:00.000Z", "2025-05-23T00:00:00.000Z"),
            ("2025-05-23T23:59:59.999Z", "2025-05-23T23:00:00.000Z"),
        ];

        for (input, expected) in hour_test_cases {
            let input_time: DateTime<Utc> = input.parse().expect("Failed to parse input time");
            let expected_time: DateTime<Utc> =
                expected.parse().expect("Failed to parse expected time");

            // Simulate the hour aggregation logic
            let calculated_time = input_time
                .date_naive()
                .and_time(
                    NaiveTime::from_hms_opt(input_time.hour(), 0, 0)
                        .expect("Failed to create naive time"),
                )
                .and_utc();

            assert_eq!(
                calculated_time.timestamp(),
                expected_time.timestamp(),
                "Hour aggregation failed for input: {}, expected: {}, got: {}",
                input,
                expected,
                calculated_time
            );
        }

        // Test day aggregation logic
        let day_test_cases = vec![
            ("2025-05-23T07:55:29.860Z", "2025-05-23T00:00:00.000Z"),
            ("2025-05-23T23:59:59.999Z", "2025-05-23T00:00:00.000Z"),
            ("2025-05-23T00:00:00.000Z", "2025-05-23T00:00:00.000Z"),
        ];

        for (input, expected) in day_test_cases {
            let input_time: DateTime<Utc> = input.parse().expect("Failed to parse input time");
            let expected_time: DateTime<Utc> =
                expected.parse().expect("Failed to parse expected time");

            // Simulate the day aggregation logic
            let calculated_time = input_time
                .date_naive()
                .and_time(NaiveTime::from_hms_opt(0, 0, 0).expect("Failed to create naive time"))
                .and_utc();

            assert_eq!(
                calculated_time.timestamp(),
                expected_time.timestamp(),
                "Day aggregation failed for input: {}, expected: {}, got: {}",
                input,
                expected,
                calculated_time
            );
        }
    }

    /// Test time delta calculations
    #[test]
    fn test_time_delta_calculations() {
        let test_time: DateTime<Utc> =
            "2025-05-23T07:55:00.000Z".parse().expect("Failed to parse test time");

        // Test minute delta
        let minute_delta =
            TimeDelta::new(MINUTE_IN_SECONDS, 0).expect("Failed to create minute delta");
        let start_time =
            test_time.checked_sub_signed(minute_delta).expect("Failed to subtract minute delta");
        let expected_start: DateTime<Utc> =
            "2025-05-23T07:54:00.000Z".parse().expect("Failed to parse expected start");
        assert_eq!(start_time.timestamp(), expected_start.timestamp());

        // Test hour delta
        let hour_delta = TimeDelta::new(HOUR_IN_SECONDS, 0).expect("Failed to create hour delta");
        let start_time =
            test_time.checked_sub_signed(hour_delta).expect("Failed to subtract hour delta");
        let expected_start: DateTime<Utc> =
            "2025-05-23T06:55:00.000Z".parse().expect("Failed to parse expected start");
        assert_eq!(start_time.timestamp(), expected_start.timestamp());

        // Test day delta
        let day_delta = TimeDelta::new(DAY_IN_SECONDS, 0).expect("Failed to create day delta");
        let start_time =
            test_time.checked_sub_signed(day_delta).expect("Failed to subtract day delta");
        let expected_start: DateTime<Utc> =
            "2025-05-22T07:55:00.000Z".parse().expect("Failed to parse expected start");
        assert_eq!(start_time.timestamp(), expected_start.timestamp());
    }

    /// Test edge cases and boundary conditions
    #[test]
    fn test_edge_cases() {
        // Test month boundary
        let month_boundary: DateTime<Utc> =
            "2025-05-31T23:59:59.999Z".parse().expect("Failed to parse month boundary");
        let expected_day_start: DateTime<Utc> =
            "2025-05-31T00:00:00.000Z".parse().expect("Failed to parse expected day start");

        let calculated_day_start = month_boundary
            .date_naive()
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).expect("Failed to create naive time"))
            .and_utc();

        assert_eq!(calculated_day_start.timestamp(), expected_day_start.timestamp());

        // Test year boundary
        let year_boundary: DateTime<Utc> =
            "2025-12-31T23:59:59.999Z".parse().expect("Failed to parse year boundary");
        let expected_day_start: DateTime<Utc> =
            "2025-12-31T00:00:00.000Z".parse().expect("Failed to parse expected day start");

        let calculated_day_start = year_boundary
            .date_naive()
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).expect("Failed to create naive time"))
            .and_utc();

        assert_eq!(calculated_day_start.timestamp(), expected_day_start.timestamp());

        // Test leap year
        let leap_year_time: DateTime<Utc> =
            "2024-02-29T12:30:45.123Z".parse().expect("Failed to parse leap year time");
        let expected_minute_start: DateTime<Utc> =
            "2024-02-29T12:30:00.000Z".parse().expect("Failed to parse expected minute start");

        let calculated_minute_start = leap_year_time
            .date_naive()
            .and_time(
                NaiveTime::from_hms_opt(leap_year_time.hour(), leap_year_time.minute(), 0)
                    .expect("Failed to create naive time"),
            )
            .and_utc();

        assert_eq!(calculated_minute_start.timestamp(), expected_minute_start.timestamp());
    }

    /// Test that the time calculation functions handle errors properly
    #[test]
    fn test_error_handling() {
        // Test invalid time creation
        let result = NaiveTime::from_hms_opt(25, 60, 60);
        assert!(result.is_none(), "Should return None for invalid time");

        // Test valid time creation
        let result = NaiveTime::from_hms_opt(23, 59, 59);
        assert!(result.is_some(), "Should return Some for valid time");
    }
}
