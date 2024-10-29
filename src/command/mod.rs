use std::sync::Arc;

use color_eyre::eyre::Report;
use poise::Command;

macro_rules! commands {
    ($($name:ident),* $(,)?) => {
        $(
            pub mod $name;
        )*

        pub fn commands() -> Vec<Command<Arc<crate::Data>, Report>> {
            vec![$( $name::$name() ),*]
        }
    };
}

commands! {
    create,
    create_for,
    remove,
    remove_for,
    random,
}
