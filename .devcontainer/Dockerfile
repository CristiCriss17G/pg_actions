FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

# Include lld linker to improve build times either by using environment variable
# RUSTFLAGS="-C link-arg=-fuse-ld=lld" or with Cargo's configuration file (i.e see .cargo/config.toml).
RUN apt update && export DEBIAN_FRONTEND=noninteractive \
   && apt upgrade -y \
   && apt -y install clang lld nala build-essential vim wget unzip nodejs \
   && apt autoremove -y && apt clean -y

# Install AWS CLI v2 from the official AWS installer instead of Debian's deprecated package.
RUN arch="$(uname -m)" \
   && case "$arch" in \
   x86_64) aws_arch="x86_64" ;; \
   aarch64|arm64) aws_arch="aarch64" ;; \
   *) echo "Unsupported architecture for AWS CLI: $arch" >&2 && exit 1 ;; \
   esac \
   && wget -q -O /tmp/awscliv2.zip "https://awscli.amazonaws.com/awscli-exe-linux-${aws_arch}.zip" \
   && unzip -q /tmp/awscliv2.zip -d /tmp \
   && /tmp/aws/install --bin-dir /usr/local/bin --install-dir /usr/local/aws-cli \
   && rm -rf /tmp/aws /tmp/awscliv2.zip

# Install postgrsql apt repo
RUN wget -qO - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo gpg --dearmor -o /usr/share/keyrings/postgresql-archive-keyring.gpg \
   && echo "deb [signed-by=/usr/share/keyrings/postgresql-archive-keyring.gpg] https://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" | sudo tee /etc/apt/sources.list.d/pgdg.list \
   && apt update \
   && apt -y install postgresql-client \
   && apt autoremove -y && apt clean -y
