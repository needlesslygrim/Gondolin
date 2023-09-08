use std::io::{BufReader, Read};

use color_eyre::eyre::{Result, WrapErr};
use tiny_http::{Header, Request, Response, StatusCode};

use crate::models::{Database, Login};

pub fn serve(db: &mut Database) -> Result<()> {
    let ip = "127.0.0.1:56423";
    let server = tiny_http::Server::http(ip)
        .map_err(|e| color_eyre::eyre::eyre!(e))
        .wrap_err_with(|| format!("Failed to start server at {ip}"))?;

    eprintln!("[+] INFO: Serving webpage at {ip}");

    for request in server.incoming_requests() {
        use tiny_http::Method as M;
        match (request.method(), request.url()) {
            (M::Get, "/") => serve_bytes(
                request,
                &include_bytes!("index.html")[..],
                "text/html; charset=utf8",
            ),
            (M::Get, "/index.js") => serve_bytes(
                request,
                &include_bytes!("index.js")[..],
                "text/javascript; charset=utf8",
            ),
            (M::Get, "/index.css") => serve_bytes(
                request,
                &include_bytes!("../dist/output.css")[..],
                "text/css; charset=utf8",
            ),
            (M::Get, "/api/query") => serve_query(request, db),
            (M::Post, "/api/new") => add_new(request, db),
            _ => serve_404(request),
        }
    }
    Ok(())
}

fn serve_bytes(request: Request, content: &[u8], content_type: &str) {
    let content_type_header = Header::from_bytes("Content-Type", content_type)
        .expect("Please don't put rubbish inside `content_type`");
    let response = Response::from_data(content).with_header(content_type_header);

    if let Err(e) = request.respond(response) {
        eprintln!("[|] WARN: Failed to respond to a request: {:#?}", e);
    }
}

fn serve_query(mut request: Request, db: &Database) {
    let body_length = request.body_length();
    let reader = request.as_reader();
    let mut buf_reader = BufReader::new(reader);
    let mut content = String::with_capacity(body_length.map(|len| len / 8).unwrap_or(0));

    if let Err(e) = buf_reader.read_to_string(&mut content) {
        eprintln!("[-] WARN: Failed to read content body of a request: {e}");
        if let Err(e) =
            request.respond(Response::from_string("500").with_status_code(StatusCode(500)))
        {
            eprintln!("[|] WARN: Failed to respond to a request: {:#?}", e);
        }
        return;
    }

    let matches = db.query(&content);
    let body = serde_json::ser::to_string(&matches);

    if let Err(e) = body {
        eprintln!("[-] WARN: Failed to serialise query matches into JSON: {e}");
        if let Err(e) =
            request.respond(Response::from_string("500").with_status_code(StatusCode(500)))
        {
            eprintln!("[|] WARN: Failed to respond to a request: {:#?}", e);
        }

        return;
    }

    // *Should* be fine.
    let body = unsafe { body.unwrap_unchecked() };
    let header = Header::from_bytes("Content-Type", "application/json")
        .expect("Don't put rubbish in here please");
    let response = Response::from_string(body).with_header(header);

    if let Err(e) = request.respond(response) {
        eprintln!("[|] WARN: Failed to respond to a request: {:#?}", e);
    };
}

fn add_new(mut request: Request, db: &mut Database) {
    let body_length = request.body_length().map(|length| length / 8).unwrap_or(0);
    let mut buf: Vec<u8> = Vec::with_capacity(body_length);
    let maybe_content_type = request
        .headers()
        .iter()
        .find(|header| header.field.as_str() == "Content-Type");
    let content_type_header = if maybe_content_type.is_none() {
        eprintln!("[|] WARN: A request was made to `/api/new` without a `Content-Type` header");
        if let Err(e) = request.respond(make_415()) {
            eprintln!("[|] WARN: Failed to respond to a request: {:#?}", e);
            return;
        }
        return;
    } else {
        // Should be fine :^)
        unsafe { maybe_content_type.unwrap_unchecked() }
    };

    if content_type_header.value != "application/json" {
        eprintln!("[|] WARN: A request was made to `/api/new` without a valid `Content-Type` of `application/json`");
        if let Err(e) = request.respond(make_415()) {
            eprintln!("[|] WARN: Failed to respond to a request: {:#?}", e);
            return;
        }
        return;
    }

    if let Err(e) = request.as_reader().read_to_end(&mut buf) {
        eprintln!("[|] WARN: Could not read the body of the request: {:#?}", e);
        if let Err(e) = request.respond(make_415()) {
            eprintln!("[|] WARN: Failed to respond to a request: {:#?}", e);
            return;
        }
        return;
    }

    let content = match String::from_utf8(buf) {
        Ok(content) => content,
        Err(e) => {
            eprintln!(
                "[|] WARN: The body of a request could not be interpreted as UTF-8: {:#?}",
                e
            );
            return;
        }
    };

    let logins: Result<Vec<Login>, _> = serde_json::de::from_str(&content);
    let mut logins = if let Err(e) = logins {
        eprintln!("[-] WARN: Failed to parse login from request: {e}");
        if let Err(e) = request.respond(make_415()) {
            eprintln!("[|] WARN: Failed to respond to a request: {:#?}", e);
            return;
        }
        return;
    } else {
        // Should be fine :).
        unsafe { logins.unwrap_unchecked() }
    };

    db.logins.append(&mut logins);
    if let Err(e) = request.respond(
        Response::from_string(StatusCode(200).default_reason_phrase())
            .with_status_code(StatusCode(200)),
    ) {
        eprintln!("[|] WARN: Failed to respond to a request: {:#?}", e);
    };
}

#[inline(always)]
fn make_415() -> Response<std::io::Cursor<std::vec::Vec<u8>>> {
    Response::from_string(StatusCode(415).default_reason_phrase()).with_status_code(StatusCode(415))
}

fn serve_404(request: Request) {
    if let Err(e) = request.respond(Response::from_string("404").with_status_code(StatusCode(404)))
    {
        eprintln!("[|] WARN: Failed to respond to a request: {:#?}", e);
    }
}
