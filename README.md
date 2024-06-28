**Command**

```text
Usage: tsr [OPTIONS]

Options:
  -c, --config <CONFIG>  [default: config.yaml]
  -h, --help             Print help
  -V, --version          Print version
```

**Configuration** 


```yaml
bind:
  addr: 127.0.0.1
  listen: 8080

server:
  info: "Powered by Rust"
  root: .
  index: index.html # optional
  error_page: 404.html # optional

allowlist: # optional
  - 127.0.0.1

blocklist: # optional
  - 114.114.114.114
```

**Benchmark (wrk)**
```text
kfatyuip@archlinux [22:45:19] [~/tsr] [main]
-> % time wrk http://localhost:8080 -t 4 -d 10s # cargo run --no-default-features --features=lru_cache --release
Running 10s test @ http://localhost:8080
  4 threads and 10 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   368.09us  153.60us   2.68ms   73.04%
    Req/Sec     3.77k   184.57     4.47k    74.69%
  151303 requests in 10.10s, 114.28MB read
  Socket errors: connect 0, read 151300, write 0, timeout 0
Requests/sec:  14981.07
Transfer/sec:     11.32MB
wrk http://localhost:8080 -t 4 -d 10s  1.43s user 10.98s system 122% cpu 10.107 total

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
