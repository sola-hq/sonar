use crate::configure_job_notifications;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveTime, TimeDelta, Timelike, Utc};
use sonar_db::{CandlestickInterval, Database};
use std::sync::Arc;
use tokio_cron_scheduler::{job::JobId, Job, JobScheduler, JobSchedulerError};
use tracing::{error, info, instrument, warn};

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
    let time_delta = TimeDelta::new(60, 0).context("Failed to create one minute time delta")?;
    aggregate_candlesticks(db, CandlestickInterval::OneMinute, time_delta, |time| {
        let end_time = time
            .date_naive()
            .and_time(
                NaiveTime::from_hms_opt(time.hour(), time.minute(), 0)
                    .expect("Failed to create naive time"),
            )
            .and_utc();
        Ok(end_time)
    })
    .await
}

/// Aggregate swap events into 1 hour candlesticks
#[instrument(skip(db))]
pub async fn aggregate_hour_candlesticks(db: Arc<Database>) -> Result<()> {
    let time_delta = TimeDelta::new(3600, 0).context("Failed to create one hour time delta")?;
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
    let time_delta = TimeDelta::new(86400, 0).context("Failed to create one day time delta")?;
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
#[instrument(skip(scheduler, db))]
pub async fn run_jobs(scheduler: &mut JobScheduler, db: Arc<Database>) -> Result<Vec<JobId>> {
    // Configure shutdown handler before starting jobs
    scheduler.shutdown_on_ctrl_c();
    scheduler.set_shutdown_handler(Box::new(|| {
        Box::pin(async move {
            info!("Shut down done");
        })
    }));

    let jobs = vec![
        create_minute_job(scheduler, db.clone()).await?,
        create_hour_job(scheduler, db.clone()).await?,
        create_day_job(scheduler, db.clone()).await?,
    ];

    if let Err(e) = scheduler.start().await {
        error!(error = ?e, "Error starting scheduler");
        return Err(anyhow!("Error starting scheduler: {}", e));
    }

    Ok(jobs)
}

/// Create and configure the minute candlestick job
async fn create_minute_job(scheduler: &mut JobScheduler, db: Arc<Database>) -> Result<JobId> {
    let db_clone = db.clone();
    let name = "aggregate minute candlesticks";
    let schedule = "0 * * * * *".to_string();

    let job = Job::new_async(&schedule, move |_uuid, _lock| {
        let db = db_clone.clone();
        Box::pin(async move {
            if let Err(e) = aggregate_minute_candlesticks(db).await {
                error!(error = ?e, "Failed to aggregate minute candlesticks");
            }
        })
    })?;

    let guid = job.guid();
    info!(job_id = ?guid, "Created minute candlestick job");

    // Configure notifications first
    if let Err(e) = configure_job_notifications(name, scheduler, job.clone()).await {
        warn!(error = ?e, job_id = ?guid, "Failed to configure job notifications");
    }

    // Then add job to scheduler
    scheduler.add(job).await?;
    Ok(guid)
}

/// Create and configure the hour candlestick job
#[instrument(skip(scheduler, db))]
async fn create_hour_job(scheduler: &mut JobScheduler, db: Arc<Database>) -> Result<JobId> {
    let db_clone = db.clone();
    let name = "aggregate hour candlesticks";
    let schedule = "0 0 * * * *".to_string();

    let job = Job::new_async(&schedule, move |_uuid, _lock| {
        let db = db_clone.clone();
        Box::pin(async move {
            if let Err(e) = aggregate_hour_candlesticks(db).await {
                error!(error = ?e, "Failed to aggregate hour candlesticks");
            }
        })
    })?;

    let guid = job.guid();
    info!(job_id = ?guid, "Created hour candlestick job");

    // Configure notifications first
    if let Err(e) = configure_job_notifications(name, scheduler, job.clone()).await {
        warn!(error = ?e, job_id = ?guid, "Failed to configure job notifications");
    }

    // Then add job to scheduler
    scheduler.add(job).await?;
    Ok(guid)
}

/// Create and configure the day candlestick job
#[instrument(skip(scheduler, db))]
async fn create_day_job(scheduler: &mut JobScheduler, db: Arc<Database>) -> Result<JobId> {
    let db_clone = db.clone();
    let name = "aggregate day candlesticks";
    let schedule = "0 0 0 * * *".to_string();

    let job = Job::new_async(&schedule, move |_uuid, _lock| {
        let db = db_clone.clone();
        Box::pin(async move {
            if let Err(e) = aggregate_day_candlesticks(db).await {
                error!(error = ?e, "Failed to aggregate day candlesticks");
            }
        })
    })?;

    let guid = job.guid();
    info!(job_id = ?guid, "Created day candlestick job");

    // Configure notifications first
    if let Err(e) = configure_job_notifications(name, scheduler, job.clone()).await {
        warn!(error = ?e, job_id = ?guid, "Failed to configure job notifications");
    }

    // Then add job to scheduler
    scheduler.add(job).await?;
    Ok(guid)
}

/// Stop all jobs and shutdown the scheduler
#[instrument(skip(scheduler, jobs))]
pub async fn stop_jobs(
    scheduler: &mut JobScheduler,
    jobs: Vec<JobId>,
    graceful_shutdown_timeout: tokio::time::Duration,
) -> Result<(), JobSchedulerError> {
    for job_id in jobs {
        if let Err(e) = scheduler.remove(&job_id).await {
            warn!(error = ?e, job_id = ?job_id, "Failed to remove job");
        }
    }
    tokio::time::sleep(graceful_shutdown_timeout).await;
    info!("Removed all jobs");
    scheduler.shutdown().await?;
    info!("Shut down scheduler");
    Ok(())
}

#[cfg(test)]
mod tests {
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
}
