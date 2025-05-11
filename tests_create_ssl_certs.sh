#!/usr/bin/env bash

# Create a test directory
mkdir -p test_certs

# Generate RSA private key and self-signed cert for example.com
openssl req -x509 -newkey rsa:2048 -nodes -keyout test_certs/example.com.key -out test_certs/example.com.crt -days 365 -subj "/CN=example.com"

# Combine into single PEM file
cat test_certs/example.com.crt test_certs/example.com.key >test_certs/example.com.pem

# Generate another for test.com
openssl req -x509 -newkey rsa:2048 -nodes -keyout test_certs/test.com.key -out test_certs/test.com.crt -days 365 -subj "/CN=test.com"
cat test_certs/test.com.crt test_certs/test.com.key >test_certs/test.com.pem
