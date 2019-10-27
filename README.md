# tokio-terminal-resize

Implements a stream of terminal resize events.

## Overview

Whenever the user resizes their terminal, a notification is sent to the
application running in it. This crate provides those notifications in the
form of a stream.

## Synopsis

```rust
let stream = tokio_terminal_resize::resizes().flatten_stream();
let prog = stream
    .for_each(|(rows, cols)| {
        println!("terminal is now {}x{}", cols, rows);
        Ok(())
    })
    .map_err(|e| eprintln!("error: {}", e));
tokio::run(prog);
```
