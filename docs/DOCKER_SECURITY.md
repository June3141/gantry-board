# Docker Socket Security

## Overview

Gantry Board uses the Docker daemon to manage preview environments for agent worktrees. By default, it connects to the Docker socket at `/var/run/docker.sock`.

## Security Implications

Mounting `/var/run/docker.sock` gives the application **full control over the Docker daemon**, which is equivalent to root access on the host machine. If the application is compromised, an attacker could:

- Create privileged containers
- Mount host filesystems
- Access the host network
- Execute commands on the host

## Recommendations

### Development

The default socket mounting is acceptable for local development:

```yaml
volumes:
  - /var/run/docker.sock:/var/run/docker.sock
```

Ensure the Docker daemon is only accessible locally and the development machine is trusted.

### Production

For production deployments, consider these alternatives (in order of preference):

1. **Rootless Docker** — Run the Docker daemon without root privileges. This limits the blast radius of a compromised socket.

   ```bash
   # Install rootless Docker
   dockerd-rootless-setuptool.sh install
   ```

2. **Docker-in-Docker (DinD) with TLS** — Run a separate Docker daemon inside a container with TLS authentication, isolating the host daemon.

   ```yaml
   services:
     dind:
       image: docker:dind
       privileged: true
       environment:
         - DOCKER_TLS_CERTDIR=/certs
       volumes:
         - docker-certs:/certs
     backend:
       environment:
         - GANTRY_DOCKER_HOST=tcp://dind:2376
         - DOCKER_TLS_VERIFY=1
   ```

3. **Restricted Docker API proxy** — Use a proxy like [docker-socket-proxy](https://github.com/Tecnativa/docker-socket-proxy) that limits which Docker API endpoints are accessible.

   ```yaml
   services:
     docker-proxy:
       image: tecnativa/docker-socket-proxy
       environment:
         - CONTAINERS=1
         - IMAGES=1
         - NETWORKS=1
       volumes:
         - /var/run/docker.sock:/var/run/docker.sock:ro
     backend:
       environment:
         - GANTRY_DOCKER_HOST=tcp://docker-proxy:2375
   ```

## Configuration

The Docker host is configurable via:

- **TOML config:** `docker.host` in `gantry.toml`
- **Environment variable:** `GANTRY_DOCKER_HOST`
- **Default:** `unix:///var/run/docker.sock`

## Resource Limits

Production containers are configured with resource limits in `docker-compose.prod.yml` to prevent runaway resource consumption.

### Default Limits

| Service   | CPU   | Memory (limit) | Memory (reserved) | PID limit |
|-----------|-------|--------------|--------------------|-----------|
| backend   | 4.0   | 4 GB         | 1 GB               | 256       |
| frontend  | 1.0   | 512 MB       | -                  | -         |

### Logging

All services use the `json-file` logging driver with rotation:

- **max-size:** 10 MB per log file
- **max-file:** 3 files (30 MB total per service)

### Adjusting Limits

Override limits via environment-specific compose overrides or by editing `docker-compose.prod.yml` directly:

```yaml
services:
  backend:
    deploy:
      resources:
        limits:
          cpus: '8.0'
          memory: 8G
          pids: 512
        reservations:
          memory: 2G
```

For high-load deployments with many concurrent agents, increase the backend CPU, memory, and PID limits proportionally. The frontend/nginx service typically needs minimal resources.

## References

- [Docker daemon attack surface](https://docs.docker.com/engine/security/#docker-daemon-attack-surface)
- [Rootless Docker](https://docs.docker.com/engine/security/rootless/)
- [Docker-in-Docker](https://hub.docker.com/_/docker)
- [Docker Compose deploy resources](https://docs.docker.com/compose/compose-file/deploy/#resources)
