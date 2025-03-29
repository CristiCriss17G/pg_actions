FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

# Include lld linker to improve build times either by using environment variable
# RUSTFLAGS="-C link-arg=-fuse-ld=lld" or with Cargo's configuration file (i.e see .cargo/config.toml).
RUN apt update && export DEBIAN_FRONTEND=noninteractive \
   && apt upgrade -y \
   && apt -y install clang lld nala build-essential vim wget nodejs \
   && apt autoremove -y && apt clean -y

# Install postgrsql apt repo
RUN wget -qO - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo gpg --dearmor -o /usr/share/keyrings/postgresql-archive-keyring.gpg \
   && echo "deb [signed-by=/usr/share/keyrings/postgresql-archive-keyring.gpg] https://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" | sudo tee /etc/apt/sources.list.d/pgdg.list \
   && apt update \
   && apt -y install postgresql-client \
   && apt autoremove -y && apt clean -y