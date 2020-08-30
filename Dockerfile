FROM rustlang/rust:nightly-slim as linkify-builder
WORKDIR /usr/src/linkify
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
ENV LINKIFY_DB_PATH=/linkify_db/linkify.db LINKIFY_USER=demo LINKIFY_PASSWORD=demo LOG_LEVEL=debug
COPY --from=linkify-builder /usr/local/cargo/bin/linkify /usr/local/bin/linkify
EXPOSE 8001
VOLUME /linkify_db
ENTRYPOINT ["linkify"]
CMD ["server"]
