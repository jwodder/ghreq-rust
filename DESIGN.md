Write a Rust "translation" of `ghreq`

- Architecture request & response types with traits à la
  <https://users.rust-lang.org/t/34567/3>

- The primary crate (`ghreq`) defines the core traits & types and also has
  features for enabling specific client backends
    - builtin sync backend: `ureq`
    - builtin async backend: `reqwest`
    - Also provide a sync backend that uses `reqwest`?
    - `isahc` could be used for both backends, but
      <https://github.com/sagebind/isahc/issues/432>

- Provide an additional crate (`ghreq-util`) containing pre-made request &
  response instances, largely crafted to my needs

Clients
=======

- Configurable client parameters:
    - base API URL
    - token
    - User-Agent
    - Accept
    - API Version
    - extra headers
    - mutation delay
    - retry config
    - default timeout(s)
    - These parameters are just stored next to backend clients rather than
      being applied to them; the headers & timeout are only applied when
      preparing each request.

- `[Async]Ghreq<C: {Sync,Async}BackendClient>`
    - Methods:
        - `prepare_request<R: ghreq::Request<T>, T>(r: &R) -> (RequestParts,
          impl [Async]Read)`
            - Should this be public?  What would it be used for?
            - Enclose return values in a `PreparedRequest<R>` struct?
        - `backend_client(&self) -> &C`
            - Rename to just `backend()`? `backend_ref()`?
        - `request<R, T>(&self, req: R) -> Result<R::RequestParser::Output,
          ghreq::Error<C::Error, R::RequestParser::Error>> where R:
          ghreq::Request<T>` — do request, run response through
          `ResponseParser`
        - do paginated request (with `PaginatedRequest` trait), return
          iterator/stream of values
        - `stream_request<R, _>(&self, req: R) -> Result<Response<[Async]Read>,
          Error…>` — do request, return reader for/stream of raw bytes
    - `type GhreqUreq = SyncGhreq<ureq::Agent>`
    - `type GhreqReqwest = AsyncGhreq<reqwest::Client>`

- `trait {Sync,Async}BackendClient` — Implemented on backend clients
    - Rename to just `*Backend`?
    - `type Request`
    - `type Response: [Async]SyncResponse`
        - `trait {Sync,Async}Response`:
            - `url() -> Url`
            - `status() -> StatusCode`
            - `headers() -> HeaderMap`
            - `body_reader(self) -> impl [Async]Read`
    - `type Error`
    - `prepare_request(&self, r: RequestParts) -> Self::Request`
        - Should this be fallible?
        - `RequestParts` fields (computed by ghreq client):
            - `url: Url` — final URL (base URL + endpoint + params)
            - `method: Method`
            - `headers: HeaderMap` — includes session headers
            - `timeout: Duration` — takes session timeout into account
    - `[async] fn send<R: [Async]Read>(&self, r: Self::Request, body: R) ->
      Result<Self::Response, Self::Error>`

Requests
========

- `trait Request`:
    - `type Output`
    - `type Error`
    - `endpoint(&self) -> ???` — required; returns either a full URL or a path
      relative to the API base URL
        - Return `Endpoint`, an enum of `Url` and `Vec<String>`?
        - Support `Cow`s?
    - `method(&self) -> Method` — required
        - Only `GET`, `POST`, `PUT`, `PATCH`, or `DELETE`
    - `headers(&self) -> HeaderMap` — defaults to returning an empty map
    - `params(&self) -> ???` — returns a collection of query parameters;
      defaults to an empty collection
        - Return type: `HashMap<String, String>`?  `Vec<(String, Option<String>)>`?
    - `body(&self) -> impl RequestBody` — required
    - `timeout(&self) -> Option<Duration>` — defaults to `None`, in which case
      the session timeout is used
    - `parser(&self) -> impl RequestParser<Output=Self::Output,
      Error=Self::Error>` — required

- `trait RequestBody`:
    - `headers(&self) -> HeaderMap` — default implementation returns an empty
      map
    - `into_{async_}read(self) -> Result<impl [Async]Read,
      std::io::Error>`
    - Implementations provided:
        - `EmptyBody`
            - Sets Content-Length to 0
        - `JsonBody(T: Serialize)`
            - Sets Content-Type to `application/json`
        - `BytesBody(Vec<u8>)`
        - `TextBody(String)`
        - `Path(PathBuf)`
            - Sets Content-Length to file size
        - `FileBody(std::fs::File)`?

- `trait PaginatedRequest`:
    - `type Item`
    - `endpoint(&self) -> ???` — required; returns either a full URL or a path
      relative to the API base URL
    - Method is always GET
    - No body
    - `headers(&self) -> HeaderMap` — defaults to returning an empty map
    - `params(&self) -> ???` — returns a collection of query parameters;
      defaults to an empty collection
    - request timeout?
    - Responses are parsed via `PageParser<Item>`

Responses
=========

- Define a clonable, backend-agnostic `ResponseParts` structure containing:
    - `initial_url: Url`
    - `url: Url` — the final URL after resolving redirects
    - `method`
    - `status`
    - `headers`
    - cf. `http::response::Parts`

- `type Response<Body> { parts, body }`

- `trait ResponseParser`:
    - `type Output`
    - `type Error`
    - `from_parts(&mut self, m: &ResponseParts)` — Called to start processing
      of the response
    - `feed_bytes(&mut self, b: Vec<u8>)` — Called repeatedly for chunks of the
      response body
    - `end(self) -> Result<Self::Output, Self::Error>`

- `trait ResponseParseExt` — blanket trait
    - `process_{async_}response(self, impl Response<impl [Async]Read>) -> Result<Output, Error | io::Error>`
    - `map(self, FnOnce(Output -> Output2)) -> impl Response<Output=Output2>`
    - `try_map(self, FnOnce(Result<Output, Error | io::Error> ->
      Result<Output2, ???>) -> impl<Response<Output=Output2, Error=???>`
        - Require that `InputError: Into<OutputError>`?

- Builtin implementations of `ResponseParser` (all using the same builtin
  `Error` type):
    - `JsonResponse<T: Deserialize>`
    - `Ignore` — throws away response data, returns `()`
    - `Vec<u8>`
    - `String`
    - `WithParts<R: ResponseParser>`
        - `Output = Response<R::Output>`
    - `PageParser<Item>` — parses a pagination response and returns a `Page`
      with the next link, a list of items, and (if known? maybe?) the total
      number of items
    - `Write(impl Write)`

Errors
======

```rust
struct ghreq::Error<ClientError, ParserError> {
    url: Url,
    method: Method,
    payload: ErrorPayload<ClientError, ParserError>,
}

// Error methods include:
// - payload_ref(&self) -> &ErrorPayload
// - payload_mut(&mut self) -> &mut ErrorPayload
// - into_payload(self) -> ErrorPayload
// - kind(&self) -> PayloadKind // ? // C-style enum with variants matching ErrorPayload
// - is_send_error(&self) -> bool // etc. ?
// - into_send_error(self) -> Option<ClientError> // etc. ?

enum ErrorPayload<ClientError, ParserError> {
    Send(ClientError),
    Status(ErrorResponse),
    ReadRequestBody(std::io::Error),
    ReadResponse(std::io::Error),
    Parse(ParserError),
}
```

- When displayed, `Error` shows `"{method} request to {url} failed: {payload}"`

- Default `ParserError` to my builtin `ResponseParser::Error` type?

- Define type aliases of `Error` (and `ErrorPayload`?) for each backend client

- HTTP errors are returned as `ErrorResponse` values with the following fields:
    - `parts: ResponseParts`
    - `body: ErrorBody` — enum of `Empty`, `Text(String)`, and `Json(serde_json::Value)`

- `ErrorResponse` and containing errors do not show the error body by default,
  but they do have `error_response_body(&self) -> Option<Cow<'_, str>>` methods
  (tentative name) that show the body with JSON pretty-printed.  This method
  can be used by error-display code in `main()` etc.
    - Add a wrapper around the types that adds the error response body to the
      Display?

Other
=====

- Put most traits in a prelude?

- Include public functionality for appending a path to a URL, replacing the URL
  if the path is itself a URL?
    - Make this a method of the `Endpoint` type?  Or of a `UrlExt` trait along
      with a method for applying params?

- Include a "tracing" feature (off by default)

- `trait HeaderMapExt`
    - `is_json_content_type(&self) -> bool`
    - `content_length(&self) -> Option<u64>`
    - `pagination_links(&self) -> PaginationLinks`
        - `PaginationLinks` (TODO: Check names against standards):
            - `next: Option<Url>`
            - `last: Option<Url>`
            - `first: Option<Url>`
            - `prev: Option<Url>`

- cf. <https://github.com/snok/container-retention-policy/blob/b439c10ae57ac70bd2301813dc2d0f708dc78f31/src/client/builder.rs#L70> regarding rate limiting
