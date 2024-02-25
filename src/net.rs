use std::{
    fs,
    hint::unreachable_unchecked,
    io::ErrorKind,
    path::Path,
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
};

use color_eyre::eyre::{bail, Result, WrapErr};
use log::{debug, error, info, warn};
use signal_hook::consts::SIGINT;
use tiny_http::{Header, Request, Response, StatusCode};
use url::Url;
use uuid::Uuid;

use crate::models::{Database, Login};

pub fn serve(db: &mut Database, port: u16, lck_path: &Path) -> Result<()> {
    let should_shutdown = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, Arc::clone(&should_shutdown))
        .wrap_err("Failed to register the shutdown bool")?;
    let ip = format!("127.0.0.1:{port}");
    let server = tiny_http::Server::http(&ip)
        .map_err(|e| color_eyre::eyre::eyre!(e))
        .wrap_err_with(|| format!("Failed to start server at {ip}"))?;

    info!("Serving webpage at {ip}");
    for request in server.incoming_requests() {
        use tiny_http::Method as M;
        let url = match Url::from_str(&format!("https://{ip}"))
            .expect("Please don't put any rubbish in this url")
            .join(request.url())
        {
            Ok(url) => url,
            Err(e) => {
                error!(
                    "Failed to parse a url: `{}`, with err: {}",
                    request.url(),
                    e
                );
                std::process::exit(1)
            }
        };
        // TODO: Go through all of these functions, and check that they follow the proper behaviour, returning correct status codes, etc.
        match (request.method(), url.path()) {
            (
                M::Get,
                "/" | "/new" | "/index.css" | "/query.js" | "/query.js.map" | "/form.js"
                | "/form.js.map",
            ) => serve_static(request),
            (M::Get, "/query") => serve_query_page(
                request,
                url.query_pairs()
                    .find(|query| &query.0 == "query")
                    .map(|query| query.1)
                    .as_deref(),
                db,
            ),
            (M::Get, "/api/v1/query") => serve_query(
                request,
                url.query_pairs()
                    .find(|query| &query.0 == "query")
                    .map(|query| query.1)
                    .as_deref(),
                db,
            ),
            (M::Get, "/api/v1/sync") => {
                db.sync()
                    .wrap_err("Failed to sync database after it was requested via API")?;
                let Err(err) = request.respond(
                    Response::from_string(StatusCode(204).default_reason_phrase())
                        .with_status_code(204),
                ) else {
                    continue;
                };

                warn!("Failed to respond to a request: {err:#?}");
            }
            (M::Post, "/api/v1/new") => add_new(request, db),
            (M::Delete, "/api/v1/remove") => remove_login(
                request,
                url.query_pairs()
                    .find(|query| &query.0 == "id")
                    .map(|query| query.1)
                    .as_deref(),
                db,
            ),
            _ => {
                info!("404 served: {}", url.path());
                serve_404(request);
            }
        }

        if should_shutdown.load(Ordering::Relaxed) {
            db.sync().wrap_err("Failed to sync database to disk")?;
            if let Err(err) = fs::remove_file(lck_path) {
                match err.kind() {
                    ErrorKind::NotFound => {
                        eprintln!("Tried to remove the lockfile, but it wasn't present");
                        std::process::exit(1);
                    }
                    _ => bail!("Failed to remove the lockfile: {}", err),
                }
            };
        }
    }
    Ok(())
}

// In debug mode, we can do a sort of "hot-reloading", by just reopening the same files
// over and over again. Therefore, we can use `unwrap()`, as in my opinion, if someone
// is editing this project's code, and doesn't have these files in the right places, it's
// their fault, and it's my project so I can do what I like :^).
#[cfg(debug_assertions)]
fn serve_static(request: Request) {
    match request.url() {
        "/" => serve_bytes(
            request,
            &fs::read("src/web/index.html").expect("Failed to open index.html")[..],
            "text/html; charset=utf8",
        ),
        "/new" => serve_bytes(
            request,
            &fs::read("src/web/form.html").expect("Failed to open form.html")[..],
            "text/html; charset=utf8",
        ),
        "/index.css" => serve_bytes(
            request,
            &fs::read("dist/index.css").expect("Failed to open index.css")[..],
            "text/css; charset=utf8",
        ),
        "/query.js" => serve_bytes(
            request,
            &fs::read("dist/query.js").expect("Failed to open query.js")[..],
            "application/javascript; charset=utf8",
        ),
        "/query.js.map" => serve_bytes(
            request,
            &fs::read("dist/query.js.map").expect("Failed to open query.js.map")[..],
            "application/javascript; charset=utf8",
        ),
        "/form.js" => serve_bytes(
            request,
            &fs::read("dist/form.js").expect("Failed to open form.js")[..],
            "application/javascript; charset=utf8",
        ),
        "/form.js.map" => serve_bytes(
            request,
            &fs::read("dist/form.js.map").expect("Failed to open form.js.map")[..],
            "application/javascript; charset=utf8",
        ),
        _ => unsafe { unreachable_unchecked() },
    };
}

// Release mode version of the previous function. Here, it uses `include_bytes!()` to
// pack the content of the files into the binary.
#[cfg(not(debug_assertions))]
fn serve_static(request: Request) {
    match request.url() {
        "/" => serve_bytes(
            request,
            &include_bytes!("web/index.html")[..],
            "text/html; charset=utf8",
        ),
        "/new" => serve_bytes(
            request,
            &include_bytes!("web/form.html")[..],
            "text/html; charset=utf8",
        ),
        "/index.css" => serve_bytes(
            request,
            &include_bytes!("../dist/index.css")[..],
            "text/css; charset=utf8",
        ),
        "/query.js" => serve_bytes(
            request,
            &include_bytes!("../dist/query.js")[..],
            "application/javascript; charset=utf8",
        ),
        "/query.js.map" => serve_bytes(
            request,
            &include_bytes!("../dist/query.js.map")[..],
            "application/javascript; charset=utf8",
        ),
        "/form.js" => serve_bytes(
            request,
            &include_bytes!("../dist/form.js")[..],
            "application/javascript; charset=utf8",
        ),
        "/form.js.map" => serve_bytes(
            request,
            &include_bytes!("../dist/form.js.map")[..],
            "application/javascript; charset=utf8",
        ),
        _ => unsafe { unreachable_unchecked() },
    };
}

fn serve_bytes(request: Request, content: &[u8], content_type: &str) {
    let content_type_header = Header::from_bytes("Content-Type", content_type)
        .expect("Please don't put rubbish inside `content_type`");
    let response = Response::from_data(content).with_header(content_type_header);

    if let Err(e) = request.respond(response) {
        warn!("Failed to respond to a request: {e:#?}");
    }
}

// We should probably allow multiple mime types to be put in the response, by looking at the `Accept` header.
// However, for now there's probably not much point since we're the only ones consuming this API. Therefore
// we just ignore all headers, and send back `application/json`.
// TODO: Maybe look at checking the header to at least see if JSON was requested, and if not return 415 with `Accept-Post` set.
fn serve_query(request: Request, query: Option<&str>, db: &Database) {
    let matches = db.query(query);
    let body = serde_json::ser::to_string(&matches);

    if let Err(e) = body {
        warn!("Failed to serialise query matches into JSON: {e}");
        if let Err(e) = request.respond(
            Response::from_string(StatusCode(500).default_reason_phrase()).with_status_code(500),
        ) {
            warn!("Failed to respond to a request: {e:#?}");
        }

        return;
    }

    // *Should* be fine.
    let body = unsafe { body.unwrap_unchecked() };
    let header = Header::from_bytes("Content-Type", "application/json")
        .expect("Don't put rubbish in here please");
    let response = Response::from_string(body)
        .with_header(header)
        .with_status_code(200);

    if let Err(e) = request.respond(response) {
        warn!("Failed to respond to a request: {e:#?}");
    };
}

// This function currently doesn't support the "hot-reloading" that the other static files do. This
// is due to not using a proper templating library, and instead just formatting the text.
fn serve_query_page(request: Request, query: Option<&str>, db: &Database) {
    let logins = db.query(query);

    let mut grids = String::new();
    for login in logins {
        let card = format!(
            include_str!("web/card.html"),
            name = login.1.name,
            username = login.1.username,
            password = login.1.password,
            id = login.0.simple()
        );
        grids.push_str(&card);
    }

    let header =
        Header::from_bytes("Content-Type", "text/html").expect("Don't put rubbish in here please");
    let response = Response::from_string(format!(include_str!("web/query.html"), grid = grids))
        .with_header(header)
        .with_status_code(200);

    if let Err(e) = request.respond(response) {
        warn!("Failed to respond to a request: {e:#?}");
    };
}

fn add_new(mut request: Request, db: &mut Database) {
    let body_length = request.body_length().unwrap_or(0);
    let mut buf: Vec<u8> = Vec::with_capacity(body_length);
    let Some(content_type_header) = request
        .headers()
        .iter()
        .find(|header| header.field.as_str() == "Content-Type")
    else {
        debug!("A request was made to `/api/v1/new` without a `Content-Type` header");
        let response =
            Response::from_string(StatusCode(415).default_reason_phrase()).with_status_code(415);
        if let Err(e) = request.respond(response) {
            warn!("Failed to respond to a request: {e:#?}");
        }
        return;
    };

    if content_type_header.value != "application/json" {
        debug!("A request was made to `/api/v1/new` without a valid `Content-Type` of `application/json`");
        let response =
            Response::from_string(StatusCode(415).default_reason_phrase()).with_status_code(415);
        if let Err(e) = request.respond(response) {
            warn!("Failed to respond to a request: {e:#?}");
        }
        return;
    }

    if let Err(e) = request.as_reader().read_to_end(&mut buf) {
        info!("Could not read the body of the request: {e:#?}");
        let response =
            Response::from_string(StatusCode(415).default_reason_phrase()).with_status_code(415);
        if let Err(e) = request.respond(response) {
            warn!("Failed to respond to a request: {e:#?}");
        }
        return;
    }

    let content = match String::from_utf8(buf) {
        Ok(content) => content,
        Err(e) => {
            debug!("The body of a request could not be interpreted as UTF-8: {e:#?}");
            return;
        }
    };

    let logins = match serde_json::de::from_str::<Vec<Login>>(&content) {
        Ok(logins) => logins,
        Err(e) => {
            info!("Failed to parse login from request: {e}");
            let response = Response::from_string(StatusCode(415).default_reason_phrase())
                .with_status_code(415);
            if let Err(e) = request.respond(response) {
                warn!("Failed to respond to a request: {e:#?}");
            }
            return;
        }
    };

    db.append_logins(logins);
    if let Err(e) = request.respond(
        Response::from_string(StatusCode(201).default_reason_phrase()).with_status_code(201),
    ) {
        warn!("Failed to respond to a request: {e:#?}");
    };
}

// Now idempotent. Returns 204 on successful deletion, and 404 otherwise. Due to idempotency, a request can be sent multiple times by the client
// legally. Only the first successful deletion will return 204, other would-be-successful requests get a 404. This is OK according to
// https://stackoverflow.com/questions/24713945/does-idempotency-include-response-codes.8
fn remove_login(request: Request, id: Option<&str>, db: &mut Database) {
    let Some(id) = id else {
        debug!("A DELETE request contained no ID");
        // I assume that this should be a 404, looking at https://www.rfc-editor.org/rfc/rfc9110.html#name-client-error-4xx a 404 seems to be most accurate.
        let response =
            Response::from_string(StatusCode(404).default_reason_phrase()).with_status_code(404);

        if let Err(e) = request.respond(response) {
            warn!("Failed to respond to a request: {e:#?}");
        }
        return;
    };

    let id = match Uuid::parse_str(id) {
        Ok(id) => id,
        Err(e) => {
            debug!("A DELETE request contained an invalid ID: {}", e);
            let response = Response::from_string(StatusCode(404).default_reason_phrase())
                .with_status_code(404);
            if let Err(e) = request.respond(response) {
                warn!("Failed to respond to a request: {e:#?}");
            }
            return;
        }
    };

    if db.remove(id).is_none() {
        let response =
            Response::from_string(StatusCode(404).default_reason_phrase()).with_status_code(404);
        if let Err(e) = request.respond(response) {
            warn!("Failed to respond to a request: {e:#?}");
        }
        return;
    }

    if let Err(e) = request.respond(
        Response::from_string(StatusCode(204).default_reason_phrase()).with_status_code(204),
    ) {
        warn!("Failed to respond to a request: {e:#?}");
    };
}

fn serve_404(request: Request) {
    if let Err(e) = request.respond(Response::from_string("404").with_status_code(404)) {
        warn!("Failed to respond to a request: {e:#?}");
    }
}
