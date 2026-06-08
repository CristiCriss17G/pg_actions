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

# RustFS
echo "Running RustFS setup"
until AWS_ACCESS_KEY_ID="$S3_ACCESS_KEY" AWS_SECRET_ACCESS_KEY="$S3_SECRET_KEY" \
    aws --endpoint-url "$S3_ENDPOINT" --region "$S3_REGION" s3api list-buckets >/dev/null; do
    echo "Waiting for RustFS to accept S3 connections"
    sleep 2
done
# create bucket
if ! AWS_ACCESS_KEY_ID="$S3_ACCESS_KEY" AWS_SECRET_ACCESS_KEY="$S3_SECRET_KEY" \
    aws --endpoint-url "$S3_ENDPOINT" --region "$S3_REGION" s3api head-bucket --bucket "$S3_BUCKET" 2>/dev/null; then
    AWS_ACCESS_KEY_ID="$S3_ACCESS_KEY" AWS_SECRET_ACCESS_KEY="$S3_SECRET_KEY" \
        aws --endpoint-url "$S3_ENDPOINT" --region "$S3_REGION" s3api create-bucket --bucket "$S3_BUCKET"
fi
AWS_ACCESS_KEY_ID="$S3_ACCESS_KEY" AWS_SECRET_ACCESS_KEY="$S3_SECRET_KEY" \
    aws --endpoint-url "$S3_ENDPOINT" --region "$S3_REGION" s3api list-buckets
echo "See RustFS buckets with \`aws --endpoint-url $S3_ENDPOINT s3api list-buckets\`"
