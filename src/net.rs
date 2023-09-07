use std::io::{BufReader, Read};

use color_eyre::eyre::{Result, WrapErr};
use tiny_http::{Header, Request, Response, StatusCode};

use crate::models::Database;

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

fn serve_404(request: Request) {
    if let Err(e) = request.respond(Response::from_string("404").with_status_code(StatusCode(404)))
    {
        eprintln!("[|] WARN: Failed to respond to a request: {:#?}", e);
    }
}