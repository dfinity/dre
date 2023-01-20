package main

import (
	"bytes"
	"crypto/tls"
	"flag"
	"io"
	"log"
	"net"
	"net/http"
	"strconv"
	"strings"
)

// Hop-by-hop headers. These are removed when sent to the backend.
// http://www.w3.org/Protocols/rfc2616/rfc2616-sec13.html
var hopHeaders = []string{
	"Connection",
	"Keep-Alive",
	"Proxy-Authenticate",
	"Proxy-Authorization",
	"Te", // canonicalized version of "TE"
	"Trailers",
	"Transfer-Encoding",
	"Upgrade",
}

func copyHeader(dst, src http.Header) {
	for k, vv := range src {
		for _, v := range vv {
			dst.Add(k, v)
		}
	}
}

func delHopHeaders(header http.Header) {
	for _, h := range hopHeaders {
		header.Del(h)
	}
}

func appendHostToXForwardHeader(header http.Header, host string) {
	// If we aren't the first proxy retain prior
	// X-Forwarded-For information as a comma+space
	// separated list and fold multiple headers into one.
	if prior, ok := header["X-Forwarded-For"]; ok {
		host = strings.Join(prior, ", ") + ", " + host
	}
	header.Set("X-Forwarded-For", host)
}

type proxy struct {
}

func (p *proxy) ServeHTTP(wr http.ResponseWriter, req *http.Request) {
	log.Println(req.RemoteAddr, " ", req.Method, " ", req.URL)

	tr := &http.Transport{
		TLSClientConfig: &tls.Config{
			InsecureSkipVerify: true,
		}}
	client := &http.Client{Transport: tr}

	// http: Request.RequestURI can't be set in client requests.
	// http://golang.org/src/pkg/net/http/client.go
	req.RequestURI = ""

	// Without this, the CONNECT method is passed to the remote
	// server. Since this is a prototype, and we will only use
	// GET queries, we can hardcode this
	req.Method = "GET"

	// Some endpoints require HTTPS requests. As previously, the scheme used to connect
	// to the proxy will be tried to the remote host.
	// Even worse, the incoming request in this function does not see the scheme defined
	// (this is a bug in the golang net library, which might never be fixed.)
	// So we set it explicitely in order to get the correct remote scheme
	req.URL.Scheme = "https"

	delHopHeaders(req.Header)

	if clientIP, _, err := net.SplitHostPort(req.RemoteAddr); err == nil {
		appendHostToXForwardHeader(req.Header, clientIP)
	}

	// Necessary or we get EOFs after a while
	// https://stackoverflow.com/questions/17714494/golang-http-request-results-in-eof-errors-when-making-multiple-requests-successi
	// See documentation about this option here https://pkg.go.dev/net/http#Request
	// I think that there was the issue because we were reusing connections to the
	// same host which might have been closed after a while
	req.Close = true
	resp, err := client.Do(req)
	if err != nil {
		log.Println("Could not query remote host: ", err)
		var resp_buff bytes.Buffer
		n, err := io.Copy(&resp_buff, bytes.NewReader([]byte("up 0")))
		if err != nil {
			log.Println("Could not write extra up metric: ", err)
		}
		wr.Header().Set("Content-Length", strconv.Itoa(int(n)))
		wr.WriteHeader(http.StatusOK)
		_, err = io.Copy(wr, &resp_buff)
		if err != nil {
			log.Println("Could not write to response writer: ", err)
		}
		log.Println(req.RemoteAddr, req.URL, "Sent back up 0")
		return
	}
	defer resp.Body.Close()

	delHopHeaders(resp.Header)

	var buff bytes.Buffer
	b, err := io.Copy(&buff, resp.Body)
	if err != nil {
		log.Println("Error: ", err)
	}
	extra, err := io.Copy(&buff, bytes.NewReader([]byte("up 1")))
	if err != nil {
		log.Println("Error: ", err)
	}

	copyHeader(wr.Header(), resp.Header)
	wr.Header().Set("Content-Length", strconv.Itoa(int(b+extra)))
	wr.WriteHeader(resp.StatusCode)
	_, err = io.Copy(wr, &buff)
	if err != nil {
		log.Println("Error: ", err)
	}
	log.Println(req.RemoteAddr, req.URL, resp.Status, "Finished sending body")
}

func main() {
	var addr = flag.String("addr", "127.0.0.1:8080", "The addr of the application.")
	flag.Parse()

	handler := &proxy{}

	log.Println("Starting proxy server on", *addr)
	if err := http.ListenAndServe(*addr, handler); err != nil {
		log.Fatal("ListenAndServe:", err)
	}
}
