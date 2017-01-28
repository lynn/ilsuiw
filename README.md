# ilsuiw
ilsuiw listens to the following commands:

    !learn add topic fact          (Append a fact to the "topic" entry.)
    !learn add topic[2] fact       (Insert a fact in the "topic" entry.)
    !learn del topic[2]            (Remove a fact in the "topic" entry.)
    ??topic[3]                     (Query a fact.)
    ??topic                        (Equivalent to ??topic[1].)

Launch with the environment variable `DISCORD_TOKEN` set to your bot’s token, while running a Redis server on `redis://127.0.0.1`.
