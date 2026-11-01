# Security of the page's server

`flightdeck open <TOOL>` starts an HTTP server on 127.0.0.1 and hands its
token to one browser tab. The page runs the tool's commands through the
server. This page states what the server defends against, each guard it
has, the test that proves each guard, the shapes of the run routes, and the
limits it does not cover.

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
| The launch code works once and expires 60 seconds after it is issued. A wrong code leaves the real one valid. The 60 seconds end when either the monotonic clock or the wall clock says they have passed, because on macOS the monotonic clock stops while the machine sleeps. | Reuse of a code from the opener page, and a guess that locks the page out. | `the_same_launch_code_a_second_time_is_refused`, `a_launch_code_older_than_60_seconds_is_refused`, `a_launch_code_is_refused_once_the_wall_clock_passes_60_seconds_while_the_monotonic_clock_stood_still` and `a_wrong_launch_code_is_refused_and_leaves_the_right_one_valid` in `tests/server.rs`. |
| The opener page has mode 0600 in a directory with mode 0700, holds the code only in its fragment, and is removed when flightdeck stops. | Another user reading the code. | `the_opener_page_is_private_to_the_user_and_holds_the_code_only_in_its_fragment` in `tests/server.rs`, and `ctrl_c_stops_open_with_exit_0_and_removes_the_opener_directory` in `tests/open.rs`. |
| The `Host` header must be exactly `127.0.0.1:<port>`. | DNS rebinding, where a hostile page's own name resolves to 127.0.0.1. | `a_host_other_than_the_servers_own_address_is_refused` in `tests/server.rs`. |
| A request whose `Sec-Fetch-Site` is anything other than `same-origin` or `none` is refused, except a `GET` navigation to a path outside `/api/`. | Requests that a hostile page sends, including one from another port on 127.0.0.1, which the browser marks `same-site`. | `a_cross_site_or_same_site_request_is_refused` and `a_cross_site_navigation_to_the_page_is_served_because_the_opener_page_starts_one` in `tests/server.rs`. |
| An `Origin` header must be `http://127.0.0.1:<port>`, and every `POST` must have one. | A hostile page's request in a browser that sends no `Sec-Fetch-Site`. | `a_foreign_origin_is_refused_on_any_method` and `a_post_without_an_origin_is_refused` in `tests/server.rs`. |
| Every `POST` must declare `Content-Type: application/json`. | A cross-site form, and any request that a browser sends without a CORS preflight. | `a_post_that_is_not_json_is_refused` in `tests/server.rs`. |
| The server answers only `GET` and `POST`, refuses `OPTIONS`, and never sends a CORS header. | Every CORS preflight, so no other origin may read a reply or send JSON. | `an_options_request_is_refused_without_cors_headers` and `every_reply_carries_the_security_headers_and_no_cors_header` in `tests/server.rs`. |
| A request body over 64 KiB is refused, and a declared length over 64 KiB is refused from the head alone. The server reads a body only on a route that takes one, after the request has passed every check above, including the token. A refused request's body is never read, apart from at most 64 KiB that the server reads and drops for half a second after the reply, so that the client receives the reply before the connection closes. | A request that fills the server's memory, or that declares a huge body to make the server allocate or wait for it. | `a_body_over_the_size_limit_is_refused` and `a_huge_declared_length_with_a_wrong_host_is_refused_and_the_server_still_answers` in `tests/server.rs`. |
| A request's line and headers together may be at most 16 KiB, with at most 32 headers. A larger head gets 431. A request that is not HTTP/1.x, or that has `Transfer-Encoding` or a `Content-Length` that is not one decimal number, gets 400. | A request whose headers fill the server's memory, and a body whose length the server and the client would read differently. | `a_request_head_over_the_size_limit_or_with_too_many_headers_is_refused` in `tests/server.rs`. |
| Each connection has 10 seconds to send its head and body. A body that does not arrive whole in that time gets 408 with `flightdeck::request::timeout`. The server searches for the end of a head only in the bytes that each read adds, and parses the head once. Each connection is answered on its own thread, and every reply carries `Connection: close`, so a connection holds one request. | A client that sends slowly, or never sends the body it declares, holding up the requests of the page, and a head sent a byte at a time that makes the server parse it again on every read. | `a_refused_request_whose_body_never_arrives_does_not_block_the_next_request`, `a_session_request_whose_body_never_arrives_gets_408` and `every_reply_closes_its_connection` in `tests/server.rs`, and the parser tests in `src/server/http/tests.rs`. |
| A reply of known length, its head and body together, must be read by the client within 10 seconds in all. Each line of a stream must be read within 10 seconds of when its write starts. A write that passes its deadline ends the reply and closes the connection, which cancels a stream's run. | A client that stops reading and holds a connection and its thread open. | `a_client_that_disconnects_stops_its_run` in `tests/routes.rs`. |
| The server holds at most 16 connections at once. A connection over the limit gets 503 and is closed. When accepting fails because the process or the system is out of file descriptors, buffers or memory, or because a client aborted its connection, the server waits one poll interval and accepts again. | A client that opens connections until the process runs out of threads or memory, and a burst of connections that stops the server. | `a_connection_over_the_connection_limit_is_refused_and_the_others_are_still_answered` in `tests/server.rs`, and the accept error tests in `src/server/tests.rs`. |
| The runner runs at most 8 commands at once, half the server's 16 connections, and the code asserts that bound when it compiles. Each stream holds one connection, so at least 8 stay free for the page's other requests, including a cancel. A cancel of a run that is not running gets 404 with `flightdeck::run::not_running`. | A page whose streams fill every connection, so that it can no longer cancel them. | `cancel_ends_a_stream_with_exit_130_and_keeps_the_rows_that_arrived` in `tests/routes.rs`. |
| The run routes need the token, check the command and every value against the tool's model before anything runs, and refuse with 422 and a `flightdeck/failure/v1` document that names the path as a list of words and the argument at fault. A body that is not a run request gets 400. A value that starts with `-` reaches the tool as a value after `--`. | A request that runs a command the tool hides, passes an argument the tool does not declare, or smuggles a flag into a value. | `a_hidden_command_is_refused_with_422_and_the_path_as_words`, `an_unknown_argument_is_refused_with_its_name_for_the_page_to_mark`, `a_body_that_is_not_a_run_request_is_refused_with_400` and `a_value_that_starts_with_a_dash_reaches_the_tool_as_the_options_value` in `tests/routes.rs`. |
| A destructive command runs only with a confirmation id that a preview of the same command line issued. An id works once, for that command line alone. With `--read-only`, the server refuses a command that is not read-only at preview and with an id. | A page that runs a destructive command without showing its preview, or reuses one preview for another row. | `books_remove_without_a_confirmation_id_is_refused`, `a_preview_streams_its_dry_run_and_ends_with_the_confirmation_id`, `a_confirmation_id_for_books_remove_2_does_not_run_books_remove_3`, `a_confirmation_id_runs_its_command_once`, `read_only_refuses_books_remove_at_preview_and_with_an_id` and `a_preview_of_a_command_without_dry_run_is_one_started_line_with_the_id` in `tests/routes.rs`. |
| A stream writes `{"event":"alive"}` after each 15 seconds without output. When a write fails because the client has gone, the server drops the run, which stops the child's process group. | A run that goes on after the page has closed, including a silent one that never writes the line whose failure would show that the client left. | `a_silent_run_streams_alive_lines_until_its_output` and `a_client_that_disconnects_stops_its_run` in `tests/routes.rs`. |
| Every reply carries `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Cross-Origin-Resource-Policy: same-origin`, `Cross-Origin-Opener-Policy: same-origin` and a content security policy that allows only the server's own scripts, styles, fonts, images and connections, and no frames, forms or base URL. | A cached token reply, a reply read as another type, a leaked address, an embedding of the page or its files, and injected scripts. | `every_reply_carries_the_security_headers_and_no_cors_header` in `tests/server.rs`. |
| No line that flightdeck prints and no reply holds the token or the launch code. | A secret in a terminal, a log or a refusal. | `open_prints_two_lines_and_no_secret_and_stops_on_sigterm_with_exit_0` in `tests/open.rs`. |
| The server stops after 30 minutes without an authenticated request, and on Ctrl-C or SIGTERM. The 30 minutes end by either clock, as the launch code's 60 seconds do. A token that arrives after the limit has passed, before the server has noticed, is refused, and the server stops at once. | A token that stays valid after the person has finished, including across a night of sleep. | `the_server_stops_by_itself_after_the_idle_limit`, `the_server_stops_after_the_idle_limit_by_the_wall_clock_while_the_monotonic_clock_stood_still`, `an_authenticated_request_restarts_the_idle_limit`, `a_refused_request_does_not_restart_the_idle_limit` and `a_token_that_arrives_after_the_idle_limit_is_refused_and_stops_the_server_before_its_next_poll` in `tests/server.rs`, and the signal tests in `tests/open.rs`. |
| When the server stops, it accepts no new connection and the runner refuses every new run. Each run then has five seconds to finish by itself, and the runner cancels the runs that are left. Each stream writes its `ended` line before its connection closes, and flightdeck removes the opener directory before it exits. | A child that outlives flightdeck, and a stream that the page cannot tell apart from a cut-off one. | `stopping_the_server_cancels_a_run_that_outlasts_the_shutdown_grace_and_ends_its_stream` in `tests/routes.rs`, and `sigterm_during_a_streamed_run_lets_the_run_finish_and_its_stream_end_before_flightdeck_exits` in `tests/open.rs`. |

### Why a cross-site navigation to the page is allowed

The opener page is a file, and a browser marks a navigation from a file to
an `http` address as `cross-site`. Refusing that navigation would refuse the
first request of every session. The server therefore allows a cross-site
`GET` whose `Sec-Fetch-Mode` is `navigate` to a path outside `/api/`. Such a
request reaches only the embedded page, which holds no secret, and the
content security policy's `frame-ancestors 'none'` stops another site from
framing it. A navigation to an `/api/` path is refused.

## The run routes

| Route | Body | Reply |
| --- | --- | --- |
| `POST /api/runs` | `{"path": [...], "values": {...}}` | a stream, or 422 |
| `POST /api/previews` | the same | a stream, or 422 |
| `POST /api/confirmations/<id>` | the same, which must match the preview's command line | a stream, or 422 |
| `POST /api/runs/<id>/cancel` | none | 204, or 404 with a failure document, such as `flightdeck::run::not_running` for a run that has ended |

A stream is a 200 reply of `application/x-ndjson`, one JSON object on each
line, tagged by `event`:

- `started`, with `run` (the id that the cancel route takes), `line` (the
  command line, POSIX-quoted, without the flags that flightdeck adds) and
  `started_at` (RFC 3339 in UTC). A preview adds `preview`, true when it
  runs the command with `--dry-run`, and `confirmation`.
- `line`, with `text`, for each line that the tool prints on standard
  output.
- `alive`, after each 15 seconds without output, which the page ignores.
- `ended`, with `outcome` (tagged by `kind`: `result`,
  `result_with_failures`, `tool_failure` with the tool's `failure`,
  `cancelled`, or `failed` with flightdeck's `failure` document), `exit`
  and `duration_ms`. `exit` is the code the child exited with, or 128 plus
  the signal that ended it, such as 130 for SIGINT, or `null` when
  flightdeck could not learn how the child ended. A preview adds
  `confirmation`, the id that its dry run issued when it ended with a
  result, or `null`.

The `ended` record is the terminal record of a stream that runs a command.
The reply has no length, so it ends when the server closes the connection,
and a stream that closes without an `ended` line was cut off.

A preview of a command without `--dry-run` runs nothing. Its stream is one
`started` line with `run: null`, `preview: false` and the confirmation id,
and has no `ended` line.

## Known limits

- **Another user can hold the server's connections.** A local process
  that opens 16 connections and sends nothing holds every connection for up
  to 10 seconds, and opening more keeps the page from reaching the server
  for as long as it continues. It cannot read or change anything, and it
  stops nothing that the page has started. That process is the same user,
  who is out of scope, or another user who knows the port.
- **A refusal for a connection over the limit is written by the accept
  thread.** The accept thread writes the 503 reply, shuts the connection
  down for writing, and drops what the client has already sent without
  waiting for more. While a slow client takes the 503 reply, for at most
  the 10 seconds of a reply's write deadline, the server accepts no other
  connection.
- **A long stream does not restart the idle limit.** Only an authenticated
  request restarts it, so a run that streams for longer than 30 minutes
  while the page sends nothing else ends when the server stops.
- **A client that reads slowly can delay the stop.** When the server
  stops, each stream writes its `ended` line, and each line may wait 10
  seconds for the client to read it.
- **The launch code stays in the browser's history.** The browser records
  the address with its fragment, so the code is in the history database.
  The page that H4 builds removes the fragment with `history.replaceState`
  before it exchanges the code. The code that remains in the history works
  once and for at most 60 seconds, so by the time anyone reads it from
  there it opens nothing.
- **With `--no-browser`, the person has 60 seconds to open the page.** The
  launch code's lifetime starts when flightdeck starts, so a person who
  opens the opener page later than that gets a refusal and runs
  `flightdeck open` again.
- **The system's opener may open the page in an editor.** flightdeck hands
  the opener page, an `.html` file, to `open` or `xdg-open`, which use the
  system's association for HTML files. Where that association names an
  editor, the page opens there and shows the launch code instead of using
  it. The file is private to the user, so this exposes nothing to anyone
  else.
- **The placeholder page does not exchange the code.** The page that the
  server embeds says that it is not built yet, so the launch code stays in
  the address bar until it expires, and nothing in the browser uses the run
  routes yet.
