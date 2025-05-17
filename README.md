DHENRY for Mytinydc.com

# http_reverse_proxy

This project has been built with the AI Deepseek, for the fun and to learn Rust.

<a href="https://www.mytinydc.com/en/blog/news-2025-mai-11/" target="_story">_The story behind this software (clic here)_</a>

## Installation

- <a href="https://www.rust-lang.org/tools/install" target="_rust">The Rust tool chain is mandatory</a>
- build

```bash
cargo build --release
# If you need to listen on port under 1024 as non root
sudo setcap 'cap_net_bind_service=+ep' target/release/http_reverse_proxy
cp target/release/http_reverse_proxy /usr/local/bin
# to store config and tls certificates (pem)
mkdir -p /etc/http_reverse_proxy/certs/
```

- Create config.yaml: copy from config-sample.yaml
- copy config.yaml to `/etc/http_reverse_proxy/`, you could specify an other path with the parameter "-c"
- run `nohup http_reverse_proxy &`

## Websocket test server

Requires installation of Nodejs. This service implements a simplistic websockets server. The Js code has been provided by the DeepSeep AI.

```bash
# install Nodejs
sudo apt install nodejs
npm install
```

Run the service (default listener port is 8080) : `nodejs wsserver.js`

To test operation, use the command: `node_modules/.bin/wscat -c ws://localhost:8080`

## Roadmap

- [x] Yaml Configuration: frontend ⇨ acl ⇨ backend ⇨ servers
- Frontends :
  - [x] Listeners HTTP/HTTPS
  - upgrade websocket
    - [x] HTTP
    - [x] HTTPS
- Backends :
  - [x] HTTP et HTTPS (including websockets)
- [x] Roundrobin distribution, **only**
- [x] Inserting the “X-Forwarded-For” header
- [ ] API Rest for configuration changes
- [ ] Apply configuration without restarting (hot-reload)
- [x] Automatic loading of certificates (pem format) for TLS resolution
- [x] Integrate a (basic) anti-bot system. The aim is not to develop a complete system, but to attempt an implementation at the heart of the LoadBalancer.
- [ ] Tls certificate fallback

## Licence

MIT Licence see file LICENSE.txt
