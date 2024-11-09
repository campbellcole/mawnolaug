use color_eyre::eyre::Result;
use poise::{BoxFuture, CreateReply};

use crate::data::FrameworkError;

/// A custom error handler to improve upon some of the builtin error handling
/// behavior. Falls back to the builtins for most cases.
async fn handle_error_inner(err: FrameworkError<'_>) -> Result<()> {
    match err {
        FrameworkError::Setup { error, .. } => {
            error!("setup error: {:?}", error);
        }
        FrameworkError::Command { error, ctx, .. } => {
            error!("command error: {:?}", error);
            ctx.send(
                CreateReply::default()
                    .content(format!("Error: {:#}", error))
                    .ephemeral(true),
            )
            .await?;
        }
        FrameworkError::MissingBotPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            error!("missing bot permissions: {}", missing_permissions);
            ctx.send(
                CreateReply::default()
                    .content(format!("Missing bot permissions: {}", missing_permissions))
                    .ephemeral(true),
            )
            .await?;
        }
        _ => {
            poise::builtins::on_error(err).await?;
        }
    }

    Ok(())
}

pub fn handle_error(err: FrameworkError<'_>) -> BoxFuture<()> {
    Box::pin(async move {
        if let Err(err) = handle_error_inner(err).await {
            error!("error handling error: {:?}", err);
        }
    })
}
