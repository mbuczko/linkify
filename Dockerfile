FROM ekidd/rust-musl-builder:latest as builder
# https://github.com/emk/rust-musl-builder

ADD --chown=rust:rust  . ./
RUN cargo install --root /tmp/linkify --path .

USER root

# Download the static build of Litestream directly into the path & make it executable.
# This is done in the builder and copied as the chmod doubles the size.
ADD --chown=rust:rust https://github.com/benbjohnson/litestream/releases/download/v0.3.4/litestream-v0.3.4-linux-amd64-static.tar.gz /tmp/litestream.tar.gz
RUN tar -C /tmp -xzf /tmp/litestream.tar.gz

# Download the s6-overlay for process supervision.
# This is done in the builder to reduce the final build size.
ADD --chown=rust:rust https://github.com/just-containers/s6-overlay/releases/download/v2.2.0.3/s6-overlay-amd64-installer /tmp/
RUN chmod +x /tmp/s6-overlay-amd64-installer

FROM alpine
RUN apk update && apk add bash

# Copy executable & Litestream from builder.
COPY --from=builder /tmp/linkify/bin/linkify /usr/local/bin/linkify
COPY --from=builder /tmp/litestream /usr/local/bin/litestream

# Install s6 for process supervision.
COPY --from=builder /tmp/s6-overlay-amd64-installer /tmp/s6-overlay-amd64-installer
RUN /tmp/s6-overlay-amd64-installer /

# Copy s6 init & service definitions.
COPY etc/cont-init.d /etc/cont-init.d
COPY etc/services.d /etc/services.d

# Copy Litestream configuration file.
COPY etc/litestream.yml /etc/litestream.yml

# The kill grace time is set to zero because our app handles shutdown through SIGTERM.
ENV S6_KILL_GRACETIME=0

# Sync disks is enabled so that data is properly flushed.
ENV S6_SYNC_DISKS=1
EXPOSE 8001

# Run the s6 init process on entry.
ENTRYPOINT [ "/init" ]
