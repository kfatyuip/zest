# Hot Reload

Zest use SIGHUP as the reload signal, when running as a daemon (zest &)

```bash
$ kill -SIGHUP $(pidof zest)
```

or using the script in scripts/ directory,

and after a socket request (chose it for better performance), it will be reloaded
