use anyhow::Result;
use std::sync::Arc;
use tokio_cron_scheduler::{
    job::{JobId, JobLocked},
    JobScheduler,
};
use tracing::{info, warn};

/// Configure job notifications for a job.
///
/// This function adds notifications to a job that will be executed when the job starts, stops, or is removed.
///
/// # Arguments
///
/// * `scheduler` - The scheduler to add the job to.
/// * `job_async` - The job to configure notifications for.
///
/// # Returns
///
/// * `JobId` - The ID of the job.
pub async fn configure_job_notifications(
    name: &str,
    sched: &mut JobScheduler,
    mut job_async: JobLocked,
) -> Result<JobId> {
    let name = Arc::new(name.to_string());

    // Add start notification (simplified - no removal logic to avoid shutdown issues)
    let name_clone = name.clone();
    if let Err(e) = job_async
        .on_start_notification_add(
            sched,
            Box::new(move |job_id, notification_id, type_of_notification| {
                let name = name_clone.clone();
                Box::pin(async move {
                    info!(
                        job = %name,
                        job_id = %job_id,
                        "Job {} {:?} started, notification {:?} ({:?})",
                        name, job_id, notification_id, type_of_notification
                    );
                })
            }),
        )
        .await
    {
        warn!(error = ?e, job_name = %name, "Failed to add start notification");
    }

    // Add stop notification
    let name_for_stop = name.clone();
    if let Err(e) = job_async
        .on_stop_notification_add(
            sched,
            Box::new(move |job_id, notification_id, type_of_notification| {
                let name = name_for_stop.clone();
                Box::pin(async move {
                    info!(
                        job = %name,
                        job_id = %job_id,
                        "Job {} {:?} was completed, notification {:?} ran ({:?})",
                        name, job_id, notification_id, type_of_notification
                    );
                })
            }),
        )
        .await
    {
        warn!(error = ?e, job_name = %name, "Failed to add stop notification");
    }

    // Add remove notification
    let name_for_remove = name.clone();
    if let Err(e) = job_async
        .on_removed_notification_add(
            sched,
            Box::new(move |job_id, notification_id, type_of_notification| {
                let name = name_for_remove.clone();
                Box::pin(async move {
                    info!(
                        job = %name,
                        job_id = %job_id,
                        "Job {} {:?} was removed, notification {:?} ran ({:?})",
                        name, job_id, notification_id, type_of_notification
                    );
                })
            }),
        )
        .await
    {
        warn!(error = ?e, job_name = %name, "Failed to add remove notification");
    }

    let one_m_job_guid = job_async.guid();
    sched.add(job_async).await?;
    Ok(one_m_job_guid)
}
