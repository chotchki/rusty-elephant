# FeOphant

A SQL database server written in Rust and inspired by PostreSQL.

Just a toy for the moment, but I'm actively working to fix that!

[![Latest Build][build-badge]][build-url]
[![codecov][codecov-badge]][codecov-url]

[build-badge]: https://github.com/chotchki/feophant/actions/workflows/test_source_coverage.yaml/badge.svg
[build-url]: https://github.com/chotchki/feophant/actions/workflows/test_source_coverage.yaml
[codecov-badge]: https://codecov.io/gh/chotchki/feophant/branch/main/graph/badge.svg?token=6JV9391LY0
[codecov-url]: https://codecov.io/gh/chotchki/feophant

[Website](https://feophant.com)

## Launch

Launch the server
`./feophant`

Lauch a postgres client application to test
`./pgbench -h 127.0.0.1 -p 50000`
`./psql -h 127.0.0.1 -p 50000`

## What works user facing
You can currently start the server, connect to it and have it throw tons of errors. To support more there is a ton of infrastructure required to wire up next steps.

## Current TODO List - Subject to constant change!

**Path to 0.8**

Add support for defining a primary key on a table. This implies the following functionality:
* Index support through the stack down to the page level.
* The concept of unique indexes.
* Transactional support for indexes.
* Failure of a statement on constraint violation. Unsure if I'll end up with a general constraint system from this.

Based on reading this really means implementing Btree indexes. They don't seem to be that bad to understand/implement.

**Path to 0.9**

Implement where clauses, will likely need to have to start tracing columns from analyizing through to later stages.

**Path to 0.10**

Implement delete for tuples

**Path to 0.11**

pgbench setup can run successfully, in memory

**Path to 0.12**

Ensure data about table structures is thread safe in the face of excessive Arc usage.

**Path to 0.13**

Support a row with more than 4kb of text in it.

**Path to 0.14**

Implement sorting.

**Path to 0.15**

Implement column aliasing

**Path to 0.16**

Implement subselect.

**Path to 0.17**

Implement Updates.

**Path to 0.18**

Did some reading on how the buffer manager works and my implementation seems to be firmly in the right direction. Take that knowledge and implement persistence

**1.0 Release Criteria**

* pgbench can run successfully
* ~~Pick a new distinct name, rename everything~~ Done
* Pick a license
* Setup fuzz testing
* Persist to disk with moderate crash safety
* Be prepared to actually use it


### Longer Term TODO

This is stuff that I should get to but aren't vital to getting to a minimal viable product.
* Right now the main function runs the server from primitives. The Tokio Tower layer will probably do it better.
* The codec that parses the network traffic is pretty naive. You could make the server allocate 2GB of data for a DDOS easily.
* * We should either add state to the codec or change how it parses to produce chunked requests. That means that when the 2GB offer is reached the server can react and terminate before we accept too much data. Its a little more nuanced than that, 2GB input might be okay but we should make decisions based on users and roles.
* There is an extension that removes the need to lock tables to repack / vaccum. Figure out how it works!
* * https://github.com/reorg/pg_repack

## Postgres Divergance

Its kinda pointless to blindly reproduce what has already been done so I'm making the following changes to the db server design vs Postgres.

* Rust's memory safety and strong type system.
* Multi-threaded async design based on Tokio instead of Postgres's multi-process design.
* * Perk of this is not needing to manage SYSV shared memory. (Postgres largely fixed this but I think its still worth noting).
* Want to avoid vaccuum for transaction wrap around. Will try 64-bit transaction IDs but might go to 128-bit.
* * I can avoid the need to freeze Transaction IDs however the hint bits will need scanning to ensure that they are updated.
* Replacing OIDs with UUIDv4s.


### Rust Notes

How to setup modules sanely: https://dev.to/stevepryde/intro-to-rust-modules-3g8k

Reasonable application error type creation: https://github.com/dtolnay/anyhow

Library Errors: https://github.com/dtolnay/thiserror

Rust's inability to treat enum variants as a type is a HUGE pain. I cheated and separated serialization from deserialization.

## Legal Stuff (Note I'm not a lawyer!)

I am explicitly striving for SQL+Driver compatibility with [PostgreSQL](https://www.postgresql.org) so things such as system tables and code that handles them will be named the same. I don't think this violates their [trademark policy](https://www.postgresql.org/about/policies/trademarks/) but if I am please just reach out to me! I have also gone with a pretty restrictive license but I'm not tied to it if that is causing an issue for others who are using the code.