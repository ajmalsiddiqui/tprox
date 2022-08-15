# tprox

A multi-connection TCP reverse proxy. The tprox server is able to proxy multiple incoming connections to the tprox client over a single TCP connection between the tprox server and client. The tprox client will then individually proxy each connection to a local server.

> Note: This is a side project and a work in progress. It is far from complete (e.g. the error handling isn't there - everything panics, and connection termination is currently not handled well). It hasn't been profiled for performance. Thus, clearly, this is NOT recommended for production use at all.


## Build

Build the binaries using `cargo build`. This results in two binaries:
1. `tprox-server`: The tprox reverse proxy server.
2. `tprox`: The tprox client that proxies proxied connections coming from `tprox-server` to a local client.

## Run

The `tprox-server` should be running on a publicly accessible endpoint, since this is what your clients will connect to in order to get proxied to your local server.

It takes a single argument: the port that the control server will listen on. E.g.:

```
$ tprox-server 3333
```

The `tprox` binary is the client. This should be on the same machine where you're running the target for the reverse proxy. It needs to be supplied the hostname and port where the tprox server is running, and the port to which connections need to be proxied to. E.g., if the tprox server is listening on `playground.ajmalsiddiqui.me:3333`, and you wish to proxy traffic to `localhost:4444`, this is how you run it:

```
$ tprox playground.ajmalsiddiqui.me:3333 4444
Proxy running at playground.ajmalsiddiqui.me:40011
```

Now any connections made to `playground.ajmalsiddiqui.me:40011` will be proxied to `localhost:4444`.

## TODO

- [ ] Proper error handling instead of panicking everywhere
- [ ] Handle connection termination properly
- [ ] Allow long lived idle connections (TCP keep-alive)
- [ ] Release profile
