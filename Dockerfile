# Planner stage ----------------------------------
FROM lukemathwalker/cargo-chef:latest as chef
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

# Builder Stage ----------------------------------
# We use the latest Rust stable release as base image
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

# Use the sqlx offline doc to compile the code
ENV SQLX_OFFLINE true

# building the binary
RUN cargo build --release --bin zero2prod

# Runtime stage ----------------------------------
FROM debian:bullseye-slim AS runtime

# workdirectory has to be set in each stage 
WORKDIR /app

# Install OpenSSL - it is dynamically linked by some of our dependencies
# Install ca-certificates - it is needed to verify TLS certificates
# when establishing HTTPS connections
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder environment to our runtime environment
COPY --from=builder /app/target/release/zero2prod zero2prod

# We need the configuration file at runtime!
COPY configuration configuration

# Set environment to be production
ENV APP_ENVIRONMENT production

# Expose the port
EXPOSE 8000

# When `docker run` is executed, launch the binary!
ENTRYPOINT ["./zero2prod"]