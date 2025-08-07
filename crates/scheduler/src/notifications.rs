use anyhow::Result;
use std::sync::Arc;
use tokio_cron_scheduler::{
    job::{JobId, JobLocked},
    JobScheduler,
};
use tracing::info;

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
    scheduler: &mut JobScheduler,
    mut job_async: JobLocked,
) -> Result<JobId> {
    let job_async_clone = job_async.clone();
    let js = scheduler.clone();
    let name = Arc::new(name.to_string());
    let name_clone = name.clone();
    // Add actions to be executed when the jobs starts/stop etc.
    job_async.on_start_notification_add(
        scheduler,
        Box::new(move|job_id, notification_id, type_of_notification| {
            let job_async_clone = job_async_clone.clone();
            let js = js.clone();
            let name = name_clone.clone();
            Box::pin(async move {
							let removed = job_async_clone.on_start_notification_remove(&js, &notification_id).await;
							info!(
								job = %name,
								removed = ?removed,
								job_id = %job_id,
								"Job {} {:?} ran on start notification {:?} ({:?})", name, job_id, notification_id, type_of_notification
							);
            })
        }),
    )
    .await?;

    let name_for_stop = name.clone();
    job_async
        .on_stop_notification_add(
            scheduler,
            Box::new(move |job_id, notification_id, type_of_notification| {
                let name = name_for_stop.clone();
                Box::pin(async move {
                    info!(
                        "Job {} {:?} was completed, notification {:?} ran ({:?})",
                        name, job_id, notification_id, type_of_notification
                    );
                })
            }),
        )
        .await?;

    let name_for_remove = name.clone();
    job_async
        .on_removed_notification_add(
            scheduler,
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
        .await?;

    let one_m_job_guid = job_async.guid();
    scheduler.add(job_async).await?;
    Ok(one_m_job_guid)
}
