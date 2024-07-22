## Rusted HTTP Server [![Rust](https://github.com/kfatyuip/tsr/actions/workflows/rust.yml/badge.svg)](https://github.com/kfatyuip/tsr/actions/workflows/rust.yml)

**Command**

```text
Usage: tsr [OPTIONS]

Options:
  -c, --config <CONFIG>  [default: ]
  -h, --help             Print help
  -V, --version          Print version
```

**Features**

`log`: print requests to stdout (enabled by default)

`index_sort`: sort files and directories like python http.server (enabled by default)

`lru_cache`: cache the pages for better performance

`allow_ip` & `block_ip`: enable for allowing ip or blocking ip

**Configuration** 


```yaml
bind:
  addr: 127.0.0.1
  listen: 8080

server:
  info: "Powered by Rust"
  root: .
  auto_index: false # optional
  index: index.html # optional
  error_page: 404.html # optional

allowlist: # requires feature "allow_ip"
  - 127.0.0.1

blocklist: # requires feature "block_ip"
  - 114.114.114.114
```

**Benchmark (wrk)**
```text
kfatyuip@archlinux [19:32:18] [~/tsr] [main]
-> % time wrk http://localhost:8080 -t 4 -d 10s # cargo run --no-default-features --features=lru_cache --release
Running 10s test @ http://localhost:8080
  4 threads and 10 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   366.46us  157.63us   3.69ms   73.90%
    Req/Sec     3.84k   188.99     4.67k    72.21%
  153835 requests in 10.10s, 121.91MB read
  Socket errors: connect 0, read 153829, write 0, timeout 0
Requests/sec:  15231.82
Transfer/sec:     12.07MB
wrk http://localhost:8080 -t 4 -d 10s  1.41s user 10.96s system 122% cpu 10.109 total

kfatyuip@archlinux [22:45:30] [~/tsr] [main]
-> % time wrk http://localhost:8080 -t 4 -d 10s # python -m http.server 8080
Running 10s test @ http://localhost:8080
  4 threads and 10 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     8.00ms    5.81ms 208.44ms   98.54%
    Req/Sec   244.62     37.21   313.00     79.00%
  9751 requests in 10.01s, 6.94MB read
Requests/sec:    974.16
Transfer/sec:    709.69KB
wrk http://localhost:8080 -t 4 -d 10s  0.16s user 0.87s system 10% cpu 10.021 total

kfatyuip@archlinux [22:45:48] [~/tsr] [main]
-> % time wrk http://localhost:8080 -t 4 -d 10s # nginx
Running 10s test @ http://localhost:8080
  4 threads and 10 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   440.22us  673.30us  26.57ms   99.79%
    Req/Sec     4.80k   419.58     6.00k    64.60%
  193247 requests in 10.10s, 157.20MB read
Requests/sec:    19134.09
Transfer/sec:    15.57MB
wrk http://localhost:8080 -t 4 -d 10s  1.24s user 4.41s system 55% cpu 10.107 total
```
