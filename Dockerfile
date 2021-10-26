####################################################################################################
## Builder
####################################################################################################
FROM ekidd/rust-musl-builder:latest AS builder

#RUN rustup target add x86_64-unknown-linux-musl
#RUN apt update && apt install -y musl-tools musl-dev
#RUN update-ca-certificates

# Create appuser
#ENV USER=appuser
#ENV UID=10001

#RUN adduser \
#    --disabled-password \
#    --gecos "" \
#    --home "/nonexistent" \
#    --shell "/sbin/nologin" \
#    --no-create-home \
#    --uid "${UID}" \
#    "${USER}"


WORKDIR /app

COPY ./ .

RUN cargo build --target x86_64-unknown-linux-musl --release --no-default-features --features headless

####################################################################################################
## Final image
####################################################################################################
FROM scratch

# Import from builder.
# COPY --from=builder /etc/passwd /etc/passwd
# COPY --from=builder /etc/group /etc/group

WORKDIR /app

# Copy our build
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/pong-royale ./

# Use an unprivileged user.
# USER appuser:appuser

CMD ["/app/pong-royale","--server"]
