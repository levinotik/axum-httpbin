# Axum HttpBin

Axum HttpBin is a project aiming to implement functionality similar to httpbin.org using the Axum web framework in Rust. It provides a collection of HTTP endpoints useful for testing and debugging HTTP clients.

## Usage

To start the server locally, clone this repository and run:

```shell
$ cargo run
```

This will start the Axum HttpBin server on `http://localhost:3000`.

## Endpoints

All endpoints return JSON responses, the exact structure of which varies based on the specific request.

So far, Axum HttpBin implements the following HTTP endpoints:

| Method  | Endpoint            | Description                            | Status |
|---------|---------------------|----------------------------------------|--------|
| GET     | `/get`              | Returns the request data as JSON.      |:white_check_mark:|
| PUT     | `/put`              | Returns the request data as JSON.      |:white_check_mark:|
| POST    | `/post`             | Returns the request data as JSON.      |:white_check_mark:|
| PATCH   | `/patch`            | Returns the request data as JSON.      |:white_check_mark:|
| DELETE  | `/delete`           | Returns the request data as JSON.      |:white_check_mark:|
| POST    | `/post/json`        | Returns the JSON data from the request.|:white_check_mark:|
| POST    | `/post/form`        | Returns the form data from the request.|:white_check_mark:|
| POST    | `/post/file`        | Returns the file data from the request.|:white_check_mark:|

## Endpoints

The JSON responses contain, at a minimum, the basic request data:

Request:
```
POST "localhost:3000/post"
```
Response:

```json
{
    "args": {},
        "headers": {
            "accept": "*/*",
            "accept-encoding": "gzip, deflate",
            "connection": "keep-alive",
            "content-length": "0",
            "host": "localhost:3000",
            "user-agent": "HTTPie/3.2.2"
        },
        "method": "POST",
        "origin": "127.0.0.1",
        "url": "/post"
}
```
## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
