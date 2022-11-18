FROM golang:1.18 as build

WORKDIR /app
COPY go.mod ./
RUN go mod download

COPY ./*.go ./
RUN go build -o /up-proxy-proto .

## Deploy
FROM gcr.io/distroless/base
WORKDIR /
COPY --from=build /up-proxy-proto /up-proxy-proto
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/up-proxy-proto"]
