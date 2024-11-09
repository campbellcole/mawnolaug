/// A helper to define all commands' modules and the `commands` function that
/// returns a list of all commands.
///
/// The `commands` function accepts a reference to the config and will configure
/// each command with the permissions set in the config if it is defined as
/// admin-only.
///
/// The generated `commands` function will include the `registration_helper`
/// command if the binary is built in debug mode.
macro_rules! commands {
    ($($name:ident),* $(,)?) => {
        $(
            pub mod $name;
        )*
        #[cfg(debug_assertions)]
        pub mod registration_helper;

        #[cfg(debug_assertions)]
        pub fn commands(config: & $crate::data::config::AppConfig) -> Vec<$crate::data::Command> {
            vec![$( $name::command(config) ),*, registration_helper::command(config)]
        }

        #[cfg(not(debug_assertions))]
        pub fn commands(config: & $crate::data::config::AppConfig) -> Vec<$crate::data::Command> {
            vec![$( $name::command(config) ),*]
        }
    };
}

commands! {
    create,
    create_for,
    random,
    remove,
    remove_for,
    trigger,
}

/// A thin wrapper around the `poise::command` macro to allow setting the
/// command's `default_member_permissions` to the value set in the config.
///
/// This macro generates the same output as the `poise::command` macro, but also
/// generates a function called `command` that accepts a reference to the config
/// and updates the poise-generated command definition with the permissions set
/// in the config.
///
/// Usage:
/// ```rust,no_run
/// use color_eyre::eyre::Result;
///
/// use crate::{Context, command::command};
///
/// command! {
///     // whether or not the command is admin only
///     true;
///     /// A command that does something
///     pub async fn my_command(ctx: Context<'_>) -> Result<()> {
///         // command body
///         ctx.say("Hello, world!").await?;
///
///         Ok(())
///     }
/// }
///
/// // then from another module
/// let my_command = my_command::command(&config);
/// ```
macro_rules! command {
    (
        $is_admin:literal;
        $(#[$attr:meta])*
        pub async fn $name:ident(
            $(
                $(#[$arg_attr:meta])*
                $arg:ident: $arg_ty:ty
            ),* $(,)?) -> Result<$ret_ty:ty> $body:block
    ) => {
        pub fn command(config: & $crate::data::config::AppConfig) -> $crate::data::Command {
            let mut cmd = $name();

            if $is_admin {
                cmd.default_member_permissions = *config.admin_permissions;
            }

            cmd
        }

        $(#[$attr])*
        #[::poise::command(slash_command, guild_only)]
        async fn $name($(
            $(#[$arg_attr])*
            $arg: $arg_ty,
        )*) -> ::color_eyre::eyre::Result<$ret_ty> {
            // please forgive me
            $(
                $crate::command::__trace_cmd!($arg $arg, stringify!($name));
            )*
            $body
        }
    };
}

pub(crate) use command;

// This macro uses a trick I came up with to pass identifiers to a macro without
// manually extracting them from an external macro. This is a hack to fix the
// same issue with the `poise::command` macro not recognizing the `Option` type
// except in this case it's with the `Context` argument. If I want to generate
// this macro invocation "the right way", I have to update the `command!` macro
// to explicitly accept the `Context` argument, then glob the other arguments so
// that I'll have the `ctx` identifier in scope to pass to a `trace!`
// invocation. Since doing that causes `poise` to stop recognizing the `Context`
// argument, I have to use this to convince the macro system that the identifier
// is actually in scope. This works by repeating each argument identifier twice
// and using the first occurrence as a sort of discriminator to alert the macro
// system that the `$ctx` macro arg is our desired identifier.
//
// The downside of this is that the context argument must be named `ctx` in the
// macro invocation, but I can live with that.
macro_rules! __trace_cmd {
    (ctx $ctx:ident, $cmd:expr) => {
        trace!(r#"{} executed command "{}""#, $ctx.author().name, $cmd);
    };
    ($($arg:tt)*) => {};
}

pub(crate) use __trace_cmd;
