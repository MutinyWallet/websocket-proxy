# Websocket TCP Proxy

This is a Websocket to TCP socket proxy that forwards packets back and forth. The Websocket side is the initiator and they specify the destination with the URL `/v1/127_0_0_1/8080`.

## Running

This runs both as an axum server or a wasm serverless worker on Cloudflare. Follow the wrangler publish instructions to deploy on cloudflare. To run as an axum server: 

```
cargo run --no-default-features --features="server"
```



## Development

With `wrangler`, you can build, test, and deploy your Worker with the following commands:

```sh
# install wrangler if you do not have it yet
$ npm install -g wrangler

# log into cloudflare if you havent before
$ wrangler login

# compiles your project to WebAssembly and will warn of any issues
$ npm run build

# run your Worker in an ideal development workflow (with a local server, file watcher & more)
$ npm run dev

# deploy your Worker globally to the Cloudflare network (update your wrangler.toml file for configuration)
$ npm run deploy
```

### CICD

There's an example workflow here for publishing on master branch pushes. You need to set `CF_API_TOKEN` in your github repo secrets first.

You also should either remove or configure `wrangler.toml` to point to a custom domain of yours:

```
routes = [
    { pattern = "example.com/about", zone_id = "<YOUR_ZONE_ID>" } # replace with your info
]
```

and any other info in `wrangler.toml` that is custom to you, like the names / id's of queues or kv's.

### WebAssembly

`workers-rs` (the Rust SDK for Cloudflare Workers used in this template) is meant to be executed as compiled WebAssembly, and as such so **must** all the code you write and depend upon. All crates and modules used in Rust-based Workers projects have to compile to the `wasm32-unknown-unknown` triple.

Read more about this on the [`workers-rs`](https://github.com/cloudflare/workers-rs) project README.
