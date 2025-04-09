# Http Proxy IPv6 Pool

Make every request from a separate IPv6 address. The primary usecase for this is to bypass rate limiting on some websites.

## Tutorial

Assuming you already have an entire IPv6 subnet routed to your server. If your provider doesn't support IPv6, you can use [Hurricane Electric's Tunnelbroker](https://tunnelbroker.net/) to get a /64 subnet routed to your server.

Get your IPv6 subnet prefix, for me it is `2a14:7c0:4b20:3b84::/64`

Open `ip_nonlocal_bind` for binding any IP address:

```sh
sysctl net.ipv6.ip_nonlocal_bind=1
```

Now you can test the sysctl setting has been applied by using `curl`:

```sh
$ curl --interface 2a14:7c0:4b20:3b84::1 ipv6.ip.sb
2a14:7c0:4b20:3b84::1

$ curl --interface 2a14:7c0:4b20:3b84::2 ipv6.ip.sb
2a14:7c0:4b20:3b84::2
```

Run with docker:

```
docker run -d --name v6-proxy --network host ghcr.io/vojkovic/http-proxy-ipv6-pool:latest -b [::1]:51080 -i 2a14:7c0:4b20:3b84::/64
```

```sh
$ while true; do curl -x http://[::1]:51080 ipv6.ip.sb; done
2a14:7c0:4b20:3b84:795d:1b7c:bfcf:d639
2a14:7c0:4b20:3b84:d4f5:a257:51b7:9883
2a14:7c0:4b20:3b84:50e7:3eb6:963c:2a0e
2a14:7c0:4b20:3b84:4494:6fe4:596c:8f6
2a14:7c0:4b20:3b84:c28d:e62e:606f:4c70
2a14:7c0:4b20:3b84:ab7e:e224:7417:b34e
2a14:7c0:4b20:3b84:845b:ba42:77e6:9b16
2a14:7c0:4b20:3b84:860e:a034:bbeb:e058
```

Also check that IPv4 is working as it will attempt to fallback to it if IPv6 fails for any reason.

```sh
$ while true; do curl -x http://[::1]:51080 ipv4.ip.sb; done
45.32.99.235
45.32.99.235
45.32.99.235
45.32.99.235
45.32.99.235
45.32.99.235
45.32.99.235
```

Great, you're all set!

## Authors

Original Author: [zu1k](https://github.com/zu1k)
IPv4 Support: [unixfox](https://github.com/unixfox)
Happy Eyeballs & Docker Support: [vojkovic](https://github.com/vojkovic)
