FROM alpine:latest

# Install curl
RUN apk add --no-cache curl

WORKDIR /app

# Download and make the updater executable
RUN curl -L https://github.com/Kerwood/confluence-updater/releases/latest/download/confluence-updater-x86_64-unknown-linux-musl -o /usr/local/bin/confluence-updater && \
    chmod +x /usr/local/bin/confluence-updater

# CMD
CMD ["/usr/local/bin/confluence-updater"]
