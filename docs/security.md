# Security of the page's server

`flightdeck open <TOOL>` starts an HTTP server on 127.0.0.1 and hands its
token to one browser tab. This page states what the server defends against,
each guard it has, the test that proves each guard, and the limits it does
not cover.

## Threat model

The server defends against two attackers:

- **A hostile page in the same browser.** A page from any other site, open
  in another tab, can send requests to 127.0.0.1 and can point a name it
  controls at 127.0.0.1 (DNS rebinding). It cannot read the opener page's
  file or the page's `sessionStorage`.
- **Another user on the same machine.** Another user can connect to any port
  on 127.0.0.1 and send any request. They cannot read files that only the
  person who ran flightdeck can read.

A process that runs as the same user is out of scope. It can already run the
tool directly, and it can read the opener page or the browser's storage.

## How the browser gets the token

1. flightdeck creates a new directory with mode 0700 under the system's
   temporary directory and writes the opener page into it with mode 0600.
2. The opener page sends the browser to `http://127.0.0.1:<port>/#code=<code>`.
   The launch code is in the URL's fragment, which a browser never sends to
   a server, so the code reaches no request line and no log. flightdeck
   opens the page's file path with the system's opener, so the code is in no
   process's arguments either.
3. The page sends the code once to `POST /api/session` and receives the
   token. A code works once, and only within 60 seconds after flightdeck
   starts.
4. Every other API request carries `Authorization: Bearer <token>`.

The launch code and the token are each 32 bytes from the operating system's
random source, written as 64 hex digits. flightdeck holds them in `secrecy`
types, compares a presented value with them in constant time, and prints
neither of them.

## Guards

| Guard | What it stops | Test |
| --- | --- | --- |
| The server listens on 127.0.0.1 only, on a port the system picks. | Requests from other machines. | Every test in `tests/server.rs` connects to 127.0.0.1. |
| Every `/api/` path except `/api/session` needs the token. A missing, wrong or malformed `Authorization` header gets 401 with a problem document that says nothing about the token. | Use of the API by anyone who does not hold the token. | `a_request_without_a_token_gets_401_with_a_problem_document`, `a_wrong_token_of_the_right_length_gets_401` and `a_malformed_authorization_header_gets_401` in `tests/server.rs`. An unknown `/api/` path also needs the token before it gets 404: `an_unknown_path_gets_404_and_an_unknown_api_path_needs_the_token_first`. |
| The launch code works once and expires 60 seconds after it is issued. A wrong code leaves the real one valid. | Reuse of a code from the opener page, and a guess that locks the page out. | `the_same_launch_code_a_second_time_is_refused`, `a_launch_code_older_than_60_seconds_is_refused` and `a_wrong_launch_code_is_refused_and_leaves_the_right_one_valid` in `tests/server.rs`. |
| The opener page has mode 0600 in a directory with mode 0700, holds the code only in its fragment, and is removed when flightdeck stops. | Another user reading the code. | `the_opener_page_is_private_to_the_user_and_holds_the_code_only_in_its_fragment` in `tests/server.rs`, and `ctrl_c_stops_open_with_exit_0_and_removes_the_opener_directory` in `tests/open.rs`. |
| The `Host` header must be exactly `127.0.0.1:<port>`. | DNS rebinding, where a hostile page's own name resolves to 127.0.0.1. | `a_host_other_than_the_servers_own_address_is_refused` in `tests/server.rs`. |
| A request whose `Sec-Fetch-Site` is anything other than `same-origin` or `none` is refused, except a `GET` navigation to a path outside `/api/`. | Requests that a hostile page sends, including one from another port on 127.0.0.1, which the browser marks `same-site`. | `a_cross_site_or_same_site_request_is_refused` and `a_cross_site_navigation_to_the_page_is_served_because_the_opener_page_starts_one` in `tests/server.rs`. |
| An `Origin` header must be `http://127.0.0.1:<port>`, and every `POST` must have one. | A hostile page's request in a browser that sends no `Sec-Fetch-Site`. | `a_foreign_origin_is_refused_on_any_method` and `a_post_without_an_origin_is_refused` in `tests/server.rs`. |
| Every `POST` must declare `Content-Type: application/json`. | A cross-site form, and any request that a browser sends without a CORS preflight. | `a_post_that_is_not_json_is_refused` in `tests/server.rs`. |
| The server answers only `GET` and `POST`, refuses `OPTIONS`, and never sends a CORS header. | Every CORS preflight, so no other origin may read a reply or send JSON. | `an_options_request_is_refused_without_cors_headers` and `every_reply_carries_the_security_headers_and_no_cors_header` in `tests/server.rs`. |
| A request body over 64 KiB is refused, and a declared length over 64 KiB is refused from the head alone. The server reads a body only on the one route that takes one, after the request has passed every check above. A refused request's body is never read, apart from at most 64 KiB that the server reads and drops for half a second after the reply, so that the client receives the reply before the connection closes. | A request that fills the server's memory, or that declares a huge body to make the server allocate or wait for it. | `a_body_over_the_size_limit_is_refused` and `a_huge_declared_length_with_a_wrong_host_is_refused_and_the_server_still_answers` in `tests/server.rs`. |
| A request's line and headers together may be at most 16 KiB, with at most 32 headers. A larger head gets 431. A request that is not HTTP/1.x, or that has `Transfer-Encoding` or a `Content-Length` that is not one decimal number, gets 400. | A request whose headers fill the server's memory, and a body whose length the server and the client would read differently. | `a_request_head_over_the_size_limit_or_with_too_many_headers_is_refused` in `tests/server.rs`. |
| Each connection has 10 seconds to send its head and body, and each write of a reply may wait 10 seconds for the client to read. Each connection is answered on its own thread, and every reply carries `Connection: close`, so a connection holds one request. | A client that sends slowly, or never sends the body it declares, holding up the requests of the page. | `a_refused_request_whose_body_never_arrives_does_not_block_the_next_request` and `every_reply_closes_its_connection` in `tests/server.rs`. |
| The server holds at most 16 connections at once. A connection over the limit gets 503 and is closed. | A client that opens connections until the process runs out of threads or memory. | `a_connection_over_the_connection_limit_is_refused_and_the_others_are_still_answered` in `tests/server.rs`. |
| Every reply carries `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Cross-Origin-Resource-Policy: same-origin`, `Cross-Origin-Opener-Policy: same-origin` and a content security policy that allows only the server's own scripts, styles, fonts, images and connections, and no frames, forms or base URL. | A cached token reply, a reply read as another type, a leaked address, an embedding of the page or its files, and injected scripts. | `every_reply_carries_the_security_headers_and_no_cors_header` in `tests/server.rs`. |
| No line that flightdeck prints and no reply holds the token or the launch code. | A secret in a terminal, a log or a refusal. | `open_prints_two_lines_and_no_secret_and_stops_on_sigterm_with_exit_0` in `tests/open.rs`. |
| The server stops after 30 minutes without an authenticated request, and on Ctrl-C or SIGTERM. | A token that stays valid after the person has finished. | `the_server_stops_by_itself_after_the_idle_limit`, `an_authenticated_request_restarts_the_idle_limit` and `a_refused_request_does_not_restart_the_idle_limit` in `tests/server.rs`, and the signal tests in `tests/open.rs`. |

### Why a cross-site navigation to the page is allowed

The opener page is a file, and a browser marks a navigation from a file to
an `http` address as `cross-site`. Refusing that navigation would refuse the
first request of every session. The server therefore allows a cross-site
`GET` whose `Sec-Fetch-Mode` is `navigate` to a path outside `/api/`. Such a
request reaches only the embedded page, which holds no secret, and the
content security policy's `frame-ancestors 'none'` stops another site from
framing it. A navigation to an `/api/` path is refused.

## Known limits

- **Another user can hold the server's connections.** A local process
  that opens 16 connections and sends nothing holds every connection for up
  to 10 seconds, and opening more keeps the page from reaching the server
  for as long as it continues. It cannot read or change anything, and it
  stops nothing that the page has started. That process is the same user,
  who is out of scope, or another user who knows the port.
- **A refusal for a connection over the limit is written by the accept
  thread.** While it writes the 503 reply and waits up to half a second for
  the client to close, the server accepts no other connection.
- **The placeholder page does not exchange the code.** The page that the
  server embeds says that it is not built yet, so the launch code stays in
  the address bar until it expires.
