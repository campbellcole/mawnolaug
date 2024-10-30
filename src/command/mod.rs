use std::sync::Arc;

use color_eyre::eyre::Report;
use poise::Command;

use crate::config::AppConfig;

macro_rules! commands {
    ($($name:ident),* $(,)?) => {
        $(
            pub mod $name;
        )*

        pub fn commands(config: &AppConfig) -> Vec<Command<Arc<crate::Data>, Report>> {
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

macro_rules! command {
    (
        $is_admin:literal;
        $(#[$attr:meta])*
        pub async fn $name:ident($($arg:ident: $arg_ty:ty),*) -> Result<$ret_ty:ty> $body:block
    ) => {
        pub fn command(config: & $crate::config::AppConfig) -> ::poise::Command<::std::sync::Arc<$crate::Data>, ::color_eyre::eyre::Report> {
            let mut cmd = $name();

            if $is_admin {
                cmd.default_member_permissions = *config.admin_permissions;
            }

            cmd
        }

        $(#[$attr])*
        #[::poise::command(slash_command)]
        async fn $name($($arg: $arg_ty),*) -> ::color_eyre::eyre::Result<$ret_ty> $body
    };
}

pub(crate) use command;
