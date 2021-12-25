# xenon

A simple [Socks5](https://datatracker.ietf.org/doc/html/rfc1928) server written in Rust.

## Features

### Methods

- [x] **No authentication**.
- [ ] **[Gssapi](https://datatracker.ietf.org/doc/html/rfc1961) authentication**.
- [x] **[Username and Password](https://datatracker.ietf.org/doc/html/rfc1929) authentication**.

### Commands

- [x] **Connect**
- [ ] **Bind**
- [ ] **Udp associate**

## Usage

The **help menu** can be accessed with `./xenon -h`.

```json
[
  { "username": "user1", "password": "securePass" },
  { "username": "user2", "password": "securePass" }
]
```

```
./xenon -a 127.0.0.1 -p 1080 -vvv -u users.json
```