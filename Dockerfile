FROM rust:slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/filament-tracker ./filament-tracker
COPY templates ./templates
COPY static ./static

ENV DATABASE_URL=sqlite:///data/db.sqlite
ENV IMAGE_DIR=/data/images
ENV PORT=8080

EXPOSE 8080
CMD ["./filament-tracker"]
