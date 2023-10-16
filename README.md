# Gondolin

> Warning: this readme is overly verbose and will probably be refactored soon.
> Please ignore any pretentious sounding prose. 

This is a project for my school's Computer Science project assessment, and is
currently *not* finished. It is a small password manager usable not only through
the terminal, but also through the Web (which is provided as an optional -
although default - feature through Cargo).

## Build instructions
```shell
bun install
bunx tailwindcss -i ./src/web/index.css -o ./dist/index.css
cargo run
```

## On the the web interface
The web interface has a rather strange architecture, which was stolen from
[Tsoding's seroost](https://github.com/tsoding/seroost/) (I'm using the same
http library and the server's event loop calls similarly named functions). I've
implemented a sort-of hot-reloading on top of `tiny-http`, where if we are in
debug-mode, any static files `index.html`, etc, are read from file every time
they are requested, but if we are in release-mode, then they are
`include!`ed into the final binary. This approach was also stolen from Tsoding,
but was stolen from his [musializer](https://github.com/tsoding/musializer)
project where he implemented hot-reloading of a sort for C.

I chose to implement it this way because...I like Rust, and don't like JS. However,
I may rewrite the frontent with a proper web framework at some point :^).

## On this project's name
This project is named after the city of
[Gondolin](https://en.wikipedia.org/wiki/Gondolin)
from J.R.R. Tolkien's legendarium. If you are not familiar with the Lord
of the Rings, in short, Gondolin was a hidden city during the First Age
of Middle-Earth. Therefore, I thought the name was quite apt for this
project, as theoretically it's hiding your passwords. 

**Note**: If you represent the Tolkien Estate and I have comitted some kind
of copyright infringement by using this name, please contact me via my email address
listed on my [GitHub profile](https://github.com/needlesslygrim).