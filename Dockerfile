# We use the latest Rust stable release as base image
FROM rust:1.68.2

# The `app` folder will be created for us by Docker in case it does not exist already.
WORKDIR /app

# Install the required system dependencies for our linking configuration
RUN apt update && apt install lld clang -y

# Copy all files from our working environment to our Docker image
COPY . .

# Use the sqlx offline doc to compile the code
ENV SQLX_OFFLINE true

# Delete the cargo registry to avoid corruptions
RUN rm -rf ~/.cargo/registry

# building the binary
RUN cargo build --release

# When `docker run` is executed, launch the binary!
ENTRYPOINT ["./target/release/zero2prod"]