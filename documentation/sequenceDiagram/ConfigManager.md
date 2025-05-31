# ConfigManager

ConfigManager is a centralized configuration loader and provider that:

- Loads settings (YAML).

- Safely exposes them to the rest of the app.

- Enables tls_acceptor setup.

- Designed for async/thread-safe usage.

## sequenceDiagram

```mermaid
sequenceDiagram
    participant Main
    participant Args
    participant ConfigManager
    participant tls_acceptor
    participant forwarder_from_https
    participant forwarder_from_http
    participant api_rest

    Main->>Args: parse()
    Args-->>Main: args

    Main->>ConfigManager: new(args)
    ConfigManager-->>Main: config_manager

    Main->>ConfigManager: load().await
    activate ConfigManager
    ConfigManager->>+ConfigManager: load()
    ConfigManager->>-ConfigManager: load_servers_tracker()
    deactivate ConfigManager

    Main->>ConfigManager: get_config().await
    activate ConfigManager
    ConfigManager-->>Main: config
    deactivate ConfigManager

    Main->>ConfigManager: get_config_tls_certs_path().await
    activate ConfigManager
    ConfigManager-->>Main: certs_path
    deactivate ConfigManager

    Main->>tls_acceptor: tls_acceptor_init(certs_path)
    tls_acceptor-->>Main: tls_acceptor

    Main->>Main: shared_manager = Arc::new(RwLock::new(config_manager))
    Note right of Main: Shared ownership<br/>for async use

    Main->>forwarder_from_https: proxy_from_https(shared_manager,frontend_name,frontend_addr,tls_acceptor)

    Main->>forwarder_from_http: proxy_from_http(shared_manager,frontend_name,frontend_addr)

    Main->>api_rest: apirest_http(shared_manager,frontend_name,frontend_addr)
```
