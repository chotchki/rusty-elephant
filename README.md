Welcome to Rusty Elephant! Chris's attempt to learn rust. As a toy I'm implementing a SQL database that can accept pgbench input.

[![Test with Source Code Coverage](https://github.com/chotchki/rusty-elephant/actions/workflows/test_source_coverage.yaml/badge.svg)](https://github.com/chotchki/rusty-elephant/actions/workflows/test_source_coverage.yaml) [![codecov](https://codecov.io/gh/chotchki/rusty-elephant/branch/main/graph/badge.svg?token=6JV9391LY0)](https://codecov.io/gh/chotchki/rusty-elephant)

Just a toy but it's already taught me a lot about Rust.

# Launch
Launch the server
`./rusty-elephant`

Lauch a postgres client application to test
`./pgbench -h 127.0.0.1 -p 50000`


# What works
You can currently start the server, connect to it and have it throw tons of errors. I'm to the point now I need to start supporting saving data.

# Next TODO
Path to 0.4: Need to support the concept of a table that can be read and written to, in memory.
    sql statement: create table foo;
    sql statement: drop table foo;

Have a generic way to store rows in a table. Need to figure out basic transactions so I can implement delete/update. With that I can implement the create and drop statements.

Did some reading on an amazing book: (http://www.interdb.jp/pg/pgsql05.html), transactions should be implemented as a combination of a range of transactions plus a visability map

# # Longer Term TODO
This is stuff that I should get to but aren't vital to getting to a minimal viable product.
* Right now the main function runs the server from primitives. The Tokio Tower layer will probably do it better.
* The codec that parses the network traffic is pretty naive. You could make the server allocate 2GB of data for a DDOS easily.
* * We should either add state to the codec or change how it parses to produce chunked requests. That means that when the 2GB offer is reached the server can react and terminate before we accept too much data. Its a little more nuanced than that, 2GB input might be okay but we should make decisions based on users and roles.

# Postgres Divergance
Its kinda pointless to blindly reproduce what has already been done so I'm making the following changes to the db server design vs Postgres.

* Multi-threaded design based on Tokio instead of Postgres's multi-process design.
* * Perk of this is not needing to manage SYSV shared memory. (Postgres largely fixed this but I think its still worth noting).
* Want to avoid vaccuum for transaction wrap around. Will try 64-bit transaction IDs but might go to 128-bit.
* Replacing OIDs with UUIDv4s.

## Rust Notes
How to setup modules sanely: https://dev.to/stevepryde/intro-to-rust-modules-3g8k

Reasonable application error type creation: https://github.com/dtolnay/anyhow

Library Errors: https://github.com/dtolnay/thiserror

Rust's inability to treat enum variants as a type is a HUGE pain. I cheated and separated serialization from deserialization.

# Legal Stuff (Note I'm not a lawyer!)
I am explicitly striving for SQL+Driver compatibility with [PostgreSQL](https://www.postgresql.org) so things such as system tables and code that handles them will be named the same. I don't think this violates their [trademark policy](https://www.postgresql.org/about/policies/trademarks/) but if I am please just reach out to me! I have also gone with a pretty restrictive license but I'm not tied to it if that is causing an issue for others who are using the code.