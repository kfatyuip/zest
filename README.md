## Rusted HTTP Server [![Rust](https://github.com/kfatyuip/zest/actions/workflows/rust.yml/badge.svg)](https://github.com/kfatyuip/zest/actions/workflows/rust.yml)

**Features**

`log`: print requests to stdout (enabled by default)

`index_sort`: sort files and directories like python http.server (enabled by default)

`ip_limit`: ip allowlist and ip blocklist (enabled by default)

`lru_cache`: cache the pages for better performance

**Configuration** 

```yaml
bind:
  addr: 127.0.0.1
  listen: 8080

server:
  info: "Powered by Rust"
  root: .
  error_page: 404.html # optional
  tick: 256 # optional (ms)
  cache: # optional
    index_capacity: 16
    file_capacity: 32

allowlist: # optional
  - 127.0.0.1

blocklist: # optional
  - 192.168.0.1/24

rate_limit: # optional
  max_requests: 1024

locations: # optional
  /:
    auto_index: false
    index: index.html

logging: # optional
  access_log: /var/log/zest/access.log
  error_log: /var/log/zest/error.log
```

**Benchmark (wrk)**
+ cargo run --release --no-default-features --features=lru_cache -- -p 8080
```text
Running 10s test @ http://localhost:8080
  4 threads and 10 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   317.11us  168.20us   4.56ms   78.71%
    Req/Sec     4.06k   204.86     5.18k    74.19%
  162856 requests in 10.10s, 117.10MB read
  Socket errors: connect 0, read 162854, write 0, timeout 0
Requests/sec:  16125.32
Transfer/sec:     11.60MB
wrk http://localhost:8080 -t 4 -d 10s  1.59s user 11.45s system 128% cpu 10.110 total
```

+ python -m http.server 8080
```text
Running 10s test @ http://localhost:8080
  4 threads and 10 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     8.00ms    5.81ms 208.44ms   98.54%
    Req/Sec   244.62     37.21   313.00     79.00%
  9751 requests in 10.01s, 6.94MB read
Requests/sec:    974.16
Transfer/sec:    709.69KB
wrk http://localhost:8080 -t 4 -d 10s  0.16s user 0.87s system 10% cpu 10.021 total
```
+ nginx
```text
Running 10s test @ http://localhost:8080
  4 threads and 10 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   440.22us  673.30us  26.57ms   99.79%
    Req/Sec     4.80k   419.58     6.00k    64.60%
  193247 requests in 10.10s, 157.20MB read
Requests/sec:    19134.09
Transfer/sec:       15.57MB
wrk http://localhost:8080 -t 4 -d 10s  1.24s user 4.41s system 55% cpu 10.107 total
```
