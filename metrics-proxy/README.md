# Metrics proxy

WARNING: This proxy is for prototyping only, it is not prod ready !

This proxy was created as a prototype for scraping metrics off of IC nodes using vector.
While vector could scrape directly the metrics from the nodes, it would fail if the node does not respond
correctly. On top of that, the `up` metric was not present there, and needs to be added.

Thus the two purposes of the proxy are:
- Respond 200 on each query
- Add the up metrics (`1` if the host responds, `0` if it does not)

# Known issues

There might be issues if the node sends back data which are not prometheus formatted. In that case, vector
will just discard the received data as not parseable

There are hard-coded ports which will change the scheme of the request for the remote destination. This
is not a sane configuration and should not stay in the final proxy.

The scheme cannot be read from the destination URL (This is a known Golang bug). For some reason, this only 
happens with `https`. We need a saner option for that as well.

# Going further

Here are some examples to base upon for a final version in rust
- https://github.com/hyperium/hyper/blob/master/examples/http_proxy.rs
- https://github.com/jbg/prox/blob/master/src/main.rs
- https://github.com/tokio-rs/axum/tree/main/examples/http-proxy
