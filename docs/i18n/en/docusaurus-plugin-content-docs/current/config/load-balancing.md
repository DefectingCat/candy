---
sidebar_label: Load Balancing
sidebar_position: 3
title: Load Balancing
---

## Load Balancing Overview

Candy supports multiple load balancing algorithms, allowing you to distribute requests to multiple backend servers, improving application availability and performance. Load balancing functionality is implemented through the `upstream` configuration.

## Configuration Methods

### 1. Define Upstream Server Groups

Define one or more upstream server groups in your configuration file, each containing multiple server addresses and weights:

```toml
# Define upstream server group
[[upstream]]
name = "backend_servers"  # Server group name (referenced in routes)
method = "weightedroundrobin"  # Load balancing algorithm (default: weightedroundrobin)
server = [
    { server = "192.168.1.100:8080", weight = 3 },  # Weight 3
    { server = "192.168.1.101:8080", weight = 1 },  # Weight 1
    { server = "http://api1.example.com", weight = 2 },  # Supports HTTP protocol prefix
    { server = "https://api2.example.com:443", weight = 1 }  # Supports HTTPS
]

# Second upstream server group (IP Hash algorithm)
[[upstream]]
name = "session_servers"
method = "iphash"  # IP Hash algorithm
server = [
    { server = "192.168.1.102:8080", weight = 1 },
    { server = "192.168.1.103:8080", weight = 1 },
    { server = "192.168.1.104:8080", weight = 1 }
]
```

### 2. Use Upstream Server Groups in Routes

Reference defined upstream server groups in virtual host route configurations:

```toml
[[host]]
ip = "0.0.0.0"
port = 8084
server_name = "loadbalance.example.com"

[[host.route]]
location = "/api"
upstream = "backend_servers"  # Reference upstream server group name
proxy_timeout = 30  # Proxy timeout (seconds)
max_body_size = 1048576  # Maximum request body size (bytes)
```

## Load Balancing Algorithms

Candy supports the following three load balancing algorithms:

### 1. Weighted Round Robin - Default

```toml
method = "weightedroundrobin"
```

- Distributes requests proportionally to weights
- Higher weight means more requests
- Suitable for scenarios with servers of varying performance
- **Example configuration:**

```toml
[[upstream]]
name = "weighted_servers"
method = "weightedroundrobin"
server = [
    { server = "192.168.1.100:8080", weight = 3 },  # Handles 3/7 requests
    { server = "192.168.1.101:8080", weight = 2 },  # Handles 2/7 requests
    { server = "192.168.1.102:8080", weight = 1 },  # Handles 1/7 requests
    { server = "192.168.1.103:8080", weight = 1 }   # Handles 1/7 requests
]
```

### 2. Round Robin

```toml
method = "roundrobin"
```

- Distributes requests sequentially to each server
- All servers have equal weight
- Suitable for scenarios with servers of similar performance

### 3. IP Hash

```toml
method = "iphash"
```

- Uses client IP address hash to select server
- Requests from the same IP are always routed to the same server
- Suitable for applications requiring session persistence
- **Note:** Session may break if server list changes

## Server Weights

Weights are used for weighted round robin algorithm, ranging from 1-255. Higher weight means more requests.

```toml
server = [
    { server = "server1:8080", weight = 5 },  # Handles 50% of requests
    { server = "server2:8080", weight = 3 },  # Handles 30% of requests
    { server = "server3:8080", weight = 2 }   # Handles 20% of requests
]
```

## Configuration Examples

### 1. Basic Load Balancing

```toml
log_level = "info"
log_folder = "./logs"

# Define upstream server group
[[upstream]]
name = "backend"
method = "weightedroundrobin"
server = [
    { server = "192.168.1.100:8080", weight = 2 },
    { server = "192.168.1.101:8080", weight = 1 },
    { server = "192.168.1.102:8080", weight = 1 }
]

# Virtual host configuration
[[host]]
ip = "0.0.0.0"
port = 80
server_name = "api.example.com"

[[host.route]]
location = "/"
upstream = "backend"
proxy_timeout = 10
max_body_size = 1048576
```

### 2. Session Persistence (IP Hash)

```toml
log_level = "info"
log_folder = "./logs"

# Define IP Hash server group
[[upstream]]
name = "session_aware"
method = "iphash"
server = [
    { server = "192.168.1.100:8080", weight = 1 },
    { server = "192.168.1.101:8080", weight = 1 },
    { server = "192.168.1.102:8080", weight = 1 }
]

[[host]]
ip = "0.0.0.0"
port = 80
server_name = "app.example.com"

[[host.route]]
location = "/app"
upstream = "session_aware"
proxy_timeout = 30
max_body_size = 10485760
```

### 3. Multiple Load Balancing Strategies

```toml
log_level = "info"
log_folder = "./logs"

# API server group (weighted round robin)
[[upstream]]
name = "api_servers"
method = "weightedroundrobin"
server = [
    { server = "api1.example.com:8080", weight = 3 },
    { server = "api2.example.com:8080", weight = 2 },
    { server = "api3.example.com:8080", weight = 1 }
]

# Static resource server group (round robin)
[[upstream]]
name = "static_servers"
method = "roundrobin"
server = [
    { server = "static1.example.com:80", weight = 1 },
    { server = "static2.example.com:80", weight = 1 }
]

[[host]]
ip = "0.0.0.0"
port = 80
server_name = "example.com"

# API route
[[host.route]]
location = "/api"
upstream = "api_servers"
proxy_timeout = 10
max_body_size = 1048576

# Static resource route
[[host.route]]
location = "/static"
upstream = "static_servers"
proxy_timeout = 30
max_body_size = 10485760
```

## Health Checks (Coming Soon)

The current version of Candy does not support active health checks. For health check functionality, consider:

1. Using external health check tools (e.g., Prometheus + Alertmanager)
2. Configuring server-level timeouts and retry mechanisms
3. Monitoring server response status regularly

## Best Practices

1. **Server Monitoring**: Regularly check server status and response time
2. **Weight Configuration**: Reasonably assign weights based on server performance
3. **Session Management**: Use IP Hash algorithm for session persistence
4. **Server Count**: Configure at least 2 servers for high availability
5. **Timeout Settings**: Set reasonable proxy timeout to avoid long waits
6. **Maximum Request Body**: Set appropriate maximum request body size based on actual business needs

## Limitations

- No support for active health checks
- No support for dynamic server up/down
- No support for connection pool configuration
- No support for traffic mirroring
