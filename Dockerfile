FROM arm64v8/rust as linkify-builder
WORKDIR /usr/src/linkify
COPY . .
RUN cargo install --path .

FROM arm64v8/debian:buster-slim
ENV LINKIFY_DB_PATH=/linkify/linkify.db LOG_LEVEL=debug
COPY --from=linkify-builder /usr/local/cargo/bin/linkify /usr/local/bin/linkify
EXPOSE 8001
VOLUME /linkify
ENTRYPOINT ["linkify"]
CMD ["server"]
