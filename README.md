# YouTube Live Comment Fetcher

## Development

### YouTube API Mock Server

For local development, you can use the YouTube API Mock server:

```bash
docker compose up
```

This will start the gRPC mock server at `localhost:50051`.

To stop the server:

```bash
docker compose down
```

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Some external dependencies may carry additional copyright notices and license terms.
When building and distributing binaries, those external library licenses may be included.

### Proto Definitions

This project uses proto definitions from [yt-api-proto](https://github.com/yuge42/yt-api-proto), 
which is licensed under the Apache License, Version 2.0. Binaries distributed from this project 
will contain work derived from these proto definitions.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.