# Locket
This is a project for my school's Computer Science project assessment, and is
currently mostly finished. It is a small password manager with a CLI and web interface.

## Build instructions

```shell
$ pnpm install
$ pnpx tailwindcss -i ./src/web/index.css -o ./dist/index.css
$ cargo run
```

## Web Interface Architecture 
The web interface has a rather strange architecture, which was stolen from
[Tsoding's seroost](https://github.com/tsoding/seroost/) (I'm using the same
http library and the server's event loop calls similarly named functions). I've
implemented a sort-of hot-reloading on top of `tiny-http`, where if we are in
debug-mode, any static files `index.html`, etc, are read from file every time
they are requested, but if we are in release-mode, then they are
`include!`ed into the final binary. This approach was also stolen from Tsoding,
but was stolen from his [musializer](https://github.com/tsoding/musializer)
project where he implemented hot-reloading for C.

# Roadmap
- [ ] TUI?
- [ ] Improved CLI deletion
- [ ] Saving the website URL
- [ ] OTP
- [x] Use URL query parameters instead of passing the query in the body of the GET request
- [ ] Web
  - [ ] Web interface
    - [x] Query logins
    - [x] Remove logins
    - [x] Add logins
  - [ ] API
    - [ ] Initialise a database via the API
    - [x] Add logins via the API
    - [x] Query logins via the API
    - [ ] Remove logins via the API
      - [x] Remove a single login
      - [ ] Remove multiple logins
