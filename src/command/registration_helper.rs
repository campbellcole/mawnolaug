use crate::data::Context;

super::command! {
    true;
    /// Display a dialog allowing debug control over command registration
    ///
    /// **Admin only** *Debug builds only*
    pub async fn registration_helper(ctx: Context<'_>) -> Result<()> {
        poise::builtins::register_application_commands_buttons(ctx).await?;

        Ok(())
    }
}
