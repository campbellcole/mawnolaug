# mawnolaug

<img src="./mawnolaug.png" width="200px" alt="mawnolaug profile picture" />

A Discord bot that creates, manages, and assorts monologue channels. The intention is to give guild users a place to rant, post cool things, or just announce their accomplishments.

The bot can be configured to take a random monologue entry and post it to a chosen channel however often you like.

## Usage

The bot is controlled using slash-commands:

- `/create`: Create a monologue channel for yourself
- `/create_for <user>`: Create a monologue channel for `<user>` \*
- `/remove`: Remove your monologue channel
- `/remove_for <user>`: Remove the monologue channel for `<user>` \*
- `/random`: Draw a random monologue from any time
- `/trigger`: Trigger a new random monologue draw. This only pulls messages sent since the last invocation or scheduled trigger \*

\* admin only command

## Configuration

mawnolaug can be configured via a TOML file or environment variables, or both. Environment variables always take precedence.

This file is expected at `mawnolaug.toml` in the working directory unless a location is specified using the `MAWNO_CONFIG` environment variable.

### Available options

```toml
token = "your discord token"
# see "Admin Commands" for an explanation. defaults to "8"
admin_permissions = "8"
# the location of the bot state
state_dir = "/path/to/state_directory"
# the timezone to use as local time instead of the OS defined timezone (optional).
# this is useful if the bot is running in a docker container or distant server.
# this is used for the `{timestamp}` specifier and the random draw schedule
timezone = "America/Los_Angeles"

[monologues]
# the Category within which monologue Channels will be created (optional)
# setting this is highly recommended to avoid littering your server
category_id = 1234567890123456
# set to true to allow anyone to post in any monologue channel (false by default)
allow_anyone = false

# this section is optional. if not defined, random draws will be disabled.
[random_draw]
# this is the Channel that random monologues will be posted to
channel_id = 1234567890123456
# a cron expression representing a schedule for random draws see 
# https://crontab.guru/ for help with these expressions
# note: the schedule parser requires 6 or 7 segments while crontab only shows 5:
# sec min hour day_of_month month day_of_week year (year optional)
schedule = "0 0 10,16,22 * * * *" # do random draws at 10am, 4pm, and 10pm local time
# a set of messages to prefix the random draw with. a random message will be chosen
# each time a new random draw occurs. see the "Message Templates" section for
# information on the template syntax
messages = [
  "Look what {author} found:",
  "At {timestamp:%I:%M %p}, {author} said:",
]
```

The following environment variables are equivalent to the above config:

```sh
MAWNO_TOKEN="your discord token"
MAWNO_ADMIN_PERMISSIONS="0"
MAWNO_STATE_DIR="/path/to/state_directory"

MAWNO_RANDOM_DRAW_CHANNEL_ID="1234567890123456"
MAWNO_RANDOM_DRAW_INTERVAL="720"

MAWNO_MONOLOGUES_CATEGORY_ID="1234567890123456"
```

mawnolaug supports reading environment variables from a `.env` file in the current directory.

### Admin Commands

By default, commands marked as "admin only" can only be triggered by a user with the Administrator permission. Handing this permission out is generally a bad idea, so you can choose which member permissions are required to trigger the admin only commands.

Before trying to decide on a value for this, you might read the [Permissions documentation](https://discord.com/developers/docs/topics/permissions) to get a better understanding of how this value works. The permissions bitfield is stored as a string as-per the above documentation.

To configure the permissions required, simply set the `admin_permissions` option to your chosen permissions integer. A few examples are:

```toml
# 1 << 4 (0x10)
admin_permissions = "16" # require the Manage Channels permission

# or

# 1 << 5 (0x20)
admin_permissions = "32" # require the Manage Guild permission
```

**Note:** This option only configures the default permission required for the admin commands. You can still manage the permissions of each command in your server's settings page.

### Message Templates

The `messages` array in the config supports a few simple templates:

- `{author}`: @mention the message author
- `{author.name}`: The author's display name
- `{author.id}`: The user ID of the author
- `{channel}`: #mention the author's monologue channel
- `{channel.id}`: The channel ID of the author's monologue channel
- `{timestamp:<strftime format>}`: The timestamp of the message with the specified format. See [`chrono::format::strftime`](https://docs.rs/chrono/0.4.38/chrono/format/strftime/index.html) for information on format specifiers

There is currently no `{channel.name}` because that would require an additional API call.
