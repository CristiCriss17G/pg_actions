#!/bin/bash

echo "Running post-create.sh"

# Check if .env file exists and if not create it from .env.example
if [ ! -f .env ]; then
    echo "Creating .env file from .env.example"
    cp .env.example .env
fi

# BRing all the variables from .env file into the shell
while IFS= read -r line; do
    # Skip empty lines and lines starting with #
    if [[ -z "$line" || "$line" =~ ^# ]]; then
        continue
    fi

    # Split the line into key and value
    IFS='=' read -r key value <<<"$line"

    # Evaluate the value to expand variables
    eval value="$value"

    # Export the variable
    export "$key=$value"
    echo "$key=$value" >>.env.new
done <.env

# add to bashrc to have in every shell
echo "export \$(grep -v '^#' $(pwd)/.env | xargs)" >> ~/.bashrc

# Replace the old .env file with the new one
mv .env.new .env

# Create .pgpass file
echo "Creating .pgpass file"
echo "$PG_HOSTNAME:$POSTGRES_PORT:*:$POSTGRES_USER:$POSTGRES_PASSWORD" >~/.pgpass
# Set permissions
echo "Setting permissions"
chmod 0600 ~/.pgpass

# make certs/CAs/rootCA.crt trusted
echo "Making certs/CAs/rootCA.crt trusted"
sudo cp certs/CAs/rootCA.crt /usr/local/share/ca-certificates/
sudo update-ca-certificates

# Cargo
echo "Running cargo fetch"
cargo fetch --locked

# MiniO
echo "Running minio setup"
mc alias set locpg https://miniolocal:9000 minio minio123
mc admin info locpg
# create bucket
mc mb locpg/$S3_BUCKET
echo "See minio details with \`mc admin info locpg\`"